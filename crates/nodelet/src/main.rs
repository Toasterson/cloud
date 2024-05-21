use bonsaidb::core::connection::StorageConnection;
use std::path::PathBuf;

use bonsaidb::local::config::Builder;
use bonsaidb::local::{config::StorageConfiguration, Database, Storage};
use clap::Parser;
use cloud::{DeploymentEvent, DeploymentReport, DeploymentState, ResourceIdentifier};
use config::{Environment, File};
use deadpool_lapin::lapin::message::Delivery;
use deadpool_lapin::lapin::options::{
    BasicAckOptions, BasicConsumeOptions, BasicNackOptions, BasicPublishOptions, QueueBindOptions,
    QueueDeclareOptions,
};
use deadpool_lapin::lapin::types::FieldTable;
use deadpool_lapin::lapin::{BasicProperties, Channel};
use deadpool_lapin::Runtime::Tokio1;
use futures::StreamExt;
use miette::Diagnostic;
use nodelet::{DeploymentStatus, Network, NetworkInterface, NodeEntry, NodeObject, Zone};
use serde::Deserialize;
use thiserror::Error;
use tracing::{debug, error, info, instrument};

#[tokio::main]
async fn main() -> miette::Result<()> {
    println!("Hello, world!");
    Ok(())
}

type Result<T, E = Error> = miette::Result<T, E>;

#[derive(Debug, Error, Diagnostic)]
enum Error {
    #[error(transparent)]
    IO(#[from] std::io::Error),

    #[error(transparent)]
    Config(#[from] config::ConfigError),

    #[error(transparent)]
    BonsaidbCore(#[from] bonsaidb::core::Error),

    #[error(transparent)]
    BonsaidbLocal(#[from] bonsaidb::local::Error),

    #[error(transparent)]
    PoolError(#[from] deadpool_lapin::PoolError),

    #[error(transparent)]
    CreatePoolError(#[from] deadpool_lapin::CreatePoolError),

    #[error(transparent)]
    LapinError(#[from] deadpool_lapin::lapin::Error),

    #[error(transparent)]
    Json(#[from] serde_json::Error),

    #[error("Unsupported routing key received")]
    UnsupportedRoutingKey,
}

#[derive(Debug, Parser)]
struct Args {
    #[arg(short, long, default_value = "bonsaidb://localhost")]
    pub connection_string: String,
}

#[derive(Debug, Deserialize)]
struct Config {
    connection_string: String,
    path: PathBuf,
    amqp: deadpool_lapin::Config,
}

fn load_config(args: Args) -> Result<Config> {
    debug!("Loading configuration");
    let cfg = config::Config::builder()
        .add_source(File::with_name("nodelet").required(false))
        .add_source(File::with_name("/etc/nodelet").required(false))
        .add_source(
            Environment::with_prefix("NODE")
                .separator("_")
                .prefix_separator("__"),
        )
        .set_default("path", "./target/nodelet.database")?
        .set_default("amqp.url", "amqp://dev:dev@localhost:5672/master")?
        .set_override("connection_string", args.connection_string)?
        .build()?;
    Ok(cfg.try_deserialize()?)
}

async fn listen(config: Config) -> Result<()> {
    debug!("Initializing local Database");
    let storage =
        Storage::open(StorageConfiguration::new(config.path).with_schema::<NodeEntry>()?)?;
    let nodedb = storage.create_database::<NodeEntry>("node-entries", true)?;
    debug!("Database setup");
    let pool = config.amqp.create_pool(Some(Tokio1))?;

    let conn = pool.get().await?;
    debug!(
        "Connected to {} as {}",
        conn.status().vhost(),
        conn.status().username()
    );

    let hostname = gethostname::gethostname().to_string_lossy().to_string();
    let queue_name = format!("nodelet.{hostname}");

    let channel = conn.create_channel().await?;

    debug!("Defining queue to listen to the exchanges");
    channel
        .queue_declare(
            &queue_name,
            QueueDeclareOptions {
                durable: true,
                ..Default::default()
            },
            FieldTable::default(),
        )
        .await?;

    channel
        .exchange_declare(
            "deployment.nodelet",
            deadpool_lapin::lapin::ExchangeKind::Topic,
            deadpool_lapin::lapin::options::ExchangeDeclareOptions {
                durable: true,
                ..Default::default()
            },
            FieldTable::default(),
        )
        .await?;

    for topic in ["zones", "networks"] {
        let debug!(
            "Defining exchange: {} from channel id {}",
            &state.inbox,
            channel.id()
        );
        channel
            .queue_bind(
                &queue_name,
                "deployment.nodelet",
                topic,
                QueueBindOptions::default(),
                FieldTable::default(),
            )
            .await?;
    }

    info!("connecting amqp consumer...");
    let mut consumer = channel
        .basic_consume(
            &queue_name,
            "nodelet.consumer",
            BasicConsumeOptions::default(),
            FieldTable::default(),
        )
        .await?;

    info!("amqp consumer connected, waiting for messages");
    while let Some(delivery) = consumer.next().await {
        match delivery {
            Ok(delivery) => {
                let tag = delivery.delivery_tag;
                match handle_message(delivery, &channel, &nodedb).await {
                    Ok(report) => {
                        debug!("handled message");
                        channel.basic_ack(tag, BasicAckOptions::default()).await?;
                        let payload = serde_json::to_vec(&report)?;
                        channel
                            .basic_publish(
                                "deployment_reports",
                                "",
                                BasicPublishOptions::default(),
                                &payload,
                                BasicProperties::default(),
                            )
                            .await?;
                    }
                    Err(e) => {
                        error!(error = ?e, "failed to handle message");
                        channel.basic_nack(tag, BasicNackOptions::default()).await?;
                    }
                }
            }
            Err(err) => return Err(Error::LapinError(err)),
        }
    }

    Ok(())
}

#[instrument(skip_all)]
async fn handle_message(
    delivery: Delivery,
    channel: &Channel,
    nodedb: &Database,
) -> Result<DeploymentReport> {
    match delivery.routing_key.as_str() {
        "zones" => {
            let zone_deployment: DeploymentEvent<Zone> = serde_json::from_slice(&delivery.data)?;
            let report = match zone_deployment {
                DeploymentEvent::Ensure { data, identifier } => {
                    let entry = NodeEntry {
                        resource_identifier: identifier.clone(),
                        object: NodeObject::Zone(data.clone()),
                        state: DeploymentStatus::Configured,
                    };
                    DeploymentReport::Ensure {
                        identifier,
                        state: DeploymentState::Configured,
                        result: Some(Ok(())),
                    }
                }
                DeploymentEvent::Remove { data, identifier } => DeploymentReport::Remove {
                    identifier,
                    state: DeploymentState::Configured,
                    result: Some(Ok(())),
                },
                DeploymentEvent::List { .. } => DeploymentReport::List { resources: vec![] },
            };
            Ok(report)
        }
        "networks" => {
            let network_deployment: DeploymentEvent<Network> =
                serde_json::from_slice(&delivery.data)?;
            let report = match network_deployment {
                DeploymentEvent::List { .. } => DeploymentReport::List { resources: vec![] },
                DeploymentEvent::Ensure { data, identifier } => DeploymentReport::Ensure {
                    identifier,
                    state: DeploymentState::Configured,
                    result: Some(Ok(())),
                },
                DeploymentEvent::Remove { data, identifier } => DeploymentReport::Remove {
                    identifier,
                    state: DeploymentState::Configured,
                    result: Some(Ok(())),
                },
            };
            Ok(report)
        }
        _ => Err(Error::UnsupportedRoutingKey),
    }
}

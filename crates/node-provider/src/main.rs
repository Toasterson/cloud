use bonsaidb::core::connection::StorageConnection;
use std::path::PathBuf;

use bonsaidb::local::config::Builder;
use bonsaidb::local::{config::StorageConfiguration, Database, Storage};
use clap::Parser;
use config::{Environment, File};
use deadpool_lapin::lapin::message::Delivery;
use deadpool_lapin::lapin::options::{
    BasicAckOptions, BasicConsumeOptions, BasicNackOptions, QueueBindOptions, QueueDeclareOptions,
};
use deadpool_lapin::lapin::types::FieldTable;
use deadpool_lapin::lapin::Channel;
use deadpool_lapin::Runtime::Tokio1;
use futures::StreamExt;
use miette::Diagnostic;
use node_provider::Zone;
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
        .add_source(File::with_name("node-provider").required(false))
        .add_source(File::with_name("/etc/node-provider").required(false))
        .add_source(
            Environment::with_prefix("NODE")
                .separator("_")
                .prefix_separator("__"),
        )
        .set_default("path", "./target/node.database")?
        .set_default("amqp.url", "amqp://dev:dev@localhost:5672/master")?
        .set_override("connection_string", args.connection_string)?
        .build()?;
    Ok(cfg.try_deserialize()?)
}

async fn listen(config: Config) -> Result<()> {
    debug!("Initializing local Database");
    let storage = Storage::open(StorageConfiguration::new(config.path).with_schema::<Zone>()?)?;
    let zones_db = storage.create_database::<Zone>("zones", true)?;
    debug!("Database setup");
    let pool = config.amqp.create_pool(Some(Tokio1))?;

    let conn = pool.get().await?;
    debug!(
        "Connected to {} as {}",
        conn.status().vhost(),
        conn.status().username()
    );

    let hostname = gethostname::gethostname().to_string_lossy().to_string();
    let queue_name = format!("node-provider.{hostname}");

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
            "deployment",
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
                "deployment",
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
            "node-provider.consumer",
            BasicConsumeOptions::default(),
            FieldTable::default(),
        )
        .await?;

    info!("amqp consumer connected, waiting for messages");
    while let Some(delivery) = consumer.next().await {
        match delivery {
            Ok(delivery) => {
                let tag = delivery.delivery_tag;
                match handle_message(delivery, &channel, &zones_db).await {
                    Ok(_) => {
                        debug!("handled message");
                        channel.basic_ack(tag, BasicAckOptions::default()).await?;
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
async fn handle_message(delivery: Delivery, channel: &Channel, zone_db: &Database) -> Result<()> {
    Ok(())
}

use std::path::PathBuf;

use bonsaidb::core::connection::AsyncStorageConnection;
use bonsaidb::local::config::Builder;
use bonsaidb::server::{DefaultPermissions, Server, ServerConfiguration};
use clap::Parser;
use config::File;
use miette::Diagnostic;
use serde::Deserialize;
use thiserror::Error;
use tracing::debug;

use cloud::Deployment;

#[derive(Error, Debug, Diagnostic)]
pub enum Error {
    #[error(transparent)]
    IOError(#[from] std::io::Error),

    #[error(transparent)]
    BonsaiDBServer(#[from] bonsaidb::server::Error),

    #[error(transparent)]
    BonsaiDBCore(#[from] bonsaidb::core::Error),

    #[error(transparent)]
    BonsaiDBLocal(#[from] bonsaidb::local::Error),

    #[error(transparent)]
    BonsaiDBBackend(#[from] bonsaidb::server::BackendError),

    #[error(transparent)]
    Config(#[from] config::ConfigError),
}

pub type Result<T, E = Error> = miette::Result<T, E>;

#[derive(Parser)]
pub struct Args {
    path: Option<PathBuf>,
}

pub fn load_config(args: Args) -> Result<Config> {
    debug!("Loading configuration");
    let cfg = config::Config::builder()
        .add_source(File::with_name("fleetadmd").required(false))
        .add_source(File::with_name("/etc/fleetadmd").required(false))
        .set_default("path", "./target/fleet.database")?
        .set_override_option("path", args.path.map(|p| p.to_string_lossy().to_string()))?
        .build()?;
    Ok(cfg.try_deserialize()?)
}

#[derive(Deserialize)]
pub struct Config {
    path: PathBuf,
}

pub async fn listen(cfg: Config) -> Result<()> {
    debug!("Initializing Database");
    let server = Server::open(
        ServerConfiguration::new(cfg.path)
            .default_permissions(DefaultPermissions::AllowAll)
            .with_schema::<Deployment>()?,
    )
    .await?;

    if server.certificate_chain().await.is_err() {
        server.install_self_signed_certificate(true).await?;
    }

    server
        .create_database::<Deployment>("deployments", true)
        .await?;

    debug!("Database setup");

    debug!("Staring Server");
    server.listen_on(5645).await?;

    Ok(())
}

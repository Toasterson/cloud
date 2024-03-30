use std::path::PathBuf;

use bonsaidb::local::{config::StorageConfiguration, Database, Storage};
use clap::Parser;
use config::File;
use miette::Diagnostic;
use node_provider::Zone;
use serde::Deserialize;
use thiserror::Error;
use tracing::debug;

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
}

fn load_config(args: Args) -> Result<Config> {
    debug!("Loading configuration");
    let cfg = config::Config::builder()
        .add_source(File::with_name("node-provider").required(false))
        .add_source(File::with_name("/etc/node-provider").required(false))
        .set_default("path", "./target/node.database")?
        .set_override("connection_string", args.connection_string)?
        .build()?;
    Ok(cfg.try_deserialize()?)
}

fn listen(config: Config) -> Result<()> {
    debug!("Initializing local Database");
    let storage = Storage::open(StorageConfiguration::new(config.path).with_schema::<Zone>()?)?;
    let zones_db = storage.create_database::<Zone>("zones", true)?;

    debug!("Database setup");

    debug!("Staring Server");

    Ok(())
}

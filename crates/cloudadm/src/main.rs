use bonsaidb::client::fabruic::{error, Certificate};
use bonsaidb::client::url::ParseError;
use bonsaidb::client::AsyncClient;
use bonsaidb::core::connection::AsyncStorageConnection;
use bonsaidb::core::schema::{Collection, Qualified, SchemaName};
use miette::Diagnostic;
use std::fs::File;
use std::io::Read;
use std::path::PathBuf;
use thiserror::Error;

use clap::{Parser, Subcommand};
use serde::{Deserialize, Serialize};

#[derive(Parser, Debug)]
pub(crate) struct Args {
    #[arg(
        long,
        default_value = "../../target/fleet.database/pinned-certificate.der"
    )]
    pub certificate: PathBuf,
    #[arg(short, long, default_value = "bonsaidb://localhost")]
    pub connection_string: String,
    #[command(subcommand)]
    pub command: Commands,
}

#[derive(Subcommand, Debug)]
pub(crate) enum Commands {
    /// List all databases of the server
    List,
    /// Create a Database
    Create { name: String },
}

#[derive(Error, Debug, Diagnostic)]
enum Error {
    #[error(transparent)]
    BonsaidbClient(#[from] bonsaidb::client::Error),
    #[error(transparent)]
    BonsaidbCore(#[from] bonsaidb::core::Error),
    #[error(transparent)]
    Parse(#[from] ParseError),
    #[error(transparent)]
    Certificate(#[from] error::Certificate),
    #[error(transparent)]
    IOError(#[from] std::io::Error),
}

type Result<T, E = Error> = miette::Result<T, E>;

#[derive(Serialize, Deserialize, Collection)]
#[collection(name = "Test")]
struct TestCollection {
    pub name: String,
}
#[tokio::main]
async fn main() -> Result<()> {
    let args = Args::parse();

    let mut f = File::open(args.certificate)?;
    let mut cert_buf = vec![];
    f.read_to_end(&mut cert_buf)?;
    drop(f);

    let client = AsyncClient::build(args.connection_string.parse()?)
        .with_certificate(Certificate::from_der(cert_buf)?)
        .build()?;

    match args.command {
        Commands::List => {
            let dbs = client.list_databases().await?;
            println!("Listing databases");
            for db in dbs {
                println!("{}: {}", db.name, db.schema.to_string());
            }
        }
        Commands::Create { name } => {
            client
                .create_database_with_schema(&name, SchemaName::new("private", "deployments"), true)
                .await?;
        }
    }

    Ok(())
}

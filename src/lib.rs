use arc_bytes::serde::Bytes;
use bonsaidb::core::schema::view::map::MappedSerializedDocuments;
use bonsaidb::core::schema::{Collection, MappedValue};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Collection)]
#[collection(name = "deployments")]
pub struct Deployment {
    pub name: String,
    pub parameters: Vec<Bytes>,
    pub resources: Vec<MappedSerializedDocuments>,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct Parameter {
    pub key: String,
    pub value: String,
}

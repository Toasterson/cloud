use bonsaidb::core::schema::Collection;
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Collection)]
#[collection(name = "zones", primary_key = String)]
pub struct Zone {
    #[natural_id]
    pub name: String,
    pub brand: ZoneBrand,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum ZoneBrand {
    LinkedPkg,
    UnlinkedPkg,
    Other(String),
}

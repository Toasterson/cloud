use std::collections::HashMap;

use bonsaidb::core::schema::Collection;
use knuffel::{Decode, DecodeScalar};
use serde::{Deserialize, Serialize};

#[derive(Debug, Serialize, Deserialize, Collection, Decode)]
#[collection(name = "zones", primary_key = String)]
pub struct Zone {
    #[knuffel(child, unwrap(argument))]
    pub brand: ZoneBrand,
    #[knuffel(child, unwrap(argument))]
    pub autoboot: bool,
    #[knuffel(child, unwrap(argument))]
    pub ip_type: ZoneIpType,
    #[knuffel(children(name = "net"))]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub network: Vec<NetworkInterface>,
    #[knuffel(children(name = "nameserver"), unwrap(argument))]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub nameservers: Vec<String>,
    #[knuffel(children(name = "dns-search"), unwrap(argument))]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub dns_search: Vec<String>,
    #[knuffel(children(name = "package"), unwrap(argument))]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub packages: Vec<String>,
    #[knuffel(children(name = "service"))]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub services: Vec<SMFService>,
}

#[derive(Debug, Serialize, Deserialize, Decode)]
pub struct SMFService {
    #[knuffel(argument)]
    pub name: String,
    #[knuffel(children)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub property_groups: Vec<SMFServicePropertyGroup>,
}

#[derive(Debug, Serialize, Deserialize, Decode)]
pub struct SMFServicePropertyGroup {
    #[knuffel(properties)]
    pub properties: HashMap<String, String>,
}

#[derive(Debug, Serialize, Deserialize, Decode)]
pub struct NetworkInterface {
    #[knuffel(argument)]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub name: Option<String>,
    #[knuffel(child, unwrap(argument))]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub physical: Option<String>,
    #[knuffel(child, unwrap(argument))]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub allowed_address: Option<String>,
    #[knuffel(child, unwrap(argument))]
    #[serde(skip_serializing_if = "Option::is_none")]
    pub defrouter: Option<String>,
}

#[derive(Debug, Serialize, Deserialize, DecodeScalar)]
pub enum ZoneIpType {
    Exclusive,
    Shared,
}

#[derive(Debug, Serialize, Deserialize, DecodeScalar)]
pub enum ZoneBrand {
    LinkedPkg,
    UnlinkedPkg,
}

use std::collections::HashMap;
use std::fmt::{Display, Formatter};
use std::str::FromStr;

use bonsaidb::core::schema::Collection;
use cloud::ResourceIdentifier;
use knuffel::{Decode, DecodeScalar};
use serde::{Deserialize, Serialize};
use thiserror::Error;

#[derive(Debug, Error)]
pub enum NodeletDataError {
    #[error("invalid vswitch type {0}, Must be one of `local`, `distributed`, `external`")]
    InvalidVSwitchKind(String),
}

#[derive(Debug, Serialize, Deserialize)]
pub enum DeploymentStatus {
    Configured,
    Installed,
    Starting,
    Started,
    Stopping,
    Stopped,
    Uninstalling,
    Archived,
}

#[derive(Debug, Serialize, Deserialize, Collection)]
#[collection(name = "node-entries", primary_key = ResourceIdentifier)]
pub struct NodeEntry {
    #[natural_id]
    pub resource_identifier: ResourceIdentifier,
    pub object: NodeObject,
    pub state: DeploymentStatus,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum NodeObject {
    Zone(Zone),
    Network(Network),
}

impl Display for NodeObject {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            NodeObject::Zone(_) => write!(f, "Zone"),
            NodeObject::Network(_) => write!(f, "Network"),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Decode, Clone)]
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

#[derive(Debug, Serialize, Deserialize, Decode, Clone)]
pub struct SMFService {
    #[knuffel(argument)]
    pub name: String,
    #[knuffel(children)]
    #[serde(skip_serializing_if = "Vec::is_empty")]
    pub property_groups: Vec<SMFServicePropertyGroup>,
}

#[derive(Debug, Serialize, Deserialize, Decode, Clone)]
pub struct SMFServicePropertyGroup {
    #[knuffel(properties)]
    pub properties: HashMap<String, String>,
}

#[derive(Debug, Serialize, Deserialize, Decode, Clone)]
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

#[derive(Debug, Serialize, Deserialize, DecodeScalar, Clone)]
pub enum ZoneIpType {
    Exclusive,
    Shared,
}

#[derive(Debug, Serialize, Deserialize, DecodeScalar, Clone)]
pub enum ZoneBrand {
    LinkedPkg,
    UnlinkedPkg,
}

#[derive(Debug, Serialize, Deserialize, Decode, Clone)]
pub struct Network {
    #[knuffel(child, unwrap(argument))]
    pub tenant: Option<String>,
    #[knuffel(child, unwrap(argument))]
    pub name: Option<String>,
    #[knuffel(children)]
    pub switches: Vec<VSwitch>,
}

#[derive(Debug, Serialize, Deserialize, Decode, Clone)]
pub struct VSwitch {
    #[knuffel(type_name)]
    pub kind: VSwitchKind,
    #[knuffel(argument)]
    pub name: String,
    #[knuffel(child, unwrap(argument))]
    pub allowed_range: Option<String>,
    #[knuffel(child, unwrap(argument))]
    pub router: Option<String>,
    #[knuffel(children(name = "public-ips"))]
    pub public_ips: Vec<PublicIp>,
}

#[derive(Debug, Serialize, Deserialize, DecodeScalar, Clone)]
pub enum VSwitchKind {
    Distributed,
    External,
    Local,
}

impl FromStr for VSwitchKind {
    type Err = NodeletDataError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        match s {
            "local" => Ok(Self::Local),
            "distributed" => Ok(Self::Distributed),
            "external" => Ok(Self::External),
            s => Err(Self::Err::InvalidVSwitchKind(s.to_owned())),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Decode, Clone)]
pub struct PublicIp {
    #[knuffel(child, unwrap(argument))]
    pub range: String,
    #[knuffel(child, unwrap(argument))]
    pub reserved: String,
    #[knuffel(child, unwrap(argument))]
    pub floating_ip: String,
}

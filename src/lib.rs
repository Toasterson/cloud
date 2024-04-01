use arc_bytes::serde::Bytes;
use bonsaidb::core::key::Key;
use bonsaidb::core::schema::Collection;
use chrono::{DateTime, NaiveDateTime};
use miette::Diagnostic;
use semver::Version;
use serde::de::DeserializeOwned;
use serde::{de::Visitor, Deserialize, Serialize};
use std::str::FromStr;
use thiserror::Error;

#[derive(Error, Diagnostic, Debug)]
pub enum ResourceIdentifierParseError {
    #[error(transparent)]
    Semver(#[from] semver::Error),
    #[error(transparent)]
    ParseInt(#[from] std::num::ParseIntError),
    #[error("no scheme provided please start the string with res:// res:/")]
    NoScheme,
    #[error("version without timestamp")]
    NoTimestamp,
    #[error("no version in resource identifier")]
    NoVersion,
}

#[derive(Key, Debug, Clone, Eq, PartialEq, Ord, PartialOrd)]
pub struct ResourceIdentifier {
    pub tenant: Option<String>,
    pub name: String,
    pub version: String,
    pub revision: i32,
    pub timestamp: i64,
}

impl Serialize for ResourceIdentifier {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(self.to_string().as_str())
    }
}

struct ResourceIdentifierVisitor;

impl<'de> Visitor<'de> for ResourceIdentifierVisitor {
    type Value = ResourceIdentifier;

    fn expecting(&self, formatter: &mut std::fmt::Formatter) -> std::fmt::Result {
        formatter.write_str("a resource identifier starting with res:/ or res://")
    }

    fn visit_str<E>(self, v: &str) -> Result<Self::Value, E>
    where
        E: serde::de::Error,
    {
        ResourceIdentifier::from_str(v).map_err(serde::de::Error::custom)
    }
}

impl<'de> Deserialize<'de> for ResourceIdentifier {
    fn deserialize<D>(deserializer: D) -> Result<Self, D::Error>
    where
        D: serde::Deserializer<'de>,
    {
        deserializer.deserialize_str(ResourceIdentifierVisitor)
    }
}

impl ResourceIdentifier {
    pub fn new(tenant: Option<String>, name: String, version: Version) -> Self {
        use chrono::prelude::*;
        Self {
            tenant,
            name,
            version: version.to_string(),
            revision: 0,
            timestamp: Utc::now().naive_utc().and_utc().timestamp(),
        }
    }
}

impl FromStr for ResourceIdentifier {
    type Err = ResourceIdentifierParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let (name_string, version_string) = s
            .split_once('@')
            .ok_or(ResourceIdentifierParseError::NoVersion)?;
        let (tenant, name) = if name_string.starts_with("res://") {
            let clean_name = name_string.replace("res://", "");
            let split = clean_name
                .split_once('/')
                .ok_or(ResourceIdentifierParseError::NoScheme)?;
            (Some(split.0.to_string()), split.1.to_string())
        } else {
            (None, name_string.replace("res:/", ""))
        };

        let (version, revision, timestamp) = if version_string.contains('-') {
            let split = version_string
                .split_once(':')
                .ok_or(ResourceIdentifierParseError::NoTimestamp)?;
            (
                Version::parse(split.0)?,
                0,
                DateTime::from_timestamp_nanos(split.1.parse()?).naive_utc(),
            )
        } else {
            let split = version_string
                .split_once(':')
                .ok_or(ResourceIdentifierParseError::NoTimestamp)?;
            let ver_split = split.0.split_once('-').unwrap();
            (
                Version::parse(ver_split.0)?,
                ver_split.1.parse()?,
                DateTime::from_timestamp_nanos(split.1.parse()?).naive_utc(),
            )
        };

        Ok(Self {
            tenant,
            name,
            version: version.to_string(),
            revision,
            timestamp: timestamp.and_utc().timestamp(),
        })
    }
}

impl ToString for ResourceIdentifier {
    fn to_string(&self) -> String {
        let version_string = if self.revision == 0 {
            format!("{}:{}", self.version, self.timestamp)
        } else {
            format!("{}-{}:{}", self.version, self.revision, self.timestamp)
        };
        match &self.tenant {
            None => format!("res:/{}@{}", self.name, version_string),
            Some(tenant) => format!("res://{tenant}/{}@{}", self.name, version_string),
        }
    }
}

#[derive(Debug, Serialize, Deserialize)]
pub enum Selector {
    Node { name: String },
    LabelMatcher { labels: Vec<String> },
}

#[derive(Debug, Serialize, Deserialize, Collection)]
#[collection(name = "deployments", primary_key = ResourceIdentifier)]
pub struct Deployment {
    #[natural_id]
    pub resource_identifier: ResourceIdentifier,
    pub created_at: NaiveDateTime,
    pub updated_at: Option<NaiveDateTime>,
    pub resources: Vec<Bytes>,
    pub files: Vec<File>,
    pub selector: Selector,
}

#[derive(Debug, Serialize, Deserialize)]
pub struct File {
    pub name: String,
    pub kind: FileKind,
    pub body: Bytes,
}

#[derive(Debug, Deserialize, Serialize)]
pub enum FileKind {
    Template,
    Normal,
}

#[derive(Debug, Deserialize, Serialize)]
pub enum DeploymentEvent<T> {
    Create { data: T, kind: String },
    Update { data: T, kind: String },
    Delete { data: T, kind: String },
    List { requester: String, kind: String },
}

#[derive(Debug, Deserialize, Serialize)]
pub enum StatusReport<T, E> {
    Ok { kind: String, data: Vec<T> },
    Err(E),
}

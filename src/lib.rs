use arc_bytes::serde::Bytes;
use bonsaidb::core::key::Key;
use bonsaidb::core::schema::Collection;
use chrono::{DateTime, NaiveDateTime, Utc};
use miette::Diagnostic;
use semver::Version;
use serde::{Deserialize, Serialize};
use std::str::FromStr;
use thiserror::Error;

#[derive(Error, Diagnostic)]
pub enum ResourceIdentifierParseError {}

#[derive(Key, Debug, Clone, Copy, Eq, PartialEq, Ord, PartialOrd)]
struct ResourceIdentifier {
    pub tenant: Option<String>,
    pub name: String,
    pub version: Version,
    pub revision: i32,
    pub timestamp: NaiveDateTime,
}

impl ResourceIdentifier {
    pub fn new(tenant: Option<String>, name: String, version: Version) -> Self {
        use chrono::prelude::*;
        Self {
            tenant,
            name,
            version,
            revision: 0,
            timestamp: Utc::now().naive_utc(),
        }
    }
}

impl FromStr for ResourceIdentifier {
    type Err = ResourceIdentifierParseError;

    fn from_str(s: &str) -> Result<Self, Self::Err> {
        let (name_string, version_string) = s.split_once('@')?;
        let (tenant, name) = if name_string.starts_with("res://") {
            let split = name_string.replace("res://", "").split_once('/')?;
            (Some(split.0.to_string()), split.1.to_string())
        } else {
            (None, name_string.replace("res:/", ""))
        };

        let (version, revision, timestamp) = if version_string.contains('-') {
            let split = version_string.split_once(':')?;
            (
                Version::parse(split.0)?,
                0,
                DateTime::from_timestamp_nanos(split.1.parse()?).naive_utc(),
            )
        } else {
            let split = version_string.split_once(':')?;
            let ver_split = split.0.split_once('-')?;
            (
                Version::parse(ver_split.0)?,
                ver_split.1.parse()?,
                DateTime::from_timestamp_nanos(split.1.parse()?).naive_utc(),
            )
        };

        Ok(Self {
            tenant,
            name,
            version,
            revision,
            timestamp,
        })
    }
}

impl Into<String> for ResourceIdentifier {
    fn into(self) -> String {
        let version_string = if self.revision == 0 {
            format!("{}:{}", self.version, self.timestamp.and_utc().timestamp())
        } else {
            format!(
                "{}-{}:{}",
                self.version,
                self.revision,
                self.timestamp.and_utc().timestamp()
            )
        };
        match self.tenant {
            None => format!("res:/{}@{}", self.name, version_string),
            Some(tenant) => format!("res://{tenant}/{}@{}", self.name, version_string),
        }
    }
}

#[derive(Debug, Serialize, Deserialize, Collection)]
#[collection(name = "deployments", primary_key = ResourceIdentifier)]
pub struct Deployment {
    pub created_at: NaiveDateTime,
    pub updated_at: Option<NaiveDateTime>,
    pub resources: Vec<Bytes>,
    pub files: Vec<File>,
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

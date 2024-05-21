use std::fmt::Display;
use arc_bytes::serde::Bytes;
use bonsaidb::core::key::Key;
use bonsaidb::core::schema::Collection;
use chrono::{DateTime, NaiveDateTime};
use miette::Diagnostic;
use semver::Version;
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
    #[error("no version in resource identifier")]
    NoVersion,
}

#[derive(Key, Debug, Clone, Eq, PartialEq, Ord, PartialOrd)]
pub struct ResourceIdentifier {
    pub tenant: Option<String>,
    pub name: String,
    pub version: String,
    pub revision: i32,
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
        Self {
            tenant,
            name,
            version: version.to_string(),
            revision: 0,
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

        let (version, revision) = if !version_string.contains('-') {
            (    
                Version::parse(version_string)?,
                0,
            )
        } else {
            let ver_split = version_string.split_once('-').unwrap();
            (
                Version::parse(ver_split.0)?,
                ver_split.1.parse()?,
            )
        };

        Ok(Self {
            tenant,
            name,
            version: version.to_string(),
            revision,
        })
    }
}

impl Display for ResourceIdentifier {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let version_string = if self.revision == 0 {
            format!("{}", self.version)
        } else {
            format!("{}-{}", self.version, self.revision)
        };
        let str = match &self.tenant {
            None => format!("res:/{}@{}", self.name, version_string),
            Some(tenant) => format!("res://{tenant}/{}@{}", self.name, version_string),
        };
        write!(f, "{}", str)
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
    pub state: DeploymentState,
    pub files: Vec<File>,
    pub selector: Selector,
}

#[derive(Debug, Serialize, Deserialize)]
pub enum DeploymentState {
    Configured,
    Installed,
    Starting,
    Started,
    Stopping,
    Stopped,
    Archived,
    Orphaned,
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
    Ensure { data: T, identifier: ResourceIdentifier },
    Remove { data: T, identifier: ResourceIdentifier },
    List { requester: String},
}

/// Emitted by the nodelet multiple times during the setup process.
/// A Statusreport is when result is None, a Final Report is when Some result is returned
#[derive(Debug, Deserialize, Serialize)]
pub enum DeploymentReport {
    Ensure {
        identifier: ResourceIdentifier,
        state: DeploymentState,
        result: Option<Result<(), String>>,
    },
    Remove {
        identifier: ResourceIdentifier,
        state: DeploymentState,
        result: Option<Result<(), String>>,
    },
    List {
        resources: Vec<(ResourceIdentifier, DeploymentState)>
    },
}

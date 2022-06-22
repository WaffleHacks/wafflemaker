use serde::{Deserialize, Serialize};
use std::{
    collections::{HashMap, HashSet},
    time::{SystemTime, UNIX_EPOCH},
};

/// Create a new hash set using a macro
macro_rules! set {
    () => {
        std::collections::HashSet::new()
    };
    ( $( $val:expr ),+ ) => {{
        let mut set = std::collections::HashSet::new();
        $(
            set.insert($val);
        )*
        set
    }};
}

/// The base response for all API requests
#[derive(Debug, Deserialize)]
pub struct BaseResponse<T> {
    pub data: T,
}

/// The base response for all API requests with lease information
#[derive(Debug, Deserialize)]
pub struct BaseResponseWithLease<T> {
    #[serde(flatten)]
    pub lease: Lease,
    pub data: T,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, Hash, PartialEq)]
#[serde(rename_all = "lowercase")]
pub enum Permission {
    Create,
    Read,
    Update,
    Delete,
    List,
    Deny,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Capabilities<'paths> {
    #[serde(skip_serializing)]
    pub data: HashMap<String, HashSet<Permission>>,
    #[serde(skip_deserializing, borrow)]
    paths: Vec<&'paths str>,
}

impl<'paths> Capabilities<'paths> {
    pub fn mapping() -> HashMap<&'static str, HashSet<Permission>> {
        use Permission::*;

        let mut m = HashMap::new();
        m.insert("auth/token/renew-self", set![Update]);
        m.insert("aws/creds/+", set![Read]);
        m.insert("database/creds/+", set![Read]);
        m.insert("database/roles/+", set![List, Create, Delete]);
        m.insert("services/data/*", set![Create, Read, Update]);
        m
    }

    /// Get a list of all the paths to query permissions for
    pub fn paths() -> Capabilities<'paths> {
        Self {
            data: HashMap::new(),
            paths: Self::mapping().keys().copied().collect(),
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Secret {
    pub data: HashMap<String, String>,
}

#[derive(Debug, Deserialize)]
pub struct Aws {
    pub access_key: String,
    pub secret_key: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct DatabaseRole<'s> {
    #[serde(skip_deserializing)]
    db_name: &'s str,
    #[serde(skip_deserializing)]
    creation_statements: Vec<String>,
    #[serde(skip_deserializing)]
    default_ttl: &'s str,

    #[serde(skip_serializing)]
    pub keys: Vec<String>,
}

impl<'s> DatabaseRole<'s> {
    pub fn new(role: &str) -> DatabaseRole<'s> {
        Self {
            db_name: "postgresql",
            default_ttl: "21600", // 6hrs in seconds
            creation_statements: vec![
                r#"CREATE ROLE "{{name}}" WITH LOGIN PASSWORD '{{password}}' VALID UNTIL '{{expiration}}' INHERIT;"#.to_owned(),
                format!(r#"GRANT "{}" TO "{{{{name}}}}";"#, role),
            ],
            keys: Default::default(),
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct RoleCredentials {
    pub password: String,
    pub username: String,
}

#[derive(Clone, Debug, Deserialize)]
pub struct Lease {
    #[serde(rename = "lease_id")]
    pub id: String,
    #[serde(rename = "lease_duration")]
    pub ttl: u64,
    #[serde(default = "now")]
    pub updated_at: u64,
}

#[derive(Debug, Serialize)]
pub struct LeaseRenewal<'l> {
    pub lease_id: &'l str,
    pub increment: u64,
}

#[derive(Debug, Serialize)]
pub struct LeaseRevocation<'l> {
    pub lease_id: &'l str,
}

/// Get the current unix timestamp
fn now() -> u64 {
    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

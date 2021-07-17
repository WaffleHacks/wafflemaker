use serde::{Deserialize, Serialize};
use std::collections::{HashMap, HashSet};

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
        m.insert("database/static-creds/+", set![Read]);
        m.insert("database/static-roles/+", set![List, Create, Delete]);
        m.insert("database/rotate-role/+", set![Update]);
        m.insert("services/data/+", set![Create, Read, Update]);
        m
    }

    /// Get a list of all the paths to query permissions for
    pub fn paths() -> Capabilities<'paths> {
        Self {
            data: HashMap::new(),
            paths: Self::mapping().keys().map(|s| *s).collect(),
        }
    }
}

#[derive(Debug, Deserialize, Serialize)]
pub struct Secret {
    pub data: HashMap<String, String>,
}

#[derive(Debug, Deserialize)]
pub struct AWS {
    pub access_key: String,
    pub secret_key: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub struct StaticRoles<'s> {
    #[serde(skip_deserializing)]
    db_name: &'s str,
    #[serde(skip_deserializing)]
    username: &'s str,
    #[serde(skip_deserializing)]
    rotation_statements: &'s [&'s str],
    #[serde(skip_deserializing)]
    rotation_period: &'s str,

    #[serde(skip_serializing)]
    pub keys: Vec<String>,
}

impl<'s> StaticRoles<'s> {
    pub fn new(username: &'s str) -> StaticRoles<'s> {
        StaticRoles {
            keys: Default::default(),
            db_name: "postgresql",
            rotation_statements: &[r#"ALTER USER "{{name}}" WITH PASSWORD '{{password}}';"#],
            rotation_period: "2628000",
            username,
        }
    }
}

#[derive(Debug, Deserialize)]
pub struct StaticCredentials {
    pub password: String,
    pub username: String,
}

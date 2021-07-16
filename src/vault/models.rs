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
        m.insert("database/static-roles/+", set![Create, Delete, Update]);
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

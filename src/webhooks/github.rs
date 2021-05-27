use serde::Deserialize;

/// The overarching GitHub webhook types
#[derive(Debug, Deserialize)]
#[serde(untagged, rename_all = "lowercase")]
pub enum Github {
    Ping {
        zen: String,
        hook_id: i64,
    },
    Push {
        after: String,
        #[serde(rename = "ref")]
        reference: String,
        repository: Repository,
    },
}

impl Github {
    /// Get the name of the webhook being executed
    pub fn name<'a>(&self) -> &'a str {
        match self {
            Self::Ping { .. } => "ping",
            Self::Push { .. } => "push",
        }
    }
}

/// The repository information
#[derive(Clone, Debug, Deserialize)]
pub struct Repository {
    #[serde(rename = "full_name")]
    pub name: String,
    pub clone_url: String,
}

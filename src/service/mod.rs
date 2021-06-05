use globset::Glob;
use serde::Deserialize;
use serde_with::{serde_as, DisplayFromStr};
use std::{collections::HashMap, path::Path};
use tokio::fs;

mod secret;

pub use secret::{Format, Secret};

/// The configuration for a service
#[derive(Debug, Deserialize)]
pub struct Service {
    dependencies: Dependencies,
    docker: Docker,
    environment: HashMap<String, String>,
    secrets: HashMap<String, Secret>,
}

impl Service {
    /// Parse a service configuration from a given file
    pub async fn parse<P: AsRef<Path>>(path: P) -> anyhow::Result<Service> {
        let raw = fs::read(path).await?;
        Ok(toml::from_slice(&raw)?)
    }
}

/// All the possible external dependencies a service can require.
#[derive(Debug, Deserialize)]
pub struct Dependencies {
    postgres: Option<Dependency>,
    redis: Option<Dependency>,
}

/// The definition of a dependency service. `State` specifies whether it is enabled
/// or disabled, and `Rename` specifies the environment variables name and implicitly
/// enables it.
#[derive(Debug, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum Dependency {
    State(bool),
    Rename(String),
}

/// The docker image configuration
#[serde_as]
#[derive(Debug, Deserialize)]
pub struct Docker {
    image: String,
    #[serde_as(as = "Vec<DisplayFromStr>")]
    tags: Vec<Glob>,
    #[serde(default, rename = "auto-update")]
    auto_update: bool,
}

#[cfg(test)]
mod tests {
    use super::{Dependency, Service};

    #[tokio::test]
    async fn deserialize() {
        let service = Service::parse("./example-service.toml")
            .await
            .expect("failed to parse service");

        println!("{:?}", service);

        assert_eq!(
            service.dependencies.postgres,
            Some(Dependency::Rename("DATABASE_URL".into()))
        );
        assert_eq!(service.dependencies.redis, Some(Dependency::State(false)));
        assert_eq!(service.docker.image, "wafflehacks/cms");
        assert_eq!(service.docker.auto_update, true);
        assert_eq!(service.docker.tags.len(), 2);
        assert_eq!(service.environment.len(), 4);
        assert_eq!(service.secrets.len(), 5);
    }
}

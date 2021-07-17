use globset::Glob;
use serde::Deserialize;
use serde_with::{serde_as, DisplayFromStr, NoneAsEmptyString};
use std::{collections::HashMap, ffi::OsStr, path::Path};
use tokio::fs;

pub mod registry;
mod secret;

pub use secret::{Format, Part as AWSPart, Secret};

/// The configuration for a service
#[derive(Debug, Deserialize)]
pub struct Service {
    #[serde(default)]
    pub dependencies: Dependencies,
    pub docker: Docker,
    #[serde(default)]
    pub environment: HashMap<String, String>,
    #[serde(default)]
    pub secrets: HashMap<String, Secret>,
    #[serde(default)]
    pub web: Web,
}

impl Service {
    /// Parse a service configuration from a given file
    pub async fn parse<P: AsRef<Path>>(path: P) -> anyhow::Result<Service> {
        let raw = fs::read(path).await?;
        Ok(toml::from_slice(&raw)?)
    }

    /// Generate the name of a service from its file path
    pub fn name(path: &Path) -> String {
        path.with_extension("")
            .iter()
            .rev()
            .map(OsStr::to_str)
            .map(Option::unwrap)
            .collect::<Vec<_>>()
            .join("-")
    }
}

/// All the possible external dependencies a service can require.
#[derive(Debug, Default, Deserialize)]
pub struct Dependencies {
    postgres: Option<Dependency>,
    redis: Option<Dependency>,
}

impl Dependencies {
    pub fn postgres(&self) -> Option<&str> {
        self.postgres
            .as_ref()
            .map(|d| d.resolve("POSTGRES_URL"))
            .flatten()
    }

    pub fn redis(&self) -> Option<&str> {
        self.redis
            .as_ref()
            .map(|d| d.resolve("REDIS_URL"))
            .flatten()
    }
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

impl Dependency {
    pub fn resolve<'n>(&'n self, default: &'n str) -> Option<&'n str> {
        match self {
            Self::Rename(name) => Some(&name),
            Self::State(true) => Some(default),
            Self::State(false) => None,
        }
    }
}

/// The docker image configuration
#[derive(Debug, Deserialize)]
pub struct Docker {
    pub image: String,
    pub tag: String,
    #[serde(default)]
    pub update: AutoUpdate,
}

#[serde_as]
#[derive(Debug, Deserialize)]
pub struct AutoUpdate {
    #[serde(default)]
    #[serde_as(as = "Vec<DisplayFromStr>")]
    pub additional_tags: Vec<Glob>,
    #[serde(default = "default_true")]
    pub automatic: bool,
}

impl Default for AutoUpdate {
    fn default() -> AutoUpdate {
        AutoUpdate {
            additional_tags: Vec::new(),
            automatic: true,
        }
    }
}

#[serde_as]
#[derive(Debug, Deserialize)]
pub struct Web {
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default)]
    #[serde_as(as = "NoneAsEmptyString")]
    pub base: Option<String>,
}

impl Default for Web {
    fn default() -> Web {
        Web {
            enabled: true,
            base: None,
        }
    }
}

fn default_true() -> bool {
    true
}

#[cfg(test)]
mod tests {
    use super::Service;

    #[tokio::test]
    async fn deserialize() {
        let service = Service::parse("./example-service.toml")
            .await
            .expect("failed to parse service");

        assert_eq!(service.dependencies.postgres(), Some("DATABASE_URL"));
        assert_eq!(service.dependencies.redis(), None);
        assert_eq!(service.docker.image, "wafflehacks/cms");
        assert_eq!(service.docker.tag, "develop");
        assert_eq!(service.docker.update.automatic, true);
        assert_eq!(service.docker.update.additional_tags.len(), 1);
        assert_eq!(service.environment.len(), 4);
        assert_eq!(service.secrets.len(), 6);
        assert_eq!(service.web.enabled, true);
        assert_eq!(service.web.base, Some("wafflehacks.tech".into()));
    }

    #[tokio::test]
    async fn defaults() {
        let service = Service::parse("./testdata/service/minimal.toml")
            .await
            .expect("failed to parse service");

        assert_eq!(service.dependencies.postgres(), None);
        assert_eq!(service.dependencies.redis(), None);
        assert_eq!(service.docker.image, "wafflehacks/cms");
        assert_eq!(service.docker.tag, "develop");
        assert_eq!(service.docker.update.automatic, true);
        assert_eq!(service.docker.update.additional_tags.len(), 0);
        assert_eq!(service.environment.len(), 0);
        assert_eq!(service.secrets.len(), 0);
        assert_eq!(service.web.enabled, true);
        assert_eq!(service.web.base, None);
    }
}

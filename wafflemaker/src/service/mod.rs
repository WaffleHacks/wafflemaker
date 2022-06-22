use crate::config;
use globset::{Glob, GlobSet, GlobSetBuilder};
use serde::{Deserialize, Serialize};
use serde_with::{serde_as, DisplayFromStr, NoneAsEmptyString};
use std::fmt::Debug;
use std::{collections::HashMap, ffi::OsStr, path::Path};
use tokio::fs;

mod dependency;
mod name;
pub mod registry;
mod secret;

use dependency::*;
pub use name::ServiceName;
pub use secret::{Format, Part as AWSPart, Secret};

/// The configuration for a service
#[derive(Clone, Debug, Deserialize, Serialize)]
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
    pub fn name(path: &Path) -> ServiceName {
        let name = path
            .strip_prefix(&config::instance().git.clone_to)
            .unwrap_or(path)
            .with_extension("")
            .iter()
            .map(OsStr::to_str)
            .map(Option::unwrap)
            .collect::<Vec<_>>()
            .join("/");

        ServiceName::new(name)
    }
}

/// All the possible external dependencies a service can require.
#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct Dependencies {
    postgres: DynamicDependency,
    redis: SimpleDependency,
}

impl Dependencies {
    pub fn postgres<'v>(&'v self, default_role: &'v str) -> Option<ResolvedDependency<'v>> {
        self.postgres.resolve("POSTGRES_URL", default_role)
    }

    pub fn redis(&self) -> Option<&str> {
        self.redis.resolve("REDIS_URL")
    }
}

/// The docker image configuration
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Docker {
    pub image: String,
    pub tag: String,
    #[serde(default)]
    pub update: AutoUpdate,
}

impl Docker {
    /// Get a glob for all the possible tags that can be updated
    pub fn allowed_tags(&self) -> Result<GlobSet, globset::Error> {
        let mut globs = self
            .update
            .additional_tags
            .iter()
            .map(Glob::glob)
            .collect::<Vec<&str>>();
        globs.push(&self.tag);

        let mut set = GlobSetBuilder::new();
        for glob in globs {
            let pattern = Glob::new(glob)?;
            set.add(pattern);
        }

        set.build()
    }
}

#[serde_as]
#[derive(Clone, Debug, Deserialize, Serialize)]
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
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Web {
    #[serde(default = "default_true")]
    pub enabled: bool,
    #[serde(default)]
    #[serde_as(as = "NoneAsEmptyString")]
    pub domain: Option<String>,
    #[serde(default)]
    #[serde_as(as = "NoneAsEmptyString")]
    pub path: Option<String>,
}

impl Default for Web {
    fn default() -> Web {
        Web {
            enabled: true,
            domain: None,
            path: None,
        }
    }
}

fn default_true() -> bool {
    true
}

#[cfg(test)]
mod tests {
    use super::Service;
    use crate::service::dependency::ResolvedDependency;

    #[tokio::test]
    async fn deserialize() {
        let service = Service::parse("../example-service.toml")
            .await
            .expect("failed to parse service");

        assert_eq!(
            service.dependencies.postgres("testing"),
            Some(ResolvedDependency::new("DATABASE_URL", "testing"))
        );
        assert_eq!(service.dependencies.redis(), None);
        assert_eq!(service.docker.image, "wafflehacks/cms");
        assert_eq!(service.docker.tag, "develop");
        assert_eq!(service.docker.update.automatic, true);
        assert_eq!(service.docker.update.additional_tags.len(), 1);
        assert_eq!(service.environment.len(), 4);
        assert_eq!(service.secrets.len(), 6);
        assert_eq!(service.web.enabled, true);
        assert_eq!(service.web.domain, Some("testing.wafflehacks.tech".into()));
        assert_eq!(service.web.path, Some("/testing".into()));
    }

    #[tokio::test]
    async fn defaults() {
        let service = Service::parse("testdata/service/minimal.toml")
            .await
            .expect("failed to parse service");

        assert_eq!(service.dependencies.postgres("testing"), None);
        assert_eq!(service.dependencies.redis(), None);
        assert_eq!(service.docker.image, "wafflehacks/cms");
        assert_eq!(service.docker.tag, "develop");
        assert_eq!(service.docker.update.automatic, true);
        assert_eq!(service.docker.update.additional_tags.len(), 0);
        assert_eq!(service.environment.len(), 0);
        assert_eq!(service.secrets.len(), 0);
        assert_eq!(service.web.enabled, true);
        assert_eq!(service.web.domain, None);
        assert_eq!(service.web.path, None);
    }
}

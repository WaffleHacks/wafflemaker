use super::{DynamicDependency, ResolvedDependency, SimpleDependency};
use globset::{Glob, GlobSet, GlobSetBuilder};
use serde::{Deserialize, Serialize};
use serde_with::{serde_as, DisplayFromStr, NoneAsEmptyString};

/// All the possible external dependencies a service can require
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
        let mut set = GlobSetBuilder::new();
        set.add(Glob::new(&self.tag)?);

        for glob in &self.update.additional_tags {
            set.add(glob.clone());
        }

        set.build()
    }
}

/// The docker image auto-update configuration
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
    fn default() -> Self {
        Self {
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
}

impl Default for Web {
    fn default() -> Self {
        Self {
            enabled: true,
            domain: None,
        }
    }
}

fn default_true() -> bool {
    true
}

use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use std::ffi::OsStr;
use std::path::Path;

mod dependency;
pub mod parts;
mod secret;

pub use dependency::{DynamicDependency, ResolvedDependency, SimpleDependency};
pub use secret::{AwsPart, Format, Secret};

/// A service specification file
#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Service {
    #[serde(default)]
    pub dependencies: parts::Dependencies,
    pub docker: parts::Docker,
    #[serde(default)]
    pub environment: HashMap<String, String>,
    #[serde(default)]
    pub secrets: HashMap<String, Secret>,
    #[serde(default)]
    pub web: parts::Web,
}

/// Generate the name of a service from its file path
pub fn service_name(path: &Path, base: &Path) -> String {
    path.strip_prefix(base)
        .unwrap_or(path)
        .with_extension("")
        .iter()
        .rev()
        .map(OsStr::to_str)
        .map(Option::unwrap)
        .collect::<Vec<_>>()
        .join("-")
}

#[cfg(test)]
mod tests {
    use super::{service_name, ResolvedDependency, Service};
    use std::path::PathBuf;

    #[test]
    fn service_name_simple() {
        let base = PathBuf::from("/this/is/the/base");
        let path = base.join("service.toml");

        assert_eq!(service_name(&path, &base), "service");
    }

    #[test]
    fn service_name_nested() {
        let base = PathBuf::from("/this/is/the/base");
        let path = base.join("some/service.toml");

        assert_eq!(service_name(&path, &base), "service-some");
    }

    macro_rules! parse {
        ($path:expr) => {{
            let path = PathBuf::from($path);
            let content = tokio::fs::read(path).await.expect("failed to open file");
            toml::from_slice::<Service>(&content).expect("failed to parse service")
        }};
    }

    #[tokio::test]
    async fn deserialize() {
        let service = parse!("../example-service.toml");

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
    }

    #[tokio::test]
    async fn defaults() {
        let service = parse!("testdata/minimal.toml");

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
    }
}

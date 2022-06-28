use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Clone, Debug, Default, Deserialize, Serialize)]
pub struct Dependencies(HashMap<String, Dependency>);

impl Dependencies {
    /// Retrieve the environment variable name for a dependency requiring no extra configuration.
    pub fn simple<'v>(&'v self, key: &str, default: &'v str) -> Option<&'v str> {
        let dependency = self.0.get(key)?;
        match dependency {
            Dependency::State(false) => None,
            Dependency::State(true) => Some(default),
            Dependency::Rename(name) => Some(name.as_str()),
            Dependency::Role { name, .. } => {
                Some(name.as_ref().map(String::as_str).unwrap_or(default))
            }
        }
    }

    /// Retrieve the config for a dependency that will generate credentials from Vault and
    /// requires a role.
    pub fn dynamic<'v>(
        &'v self,
        key: &str,
        default_env: &'v str,
        default_role: &'v str,
    ) -> Option<ResolvedDependency<'v>> {
        let dependency = self.0.get(key)?;
        match dependency {
            Dependency::State(false) => None,
            Dependency::State(true) => Some(ResolvedDependency::new(default_env, default_role)),
            Dependency::Rename(name) => Some(ResolvedDependency::new(name.as_str(), default_role)),
            Dependency::Role { name, role } => {
                let env = name.as_ref().map(String::as_str).unwrap_or(default_env);
                Some(ResolvedDependency::new(env, role.as_str()))
            }
        }
    }

    /// Get all the requested dependencies
    pub fn all(&self) -> Vec<String> {
        self.0.keys().map(String::to_owned).collect()
    }
}

/// A dependency that pulls credentials from Vault and requires a role. Like a `SimpleDependency`,
/// it can be explicitly enabled with a default environment variable name, or implicitly enabled
/// with a custom environment variable name. However, it can also take a custom role to pull
/// credentials from which will also implicitly enable it.
#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(untagged)]
pub enum Dependency {
    State(bool),
    Rename(String),
    Role { role: String, name: Option<String> },
}

impl Default for Dependency {
    fn default() -> Dependency {
        Dependency::State(false)
    }
}

/// The collapsed version of a `DynamicDependency` that has a value for both the
/// name and role, whether they are the default or not.
#[derive(Debug, PartialEq)]
pub struct ResolvedDependency<'value> {
    pub name: &'value str,
    pub role: &'value str,
}

impl<'v> ResolvedDependency<'v> {
    pub(crate) fn new<N, R>(name: &'v N, role: &'v R) -> ResolvedDependency<'v>
    where
        N: AsRef<str> + ?Sized,
        R: AsRef<str> + ?Sized,
    {
        ResolvedDependency {
            name: name.as_ref(),
            role: role.as_ref(),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{Dependencies, ResolvedDependency};
    use serde::Deserialize;
    use std::fs;

    #[derive(Deserialize)]
    struct Test {
        dependencies: Dependencies,
    }

    #[test]
    fn dynamic() {
        let raw =
            fs::read("testdata/service/dependencies.toml").expect("failed to parse test service");
        let parsed = toml::from_slice::<Test>(&raw).expect("failed to parse toml");
        let dependencies = parsed.dependencies;

        assert_eq!(dependencies.dynamic("state_false", "test", "test"), None);
        assert_eq!(
            dependencies.dynamic("state_true", "test", "test"),
            Some(ResolvedDependency::new("test", "test"))
        );
        assert_eq!(
            dependencies.dynamic("rename", "test", "test"),
            Some(ResolvedDependency::new("dynamic", "test"))
        );
        assert_eq!(
            dependencies.dynamic("role", "test", "test"),
            Some(ResolvedDependency::new("test", "dynamic"))
        );
        assert_eq!(
            dependencies.dynamic("role_rename", "test", "test"),
            Some(ResolvedDependency::new("dynamic", "dynamic"))
        );
    }

    #[test]
    fn simple() {
        let raw =
            fs::read("testdata/service/dependencies.toml").expect("failed to parse test service");
        let parsed = toml::from_slice::<Test>(&raw).expect("failed to parse toml");
        let dependencies = parsed.dependencies;

        assert_eq!(dependencies.simple("state_false", "test"), None);
        assert_eq!(dependencies.simple("state_true", "test"), Some("test"));
        assert_eq!(dependencies.simple("rename", "test"), Some("dynamic"));
        assert_eq!(dependencies.simple("role", "test"), Some("test"));
        assert_eq!(dependencies.simple("role_rename", "test"), Some("dynamic"));
    }
}

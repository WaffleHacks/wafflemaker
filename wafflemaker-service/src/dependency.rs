use serde::{Deserialize, Serialize};

/// A simple dependency that can be toggled on or off with a boolean, or implicitly enabled
/// by specifying an environment variable name.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(untagged)]
pub enum SimpleDependency {
    State(bool),
    Rename(String),
}

impl SimpleDependency {
    /// Resolve the dependency to an environment variable name
    pub fn resolve<'n>(&'n self, default: &'n str) -> Option<&'n str> {
        match self {
            Self::Rename(name) => Some(name),
            Self::State(true) => Some(default),
            Self::State(false) => None,
        }
    }
}

impl Default for SimpleDependency {
    fn default() -> Self {
        Self::State(false)
    }
}

/// A dependency that pulls credentials from Vault and requires a role. Like a `SimpleDependency`,
/// it can be explicitly enabled with a default environment variable name, or implicitly enabled
/// with a custom environment variable name. However, it can also take a custom role to pull
/// credentials from which will also implicitly enable it.
#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(untagged)]
pub enum DynamicDependency {
    State(bool),
    Rename(String),
    Role { role: String, name: Option<String> },
}

impl DynamicDependency {
    /// Resolve the dependency to an environment variable name and role id
    pub fn resolve<'v>(
        &'v self,
        default_env: &'v str,
        default_role: &'v str,
    ) -> Option<ResolvedDependency<'v>> {
        match self {
            Self::Rename(name) => Some(ResolvedDependency::new(name, default_role)),
            Self::Role { name, role } => {
                let name = name.as_ref().map(String::as_str).unwrap_or(default_env);
                Some(ResolvedDependency::new(name, role))
            }
            Self::State(true) => Some(ResolvedDependency::new(default_env, default_role)),
            Self::State(false) => None,
        }
    }
}

impl Default for DynamicDependency {
    fn default() -> Self {
        Self::State(false)
    }
}

/// The collapsed version of a [DynamicDependency] that has a value for both the environment
/// variable name and role, whether the are the default or not.
#[derive(Debug, PartialEq)]
pub struct ResolvedDependency<'v> {
    pub name: &'v str,
    pub role: &'v str,
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
    use super::{DynamicDependency, ResolvedDependency, SimpleDependency};
    use serde::Deserialize;
    use std::fs;

    #[test]
    fn dynamic() {
        #[derive(Debug, Deserialize)]
        struct DynamicTest {
            state_false: DynamicDependency,
            state_true: DynamicDependency,
            rename: DynamicDependency,
            role: DynamicDependency,
            role_rename: DynamicDependency,
        }

        macro_rules! run {
            ($field:expr; none) => {
                assert_eq!($field.resolve("test", "test"), None);
            };
            ($field:expr; default) => {
                run!($field; env = "test"; role = "test");
            };
            ($field:expr; env = $env:expr) => {
                run!($field; env = $env; role = "test");
            };
            ($field:expr; role = $role:expr) => {
                run!($field; env = "test"; role = $role);
            };
            ($field:expr; env = $env:expr; role = $role:expr) => {
                assert_eq!($field.resolve("test", "test"), Some(ResolvedDependency::new($env, $role)));
            };
        }

        let raw =
            fs::read("testdata/dependency_dynamic.toml").expect("failed to parse test service");
        let parsed = toml::from_slice::<DynamicTest>(&raw).expect("failed to parse toml");

        run!(parsed.state_false; none);
        run!(parsed.state_true; default);
        run!(parsed.rename; env = "dynamic");
        run!(parsed.role; role = "dynamic");
        run!(parsed.role_rename; env = "dynamic"; role = "dynamic");
    }

    #[test]
    fn simple() {
        #[derive(Debug, Deserialize)]
        struct SimpleTest {
            state_false: SimpleDependency,
            state_true: SimpleDependency,
            rename: SimpleDependency,
        }

        let raw =
            fs::read("testdata/dependency_simple.toml").expect("failed to parse test service");
        let parsed = toml::from_slice::<SimpleTest>(&raw).expect("failed to parse toml");

        assert_eq!(parsed.state_false.resolve("test"), None);
        assert_eq!(parsed.state_true.resolve("test"), Some("test"));
        assert_eq!(parsed.rename.resolve("test"), Some("simple"));
    }
}

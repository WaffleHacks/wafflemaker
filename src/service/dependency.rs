use serde::Deserialize;

/// A simple dependency that can be toggled on or off with a boolean, or implicitly enabled
/// by specifying an environment variable name.
#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum SimpleDependency {
    State(bool),
    Rename(String),
}

impl SimpleDependency {
    pub fn resolve<'n>(&'n self, default: &'n str) -> Option<&'n str> {
        match self {
            Self::Rename(name) => Some(&name),
            Self::State(true) => Some(default),
            Self::State(false) => None,
        }
    }
}

impl Default for SimpleDependency {
    fn default() -> SimpleDependency {
        SimpleDependency::State(false)
    }
}

/// A dependency that pulls credentials from Vault and requires a role. Like a `SimpleDependency`,
/// it can be explicitly enabled with a default environment variable name, or implicitly enabled
/// with a custom environment variable name. However, it can also take a custom role to pull
/// credentials from which will also implicitly enable it.
#[derive(Clone, Debug, Deserialize, PartialEq)]
#[serde(untagged)]
pub enum DynamicDependency {
    State(bool),
    Rename(String),
    Role { role: String, name: Option<String> },
}

impl DynamicDependency {
    pub fn resolve<'v>(
        &'v self,
        default_env: &'v str,
        default_role: &'v str,
    ) -> Option<ResolvedDependency<'v, 'v>> {
        let (name, role) = match self {
            Self::Rename(name) => (name.as_str(), default_role),
            Self::Role {
                name: variable,
                role,
            } => {
                let env = variable.as_ref().map(|s| s.as_str()).unwrap_or(default_env);
                (env, role.as_str())
            }
            Self::State(true) => (default_env, default_role),
            Self::State(false) => return None,
        };
        Some(ResolvedDependency::new(name, role))
    }
}

impl Default for DynamicDependency {
    fn default() -> DynamicDependency {
        DynamicDependency::State(false)
    }
}

/// The collapsed version of a `DynamicDependency` that has a value for both the
/// name and role, whether they are the default or not.
#[derive(Debug, PartialEq)]
pub struct ResolvedDependency<'name, 'role> {
    pub name: &'name str,
    pub role: &'role str,
}

impl<'n, 'r> ResolvedDependency<'n, 'r> {
    pub(crate) fn new<N, R>(name: &'n N, role: &'r R) -> ResolvedDependency<'n, 'r>
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

    #[derive(Debug, Deserialize)]
    struct DynamicTest {
        state_false: DynamicDependency,
        state_true: DynamicDependency,
        rename: DynamicDependency,
        role: DynamicDependency,
        role_rename: DynamicDependency,
    }

    #[derive(Debug, Deserialize)]
    struct SimpleTest {
        state_false: SimpleDependency,
        state_true: SimpleDependency,
        rename: SimpleDependency,
    }

    #[test]
    fn dynamic() {
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

        let raw = fs::read("./testdata/service/dependency_dynamic.toml")
            .expect("failed to parse test service");
        let parsed = toml::from_slice::<DynamicTest>(&raw).expect("failed to parse toml");

        run!(parsed.state_false; none);
        run!(parsed.state_true; default);
        run!(parsed.rename; env = "dynamic");
        run!(parsed.role; role = "dynamic");
        run!(parsed.role_rename; env = "dynamic"; role = "dynamic");
    }

    #[test]
    fn simple() {
        let raw = fs::read("./testdata/service/dependency_simple.toml")
            .expect("failed to parse test service");
        let parsed = toml::from_slice::<SimpleTest>(&raw).expect("failed to parse toml");

        assert_eq!(parsed.state_false.resolve("test"), None);
        assert_eq!(parsed.state_true.resolve("test"), Some("test"));
        assert_eq!(parsed.rename.resolve("test"), Some("simple"));
    }
}

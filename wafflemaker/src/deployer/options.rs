use std::collections::HashMap;

#[derive(Debug, PartialEq)]
pub struct RoutingOpts {
    pub domain: String,
    pub path: Option<String>,
}

/// Options for creating a container
#[derive(Debug, PartialEq)]
pub struct CreateOpts {
    pub name: String,
    pub routing: Option<RoutingOpts>,
    pub environment: HashMap<String, String>,
    pub image: String,
    pub tag: String,
}

impl CreateOpts {
    /// Create a new builder for the container options
    pub fn builder() -> CreateOptsBuilder {
        CreateOptsBuilder::new()
    }
}

/// The builder for container options
#[derive(Debug, Default)]
pub struct CreateOptsBuilder {
    name: String,
    routing: Option<RoutingOpts>,
    environment: HashMap<String, String>,
    image: String,
    tag: String,
}

impl CreateOptsBuilder {
    /// Create a new `ServiceOptsBuilder`
    pub fn new() -> Self {
        Default::default()
    }

    /// Set the deployment name
    pub fn name<S: Into<String>>(mut self, name: S) -> Self {
        self.name = name.into();
        self
    }

    /// Set the routing options
    pub fn routing<S: Into<String>>(mut self, domain: S, path: Option<&str>) -> Self {
        self.routing = Some(RoutingOpts {
            domain: domain.into(),
            path: path.map(|s| s.to_owned()),
        });
        self
    }

    /// Set the image to deploy
    pub fn image<S: Into<String>>(mut self, image: S, tag: S) -> Self {
        self.image = image.into();
        self.tag = tag.into();
        self
    }

    /// Add an environment variable
    pub fn environment<K: Into<String>, V: Into<String>>(mut self, key: K, value: V) -> Self {
        self.environment
            .insert(key.into().to_uppercase(), value.into());
        self
    }

    /// Build the options
    pub fn build(self) -> CreateOpts {
        CreateOpts {
            name: self.name,
            routing: self.routing,
            environment: self.environment,
            image: self.image,
            tag: self.tag,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{CreateOpts, RoutingOpts};
    use std::collections::HashMap;

    #[test]
    fn container_opts_builder() {
        let mut map = HashMap::new();
        map.insert("HELLO".into(), "world".into());
        map.insert(
            "DATABASE_URL".into(),
            "postgres://user:password@0.0.0.0:5432/database".into(),
        );
        map.insert("ANOTHER".into(), "VaLuE".into());

        let opts = CreateOpts {
            name: "hello-world".into(),
            routing: Some(RoutingOpts {
                domain: "hello.world".into(),
                path: Some("/testing".into()),
            }),
            environment: map,
            image: "wafflehacks/testing".into(),
            tag: "latest".into(),
        };
        let from_builder = CreateOpts::builder()
            .name("hello-world")
            .image("wafflehacks/testing", "latest")
            .routing("hello.world", Some("/testing"))
            .environment("another", "VaLuE")
            .environment(
                "database_url",
                "postgres://user:password@0.0.0.0:5432/database",
            )
            .environment("hello", "world")
            .build();

        assert_eq!(opts, from_builder);
    }
}

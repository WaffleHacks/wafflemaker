use crate::config::{Deployment, DeploymentEngine};
use arc_swap::ArcSwap;
use async_trait::async_trait;
use once_cell::sync::Lazy;
use std::{collections::HashMap, sync::Arc};

mod docker;
mod error;
mod noop;

use docker::Docker;
pub use error::Error;
use error::Result;

static STATIC_INSTANCE: Lazy<ArcSwap<Box<dyn Deployer>>> =
    Lazy::new(|| ArcSwap::from_pointee(Box::new(noop::Noop)));

/// Connect to the deployer service
fn connect(config: &Deployment) -> Result<Box<dyn Deployer>> {
    let domain = config.domain.to_owned();
    let deployer: Box<dyn Deployer> = match &config.engine {
        DeploymentEngine::Docker {
            connection,
            endpoint,
            timeout,
            state,
        } => Box::new(Docker::new(connection, endpoint, timeout, domain, state)?),
    };

    Ok(deployer)
}

/// Create the deployer service and test its connection
pub async fn initialize(config: &Deployment) -> Result<()> {
    let deployer = connect(config)?;
    deployer.test().await?;

    STATIC_INSTANCE.swap(Arc::from(deployer));
    Ok(())
}

/// Retrieve an instance of the deployer service
pub fn instance() -> Arc<Box<dyn Deployer>> {
    STATIC_INSTANCE.load().clone()
}

/// The interface for managing the deployments
#[async_trait]
pub trait Deployer: Send + Sync {
    /// Test the connection to the deployer
    async fn test(&self) -> Result<()>;

    /// Get a list of all the running services
    async fn list(&self) -> Result<Vec<ServiceInfo>>;

    /// Create a new service
    async fn create(&self, options: CreateOpts) -> Result<String>;

    /// Start a service
    async fn start(&self, name: String) -> Result<()>;

    /// Stop a service
    async fn stop(&self, name: String) -> Result<()>;

    /// Delete a service
    async fn delete(&self, name: String) -> Result<()>;
}

/// Information about a container
#[derive(Debug)]
pub struct ServiceInfo {
    id: String,
    domain: Option<String>,
    image: String,
    status: Status,
}

/// Possible states a service can be in
#[derive(Debug)]
pub enum Status {
    Created,
    Running,
    Restarting,
    Stopped,
    Killed,
}

/// Options for creating a container
#[derive(Debug, PartialEq)]
pub struct CreateOpts {
    name: String,
    domain: Option<String>,
    environment: HashMap<String, String>,
    image: String,
    tag: String,
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
    domain: Option<String>,
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

    /// Set the domain
    pub fn domain<S: Into<String>>(mut self, domain: S) -> Self {
        self.domain = Some(domain.into());
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
            domain: self.domain,
            environment: self.environment,
            image: self.image,
            tag: self.tag,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::CreateOpts;
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
            domain: Some("hello.world".into()),
            environment: map,
            image: "wafflehacks/testing".into(),
            tag: "latest".into(),
        };
        let from_builder = CreateOpts::builder()
            .name("hello-world")
            .image("wafflehacks/testing", "latest")
            .domain("hello.world")
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

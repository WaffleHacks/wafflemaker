use crate::config::{Deployment, DeploymentEngine};
use async_trait::async_trait;
use once_cell::sync::OnceCell;
use std::{collections::HashMap, sync::Arc};
use tokio::sync::broadcast::Receiver;

mod docker;
mod error;

use docker::Docker;
pub use error::Error;
use error::Result;

static INSTANCE: OnceCell<Arc<Box<dyn Deployer>>> = OnceCell::new();

/// Create the deployer service and test its connection
pub async fn initialize(config: &Deployment, stop: Receiver<()>) -> Result<()> {
    let domain = config.domain.to_owned();
    let deployer: Box<dyn Deployer> = match &config.engine {
        DeploymentEngine::Docker {
            connection,
            endpoint,
            timeout,
            network,
            state,
        } => Box::new(
            Docker::new(connection, endpoint, timeout, domain, network, state, stop).await?,
        ),
    };

    deployer.test().await?;

    INSTANCE.get_or_init(|| Arc::from(deployer));
    Ok(())
}

/// Retrieve an instance of the deployer service
pub fn instance() -> Arc<Box<dyn Deployer>> {
    INSTANCE.get().unwrap().clone()
}

/// The interface for managing the deployments
#[async_trait]
pub trait Deployer: Send + Sync {
    /// Test the connection to the deployer
    async fn test(&self) -> Result<()>;

    /// Get a map of all the registered services from the name to the id
    async fn list(&self) -> Result<HashMap<String, String>>;

    /// Get a service's deployment id from its name
    async fn service_id(&self, name: &str) -> Result<Option<String>>;

    /// Create a new service
    async fn create(&self, options: CreateOpts) -> Result<String>;

    /// Start a service with its ID
    async fn start(&self, id: &str) -> Result<()>;

    /// Get a service's internal IP address
    async fn ip(&self, id: &str) -> Result<String>;

    /// Stop a service with its ID
    async fn stop(&self, id: &str) -> Result<()>;

    /// Delete a service by its ID
    async fn delete(&self, id: &str) -> Result<()>;

    /// Delete a service by its name
    async fn delete_by_name(&self, name: &str) -> Result<()>;
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

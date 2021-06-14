use crate::config::{Deployment, DeploymentEngine};
use arc_swap::ArcSwap;
use async_trait::async_trait;
use once_cell::sync::Lazy;
use std::{collections::HashMap, sync::Arc};

mod docker;
mod error;

use docker::Docker;
pub use error::Error;
use error::Result;

static STATIC_INSTANCE: Lazy<ArcSwap<Box<dyn Deployer>>> = Lazy::new(|| {
    ArcSwap::from_pointee(
        connect(&Deployment::default())
            .expect("failed to connect to local docker socket for deployment"),
    )
});

/// Connect to the deployer service
fn connect(config: &Deployment) -> Result<Box<dyn Deployer>> {
    let domain = config.domain.to_owned();
    let deployer: Box<dyn Deployer> = match &config.engine {
        DeploymentEngine::Docker {
            connection,
            endpoint,
            timeout,
        } => Box::new(Docker::new(connection, endpoint, timeout, domain)?),
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
    async fn start(&self, id: String) -> Result<()>;

    /// Stop a service
    async fn stop(&self, id: String) -> Result<()>;

    /// Delete a service
    async fn delete(&self, id: String) -> Result<()>;
}

/// Information about a container
#[derive(Debug)]
pub struct ServiceInfo {
    id: String,
    subdomain: String,
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
    subdomain: String,
    environment: HashMap<String, String>,
    image: String,
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
    subdomain: String,
    environment: HashMap<String, String>,
    image: String,
}

impl CreateOptsBuilder {
    /// Create a new `ServiceOptsBuilder`
    pub fn new() -> Self {
        Default::default()
    }

    /// Set the subdomain
    pub fn subdomain(mut self, name: String) -> Self {
        self.subdomain = name;
        self
    }

    /// Set the image to deploy
    pub fn image(mut self, image: String, tag: String) -> Self {
        self.image = format!("{}:{}", image, tag);
        self
    }

    /// Add an environment variable
    pub fn environment<I: Into<String>>(mut self, key: String, value: I) -> Self {
        self.environment.insert(key.to_uppercase(), value.into());
        self
    }

    /// Build the options
    pub fn build(self) -> CreateOpts {
        CreateOpts {
            subdomain: self.subdomain,
            environment: self.environment,
            image: self.image,
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
            subdomain: "hello.world".into(),
            environment: map,
            image: "wafflehacks/testing:latest".into(),
        };
        let from_builder = CreateOpts::builder()
            .image("wafflehacks/testing".into(), "latest".into())
            .subdomain("hello.world".into())
            .environment("another".into(), "VaLuE")
            .environment(
                "database_url".into(),
                "postgres://user:password@0.0.0.0:5432/database",
            )
            .environment("hello".into(), "world")
            .build();

        assert_eq!(opts, from_builder);
    }
}

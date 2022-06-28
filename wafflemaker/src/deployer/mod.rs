use crate::config::Deployment;
use once_cell::sync::OnceCell;
use std::sync::Arc;
use tokio::sync::broadcast::Receiver;

mod error;
mod events;
mod options;
mod service;

pub use error::Error;
use error::Result;
pub use options::{CreateOpts, RoutingOpts};
use service::Deployer;

static INSTANCE: OnceCell<Arc<Deployer>> = OnceCell::new();

/// Create the deployer service and test its connection
pub async fn initialize(config: &Deployment, dns_server: &str, stop: Receiver<()>) -> Result<()> {
    let deployer = Deployer::new(config, dns_server, stop).await?;
    deployer.test().await?;

    INSTANCE.get_or_init(|| Arc::from(deployer));
    Ok(())
}

/// Retrieve an instance of the deployer service
pub fn instance() -> Arc<Deployer> {
    INSTANCE.get().unwrap().clone()
}

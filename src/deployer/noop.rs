use super::{CreateOpts, Deployer, Result, ServiceInfo};
use async_trait::async_trait;

/// This is designed to be a stub deployer while the wafflemaker is warming up.
/// It does absolutely nothing.
#[derive(Debug)]
pub(crate) struct Noop;

#[async_trait]
impl Deployer for Noop {
    async fn test(&self) -> Result<()> {
        panic!("Noop deployer should not be called, use deployer::initialize")
    }

    async fn list(&self) -> Result<Vec<ServiceInfo>> {
        panic!("Noop deployer should not be called, use deployer::initialize")
    }

    async fn create(&self, _: CreateOpts) -> Result<String> {
        panic!("Noop deployer should not be called, use deployer::initialize")
    }

    async fn start(&self, _: String) -> Result<()> {
        panic!("Noop deployer should not be called, use deployer::initialize")
    }

    async fn stop(&self, _: String) -> Result<()> {
        panic!("Noop deployer should not be called, use deployer::initialize")
    }

    async fn delete(&self, _: String) -> Result<()> {
        panic!("Noop deployer should not be called, use deployer::initialize")
    }
}

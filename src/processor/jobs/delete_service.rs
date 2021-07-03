use super::Job;
use crate::{deployer, dns, fail};
use async_trait::async_trait;
use tracing::{info, instrument};

#[derive(Debug)]
pub struct DeleteService {
    name: String,
}

impl DeleteService {
    /// Create a new delete service job
    pub fn new(name: String) -> Self {
        Self { name }
    }
}

#[async_trait]
impl Job for DeleteService {
    #[instrument(skip(self), fields(name = %self.name))]
    async fn run(&self) {
        fail!(dns::instance().delete(&self.name).await);
        info!("deleted DNS records (if they existed)");

        fail!(deployer::instance().delete(self.name.clone()).await);
        info!("successfully deleted deployment");
    }

    fn name<'a>(&self) -> &'a str {
        "delete_service"
    }
}

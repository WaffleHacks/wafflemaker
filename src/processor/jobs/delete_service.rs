use super::Job;
use crate::{deployer, fail, vault};
use async_trait::async_trait;
use tracing::{debug, info, instrument};

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
        if let Err(_) = deployer::instance().stop(self.name.clone()).await {
            debug!("deployment already stopped");
        }

        fail!(deployer::instance().delete(self.name.clone()).await);

        if let Err(_) = vault::instance().delete_database_role(&self.name).await {
            debug!("no database role configured");
        }

        info!("successfully deleted deployment");
    }

    fn name<'a>(&self) -> &'a str {
        "delete_service"
    }
}

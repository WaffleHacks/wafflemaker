use super::Job;
use crate::{deployer, fail, service::registry::REGISTRY, vault};
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
        let mut reg = REGISTRY.write().await;
        reg.remove(&self.name);

        let id = match fail!(deployer::instance().service_id(&self.name).await) {
            Some(id) => id,
            None => {
                info!("deployment does not exist");
                return;
            }
        };

        if deployer::instance().stop(&id).await.is_err() {
            debug!("deployment already stopped");
        }

        fail!(deployer::instance().delete_by_name(&self.name).await);

        fail!(vault::instance().revoke_leases(&id).await);

        if vault::instance()
            .delete_database_role(&self.name)
            .await
            .is_err()
        {
            debug!("no database role configured");
        }

        info!("successfully deleted deployment");
    }

    fn name<'a>(&self) -> &'a str {
        "delete_service"
    }
}

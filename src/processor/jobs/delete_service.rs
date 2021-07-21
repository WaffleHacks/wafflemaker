use super::Job;
use crate::{
    deployer, fail_notify,
    notifier::{self, Event, State},
    service::registry::REGISTRY,
    vault,
};
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
        macro_rules! fail {
            ($result:expr) => {
                fail_notify!(service_delete, &self.name; $result; "an error occurred while deleting service");
            };
        }

        let mut reg = REGISTRY.write().await;
        if reg.remove(&self.name).is_none() {
            info!("service was never deployed, skipping");
            notifier::notify(Event::service_delete(&self.name, State::Success)).await;
            return;
        }

        notifier::notify(Event::service_delete(&self.name, State::InProgress)).await;

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
        notifier::notify(Event::service_delete(&self.name, State::Success)).await;
    }

    fn name<'a>(&self) -> &'a str {
        "delete_service"
    }
}

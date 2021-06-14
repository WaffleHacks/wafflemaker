use super::{Job, SharedJobQueue};
use crate::service::Service;
use async_trait::async_trait;
use std::path::PathBuf;
use tracing::instrument;

#[derive(Debug)]
pub struct UpdateService {
    config: Service,
    name: String,
}

impl UpdateService {
    /// Create a new update service job
    pub fn new(config: Service, path: PathBuf) -> Self {
        let name = path.to_str().unwrap().replace("/", ".");
        Self { config, name }
    }
}

#[async_trait]
impl Job for UpdateService {
    #[instrument(
        name = "update_service",
        skip(self, _queue),
        fields(name = %self.name)
    )]
    async fn run(&self, _queue: SharedJobQueue) {
        // TODO: begin deployment
    }

    fn name<'a>(&self) -> &'a str {
        "update_service"
    }
}

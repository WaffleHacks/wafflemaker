use super::Job;
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
    #[instrument(skip(self), fields(name = %self.name))]
    async fn run(&self) {
        // TODO: begin deployment
    }

    fn name<'a>(&self) -> &'a str {
        "update_service"
    }
}

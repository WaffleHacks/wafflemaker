use super::{Job, SharedJobQueue};
use crate::git::Repository;
use async_trait::async_trait;
use std::{path::PathBuf, sync::Arc};
use tracing::instrument;

#[derive(Debug)]
pub struct DeleteService {
    name: String,
}

impl DeleteService {
    /// Create a new delete service job
    pub fn new(path: PathBuf) -> Self {
        let name = path.to_str().unwrap().replace("/", ".");
        Self { name }
    }
}

#[async_trait]
impl Job for DeleteService {
    #[instrument(
        name = "delete_service",
        skip(self, path, queue, repo),
        fields(name = %self.name)
    )]
    async fn run(&self, path: Arc<PathBuf>, queue: SharedJobQueue, repo: &Repository) {
        // TODO: begin deletion
    }

    fn name<'a>(&self) -> &'a str {
        "delete_service"
    }
}

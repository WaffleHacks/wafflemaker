use super::Job;
use async_trait::async_trait;
use tracing::instrument;

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
        // TODO: begin deletion
    }

    fn name<'a>(&self) -> &'a str {
        "delete_service"
    }
}

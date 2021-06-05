use crate::{config::SharedConfig, git::Repository};
use std::sync::Arc;
use tokio::sync::watch;
use tracing::info;

pub mod jobs;
mod worker;

use jobs::{JobQueue, SharedJobQueue};

/// Create a new job processor
pub fn spawn(repo: Repository, config: SharedConfig) -> (SharedJobQueue, watch::Sender<bool>) {
    let queue = Arc::new(JobQueue::new());

    // Register handler to stop processing
    let (tx, rx) = watch::channel(false);

    info!(count = config.server.workers, "spawning job workers");

    let path = Arc::new(config.git.clone_to.clone());

    // Spawn the workers
    for id in 0..config.server.workers {
        tokio::spawn(worker::worker(
            id,
            path.clone(),
            repo.clone(),
            queue.clone(),
            rx.clone(),
        ));
    }

    (queue, tx)
}

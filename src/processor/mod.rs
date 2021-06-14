use crate::config::SharedConfig;
use std::sync::Arc;
use tokio::sync::watch;
use tracing::info;

pub mod jobs;
mod worker;

use jobs::{JobQueue, SharedJobQueue};

/// Create a new job processor
pub fn spawn(config: SharedConfig) -> (SharedJobQueue, watch::Sender<bool>) {
    let queue = Arc::new(JobQueue::new());

    // Register handler to stop processing
    let (tx, rx) = watch::channel(false);

    info!(count = config.server.workers, "spawning job workers");

    // Spawn the workers
    for id in 0..config.server.workers {
        tokio::spawn(worker::worker(id, queue.clone(), rx.clone()));
    }

    (queue, tx)
}

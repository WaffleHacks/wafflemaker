use crate::{
    git::Repository,
    jobs::{JobQueue, SharedJobQueue},
};
use std::sync::Arc;
use tokio::sync::watch;
use tracing::info;

mod worker;

/// Create a new job processor
pub fn spawn(repo: Repository, num_workers: u32) -> (SharedJobQueue, watch::Sender<bool>) {
    let queue = Arc::new(JobQueue::new());

    // Register handler to stop processing
    let (tx, rx) = watch::channel(false);

    info!(count = num_workers, "spawning job workers");

    // Spawn the workers
    for id in 0..num_workers {
        tokio::spawn(worker::worker(id, repo.clone(), queue.clone(), rx.clone()));
    }

    (queue, tx)
}

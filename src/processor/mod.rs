use crate::config::SharedConfig;
use tokio::sync::watch;
use tracing::info;

pub mod jobs;
mod worker;

/// Create a new job processor
pub fn spawn(config: SharedConfig) -> watch::Sender<bool> {
    // Register handler to stop processing
    let (tx, rx) = watch::channel(false);

    info!(count = config.server.workers, "spawning job workers");

    // Spawn the workers
    for id in 0..config.server.workers {
        tokio::spawn(worker::worker(id, rx.clone()));
    }

    tx
}

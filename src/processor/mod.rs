use crate::config::SharedConfig;
use tokio::sync::broadcast;
use tracing::info;

pub mod jobs;
mod worker;

/// Create a new job processor
pub fn spawn(config: SharedConfig, stop: broadcast::Sender<()>) {
    info!(count = config.agent.workers, "spawning job workers");

    // Spawn the workers
    for id in 0..config.agent.workers {
        tokio::spawn(worker::worker(id, stop.subscribe()));
    }
}

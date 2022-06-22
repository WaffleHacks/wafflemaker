use crate::config;
use tokio::sync::broadcast;
use tracing::info;

pub mod jobs;
mod worker;

/// Create a new job processor
pub fn spawn(stop: broadcast::Sender<()>) {
    let cfg = config::instance();
    info!(count = cfg.agent.workers, "spawning job workers");

    // Spawn the workers
    for id in 0..cfg.agent.workers {
        tokio::spawn(worker::worker(id, stop.subscribe()));
    }
}

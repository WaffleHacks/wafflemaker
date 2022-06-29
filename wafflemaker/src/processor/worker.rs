use crate::{processor::jobs, Config};
use std::sync::Arc;
use tokio::{select, sync::broadcast::Receiver};
use tracing::{info, instrument};

/// Process incoming job workloads
#[instrument(skip(stop))]
pub async fn worker(id: u32, config: Arc<Config>, mut stop: Receiver<()>) {
    info!("started worker {}", id);

    let queue = jobs::instance();
    loop {
        select! {
            _ = stop.recv() => {
                info!("worker stopping");
                break;
            }
            job = queue.pop() => {
                info!(name = job.name(), "received new job");
                job.run(config.clone()).await;
            }
        }
    }
}

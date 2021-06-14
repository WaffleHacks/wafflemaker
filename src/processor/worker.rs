use crate::processor::jobs;
use tokio::{select, sync::watch::Receiver};
use tracing::{info, instrument};

/// Process incoming job workloads
#[instrument(skip(stop))]
pub async fn worker(id: u32, mut stop: Receiver<bool>) {
    info!("started worker {}", id);

    let queue = jobs::instance();
    loop {
        select! {
            _ = stop.changed() => {
                info!("worker stopping");
                break;
            }
            job = queue.pop() => {
                info!(name = job.name(), "received new job");
                job.run().await;
            }
        }
    }
}

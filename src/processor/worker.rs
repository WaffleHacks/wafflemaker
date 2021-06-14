use crate::processor::jobs::SharedJobQueue;
use tokio::{select, sync::watch::Receiver};
use tracing::{info, instrument};

/// Process incoming job workloads
#[instrument(skip(queue, stop))]
pub async fn worker(id: u32, queue: SharedJobQueue, mut stop: Receiver<bool>) {
    info!("started worker {}", id);

    loop {
        select! {
            _ = stop.changed() => {
                info!("worker stopping");
                break;
            }
            job = queue.pop() => {
                info!(name = job.name(), "received new job");
                job.run(queue.clone()).await;
            }
        }
    }
}

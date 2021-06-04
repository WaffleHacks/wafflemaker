use crate::git::Repository;
use crate::jobs::SharedJobQueue;
use tokio::{select, sync::watch::Receiver};
use tracing::{info, instrument};

/// Process incoming job workloads
#[instrument(skip(queue, repo, stop))]
pub async fn worker(id: u32, repo: Repository, queue: SharedJobQueue, mut stop: Receiver<bool>) {
    info!("started worker {}", id);

    loop {
        select! {
            _ = stop.changed() => {
                info!("worker stopping");
                break;
            }
            job = queue.pop() => {
                info!("received new job");
                job.run(queue.clone(), &repo).await;
            }
        }
    }
}

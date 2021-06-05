use crate::{git::Repository, processor::jobs::SharedJobQueue};
use std::{path::PathBuf, sync::Arc};
use tokio::{select, sync::watch::Receiver};
use tracing::{info, instrument};

/// Process incoming job workloads
#[instrument(skip(queue, repo, stop))]
pub async fn worker(
    id: u32,
    path: Arc<PathBuf>,
    repo: Repository,
    queue: SharedJobQueue,
    mut stop: Receiver<bool>,
) {
    info!("started worker {}", id);

    loop {
        select! {
            _ = stop.changed() => {
                info!("worker stopping");
                break;
            }
            job = queue.pop() => {
                info!(name = job.name(), "received new job");
                job.run(path.clone(), queue.clone(), &repo).await;
            }
        }
    }
}

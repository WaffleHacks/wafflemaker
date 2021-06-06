use crate::{deployer::Deployer, git::Repository};
use async_trait::async_trait;
use deadqueue::unlimited::Queue;
use std::{path::PathBuf, sync::Arc};

mod delete_service;
mod plan_update;
mod update_service;

pub use delete_service::DeleteService;
pub use plan_update::PlanUpdate;
pub use update_service::UpdateService;

pub type JobQueue = Queue<Box<dyn Job>>;
pub type SharedJobQueue = Arc<JobQueue>;

/// Dispatch a job to one of the processors
pub fn dispatch(queue: SharedJobQueue, job: impl Job + 'static) {
    queue.push(Box::new(job))
}

/// A job that can be run in a separate thread
#[async_trait]
pub trait Job: Send + Sync {
    /// Run the job
    async fn run(
        &self,
        path: Arc<PathBuf>,
        queue: SharedJobQueue,
        repo: &Repository,
        deployer: Arc<Box<dyn Deployer>>,
    );

    /// The name of the job
    fn name<'a>(&self) -> &'a str;
}

/// Log error and stop execution from within a job
#[macro_export]
macro_rules! fail {
    ($result:expr) => {
        match $result {
            Ok(v) => v,
            Err(e) => {
                tracing::error!(error = %e, "an error occurred while processing the job");
                return;
            }
        }
    };
    ($result:expr ; $message:expr) => {
        fail!($result; $message,)
    };
    ($result:expr ; $fmt:expr , $( $arg:expr , )* ) => {
        match $result {
            Ok(v) => v,
            Err(e) => {
                tracing::error!(error = %e, $fmt, $( $arg ,)* );
                return;
            }
        }
    };
}

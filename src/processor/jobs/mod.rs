use crate::{config::SharedConfig, git::Repository};
use async_trait::async_trait;
use deadqueue::unlimited::Queue;
use std::sync::Arc;

mod plan_update;

pub use plan_update::PlanUpdate;

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
    async fn run(&self, config: SharedConfig, queue: SharedJobQueue, repo: &Repository);

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

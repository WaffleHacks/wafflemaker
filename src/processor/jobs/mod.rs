use async_trait::async_trait;
use deadqueue::unlimited::Queue;
use once_cell::sync::Lazy;
use std::sync::Arc;

mod delete_service;
mod plan_update;
mod update_service;

pub use delete_service::DeleteService;
pub use plan_update::PlanUpdate;
pub use update_service::UpdateService;

pub type JobQueue = Queue<Box<dyn Job>>;

static STATIC_INSTANCE: Lazy<Arc<JobQueue>> = Lazy::new(|| Arc::from(JobQueue::new()));

/// Dispatch a job to one of the processors
pub fn dispatch(job: impl Job + 'static) {
    STATIC_INSTANCE.push(Box::new(job));
}

/// Retrieve an instance of the queue
pub fn instance() -> Arc<JobQueue> {
    STATIC_INSTANCE.clone()
}

/// A job that can be run in a separate thread
#[async_trait]
pub trait Job: Send + Sync {
    /// Run the job
    async fn run(&self);

    /// The name of the job
    fn name<'a>(&self) -> &'a str;
}

/// Log error and stop execution from within a job
// TODO: notify job failed
#[macro_export]
macro_rules! fail {
    ($result:expr) => {
        match $result {
            Ok(v) => v,
            Err(e) => {
                fail!(@e e; "an error occurred while processing the job",);
                return;
            }
        }
    };
    ($result:expr ; $message:expr) => {
        fail!($result; $message,)
    };
    ($result:expr ; $fmt:expr , $( $arg:expr ),* ) => {
        match $result {
            Ok(v) => v,
            Err(e) => {
                fail!(@e e; $fmt, $( $arg, )* );
                return;
            }
        }
    };

    // Internal rules for displaying the error
    (@e $error:expr; $fmt:expr , $( $arg:expr ),* ) => {
        use std::error::Error;
        match $error.source() {
            Some(e) => tracing::error!(error = %$error, source = %e, $fmt, $( $arg , )*),
            None => tracing::error!(error = %$error, $fmt, $( $arg , )*),
        }
    };
}

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

/// Log error and stop execution from within a job. A notification
/// with the specified event and args will also be sent.
///
/// This is intended to be called from a macro within the function so extra
/// arguments are automatically passed in.
///
/// **NOTE:** a status parameter is automatically added with the error.
#[macro_export]
macro_rules! fail_notify {
    ($event:ident , $( $arg:expr ),* ; $result:expr ; $message:expr) => {
        match $result {
            Ok(v) => v,
            Err(e) => {
                use $crate::notifier::{notify, Event, State};
                use std::error::Error;

                match e.source() {
                    Some(s) => tracing::error!(error = %e, source = %s, $message),
                    None => tracing::error!(error = %e, $message),
                }

                notify(Event::$event( $( $arg ),*, State::Failure(e.to_string()) )).await;
                return;
            }
        }
    };
}

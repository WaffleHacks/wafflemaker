use arc_swap::ArcSwap;
use once_cell::sync::Lazy;
use std::{
    path::Path,
    sync::{mpsc, Arc},
    thread::JoinHandle,
};
use tokio::sync::oneshot;
use tracing::instrument;

mod diff;
mod pull;
mod service;

pub use diff::{Action, DiffFile};
use service::{Method, Return};

type Result<T> = std::result::Result<T, git2::Error>;

static STATIC_INSTANCE: Lazy<ArcSwap<Repository>> =
    Lazy::new(|| ArcSwap::from_pointee(Repository::default()));

/// A high-level async wrapper around `git2::Repository`
#[derive(Clone)]
pub struct Repository(mpsc::SyncSender<(Method, oneshot::Sender<Return>)>);

impl Repository {
    /// Pull a reference from the given remote URL
    #[instrument(name = "pull_dispatch", skip(self))]
    pub async fn pull(&self, clone_url: String, refspec: String, latest: String) -> Result<()> {
        // Send command
        let (tx, rx) = oneshot::channel();
        self.0
            .send((Method::Pull(clone_url, refspec, latest), tx))
            .unwrap();

        // Get the result
        match rx.await.unwrap() {
            Return::Pull(r) => r,
            _ => unreachable!(),
        }
    }

    /// Calculate the diff between two commits
    #[instrument(name = "diff_dispatch", skip(self))]
    pub async fn diff(&self, before: String, after: String) -> Result<Vec<DiffFile>> {
        // Send command
        let (tx, rx) = oneshot::channel();
        self.0.send((Method::Diff(before, after), tx)).unwrap();

        // Get the result
        match rx.await.unwrap() {
            Return::Diff(r) => r,
            _ => unreachable!(),
        }
    }

    /// Signal the service to shutdown
    pub fn shutdown(&self) {
        // Notify of shutdown
        let (tx, _) = oneshot::channel();
        self.0.send((Method::Shutdown, tx)).unwrap();
    }
}

impl Default for Repository {
    fn default() -> Repository {
        let (channel, _) = service::spawn("./configuration");
        Repository(channel)
    }
}

/// Start and connect to the git service
pub fn initialize<P: AsRef<Path>>(path: P) -> JoinHandle<()> {
    let (channel, handle) = service::spawn(path);
    STATIC_INSTANCE.swap(Arc::from(Repository(channel)));
    handle
}

/// Retrieve an instance of the repository
pub fn instance() -> Arc<Repository> {
    STATIC_INSTANCE.load().clone()
}

use once_cell::sync::OnceCell;
use std::{
    path::Path,
    sync::{mpsc, Arc},
    thread::JoinHandle,
};
use tokio::sync::oneshot;
use tracing::instrument;

mod diff;
mod head;
mod pull;
mod service;

pub use diff::{Action, DiffFile};
use service::{Method, Return};

type Result<T> = std::result::Result<T, git2::Error>;

static INSTANCE: OnceCell<Arc<Repository>> = OnceCell::new();

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

    /// Get the current head of the repository
    #[instrument(skip(self))]
    pub async fn head(&self) -> Result<String> {
        // Send command
        let (tx, rx) = oneshot::channel();
        self.0.send((Method::Head, tx)).unwrap();

        // Get the result
        match rx.await.unwrap() {
            Return::Head(c) => c,
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

/// Start and connect to the git service
pub fn initialize<P: AsRef<Path>>(path: P) -> JoinHandle<()> {
    let (channel, handle) = service::spawn(path);
    INSTANCE.get_or_init(|| Arc::from(Repository(channel)));
    handle
}

/// Retrieve an instance of the repository
pub fn instance() -> Arc<Repository> {
    INSTANCE.get().unwrap().clone()
}

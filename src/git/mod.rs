use std::{path::Path, sync::mpsc, thread::JoinHandle};
use tokio::sync::oneshot;
use tracing::instrument;

mod diff;
mod pull;
mod service;

use diff::DiffFile;
use service::{Method, Return};

type Result<T> = std::result::Result<T, git2::Error>;

/// A high-level async wrapper around `git2::Repository`
#[derive(Clone)]
pub struct Repository(mpsc::SyncSender<(Method, oneshot::Sender<Return>)>);

impl Repository {
    /// Start and connect to the git service
    pub fn connect<P: AsRef<Path>>(path: P) -> (Self, JoinHandle<()>) {
        let (channel, handle) = service::spawn(path);
        (Self(channel), handle)
    }

    /// Pull a reference from the given remote URL
    #[instrument(name = "pull_dispatch", skip(self))]
    pub async fn pull(&self, clone_url: String, refspec: String) -> Result<()> {
        // Send command
        let (tx, rx) = oneshot::channel();
        self.0.send((Method::Pull(clone_url, refspec), tx)).unwrap();

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
    pub fn shutdown(self) {
        // Notify of shutdown
        let (tx, _) = oneshot::channel();
        self.0.send((Method::Shutdown, tx)).unwrap();
    }
}

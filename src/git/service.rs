use super::{
    diff::{self, DiffFile},
    pull, Result,
};
use git2::Repository;
use std::{
    path::Path,
    sync::mpsc,
    thread::{self, JoinHandle},
};
use tokio::sync::oneshot;
use tracing::{info, info_span};

/// Spawn the git service thread. Interacts with the local git repository
/// in a separate, blocking thread to ensure the webserver threads are
/// not slowed down.
pub fn spawn<P: AsRef<Path>>(
    path: P,
) -> (
    mpsc::Sender<(Method, oneshot::Sender<Return>)>,
    JoinHandle<()>,
) {
    let path = path.as_ref().to_path_buf();

    // Create the method calling channels
    let (tx, rx) = mpsc::channel();

    // Handle the calls
    let handle = thread::spawn(move || {
        let span = info_span!("git");
        let _ = span.enter();

        // Initialize the repository
        let repository = Repository::init(path).unwrap();

        // Continuously wait for commands
        loop {
            match rx.recv() {
                Ok((Method::Shutdown, _)) => {
                    info!(parent: &span, "shutting down git service");
                    break;
                }
                Ok((method, tx)) => {
                    info!(parent: &span, method = method.name(), "new method call");
                    handle_call(&repository, method, tx);
                }
                Err(_) => break,
            };
        }
    });

    (tx, handle)
}

/// Run the necessary repository interaction
fn handle_call(repo: &Repository, method: Method, tx: oneshot::Sender<Return>) {
    match method {
        Method::Diff(before, after) => {
            let result = diff::run(&repo, &before, &after);
            tx.send(Return::Diff(result))
                .expect("failed to send on channel");
        }
        Method::Pull(clone_url, refspec) => {
            let result = pull::run(repo, &clone_url, &refspec);
            tx.send(Return::Pull(result))
                .expect("failed to send on channel");
        }
        _ => unreachable!(),
    }
}

/// The methods and their arguments that can be called
#[derive(Debug)]
pub enum Method {
    Pull(String, String),
    Diff(String, String),
    Shutdown,
}

impl Method {
    pub fn name(&self) -> &str {
        match self {
            Self::Pull(_, _) => "pull",
            Self::Diff(_, _) => "diff",
            Self::Shutdown => "shutdown",
        }
    }
}

/// The return types corresponding to each method
#[derive(Debug)]
pub enum Return {
    Pull(Result<()>),
    Diff(Result<Vec<DiffFile>>),
}
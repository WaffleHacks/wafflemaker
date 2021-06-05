use super::{Job, SharedJobQueue};
use crate::{
    fail,
    git::{Action, Repository},
};
use async_trait::async_trait;
use std::{path::PathBuf, sync::Arc};
use tracing::{info, instrument, warn};

#[derive(Debug)]
pub struct PlanUpdate {
    before: String,
    after: String,
}

impl PlanUpdate {
    /// Create a new plan update job
    pub fn new<S: Into<String>>(before: S, after: S) -> Self {
        Self {
            before: before.into(),
            after: after.into(),
        }
    }
}

#[async_trait]
impl Job for PlanUpdate {
    #[instrument(name = "plan_update", skip(self, queue, repo), fields(before = %self.before, after = %self.after))]
    async fn run(&self, path: Arc<PathBuf>, queue: SharedJobQueue, repo: &Repository) {
        // Diff the deployment
        let files = fail!(
            repo.diff(self.before.to_string(), self.after.to_string())
                .await
        );

        // Spawn jobs for the changed files
        for diff in files {
            if diff.binary {
                // Ignore binary files
                continue;
            }

            if matches!(diff.action, Action::Unknown) {
                // Ignore unknown operations
                warn!(
                    action = ?diff.action,
                    path = %diff.path.display(),
                    "unknown file delta",
                );
                continue;
            }

            // TODO: parse configuration

            match diff.action {
                Action::Modified => {
                    // TODO: spawn update job
                    info!(path = %diff.path.display(), "updating service")
                }
                Action::Deleted => {
                    // TODO: spawn delete job
                    info!(path = %diff.path.display(), "deleting service")
                }
                _ => unreachable!(),
            }
        }
    }

    fn name<'a>(&self) -> &'a str {
        "plan_update"
    }
}

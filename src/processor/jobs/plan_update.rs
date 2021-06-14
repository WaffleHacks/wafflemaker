use super::{DeleteService, Job, UpdateService};
use crate::{
    fail,
    git::{self, Action},
    service::Service,
};
use async_trait::async_trait;
use std::path::PathBuf;
use tracing::{error, info, instrument, warn};

#[derive(Debug)]
pub struct PlanUpdate {
    base_path: PathBuf,
    before: String,
    after: String,
}

impl PlanUpdate {
    /// Create a new plan update job
    pub fn new<S: Into<String>, P: Into<PathBuf>>(base_path: P, before: S, after: S) -> Self {
        Self {
            base_path: base_path.into(),
            before: before.into(),
            after: after.into(),
        }
    }
}

#[async_trait]
impl Job for PlanUpdate {
    #[instrument(
        skip(self),
        fields(before = %self.before, after = %self.after, name = %self.name())
    )]
    async fn run(&self) {
        // Diff the deployment
        let files = fail!(
            git::instance()
                .diff(self.before.to_string(), self.after.to_string())
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

            match diff.action {
                Action::Modified => {
                    // Parse the configuration
                    let config = match Service::parse(self.base_path.join(&diff.path)).await {
                        Ok(c) => c,
                        Err(e) => {
                            error!(
                                error = %e,
                                path = %diff.path.display(),
                                "failed to parse service configuration"
                            );
                            continue;
                        }
                    };

                    // Spawn update job
                    info!(path = %diff.path.display(), "updating service");
                    super::dispatch(UpdateService::new(config, diff.path));
                }
                Action::Deleted => {
                    // Spawn delete job
                    info!(path = %diff.path.display(), "deleting service");
                    super::dispatch(DeleteService::new(diff.path));
                }
                _ => unreachable!(),
            }
        }
    }

    fn name<'a>(&self) -> &'a str {
        "plan_update"
    }
}

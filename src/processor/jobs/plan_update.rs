use super::{DeleteService, Job, UpdateService};
use crate::{
    fail_notify,
    git::{self, Action},
    notifier::{self, Event, State},
    service::Service,
};
use async_trait::async_trait;
use std::{ffi::OsStr, path::PathBuf};
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

    /// Convert the full `before` commit hash to a shortened version
    fn short_before(&self) -> &str {
        &self.before[..8]
    }

    /// Convert the full `after` commit hash to the shortened version
    fn short_after(&self) -> &str {
        &self.after[..8]
    }
}

#[async_trait]
impl Job for PlanUpdate {
    #[instrument(
        skip(self),
        fields(before = %self.short_before(), after = %self.short_after(), name = %self.name())
    )]
    async fn run(&self) {
        macro_rules! fail {
            ($result:expr) => {
                fail_notify!(deployment, &self.after; $result; "an error occurred while planning deployment");
            };
        }

        notifier::notify(Event::deployment(&self.after, State::InProgress)).await;

        // Diff the deployment
        let files = fail!(
            git::instance()
                .diff(self.before.to_string(), self.after.to_string())
                .await
        );

        // Spawn jobs for the changed files
        let mut parse_failures = Vec::new();
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

            if diff.path.extension().map(OsStr::to_str).flatten() != Some("toml") {
                info!(path = %diff.path.display(), "skipping non-service file");
                continue;
            }

            let name = Service::name(diff.path.as_path());

            match diff.action {
                Action::Modified => {
                    // Parse the configuration
                    let config = match Service::parse(self.base_path.join(&diff.path)).await {
                        Ok(c) => c,
                        Err(e) => {
                            let displayable = diff.path.display();
                            parse_failures.push(displayable.to_string());
                            error!(
                                error = %e,
                                path = %displayable,
                                "failed to parse service configuration"
                            );
                            continue;
                        }
                    };

                    // Spawn update job
                    info!(path = %diff.path.display(), name = %name, "updating service");
                    super::dispatch(UpdateService::new(config, name));
                }
                Action::Deleted => {
                    // Spawn delete job
                    info!(path = %diff.path.display(), name = %name, "deleting service");
                    super::dispatch(DeleteService::new(name));
                }
                _ => unreachable!(),
            }
        }

        let state = if parse_failures.len() == 0 {
            State::Success
        } else {
            State::Failure(format!("unable to parse: {}", parse_failures.join(", ")))
        };
        notifier::notify(Event::deployment(&self.after, state)).await;
    }

    fn name<'a>(&self) -> &'a str {
        "plan_update"
    }
}

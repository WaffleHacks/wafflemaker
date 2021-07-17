use super::{
    errors::{BodyDeserializeError, GitError, UndeployableError},
    webhooks::{validators, Docker, Github},
};
use crate::{
    config, git,
    processor::jobs::{self, PlanUpdate},
};
use bytes::Bytes;
use tracing::info;
use warp::{http::StatusCode, reject, Rejection, Reply};

/// Handle webhooks from Docker image pushes
pub async fn docker(body: Docker, authorization: String) -> Result<impl Reply, Rejection> {
    let cfg = config::instance();
    validators::docker(authorization, &cfg.webhooks.docker)?;

    // TODO: check if image is allowed to be deployed

    // TODO: spawn container update job to update any containers using the specified image

    Ok(StatusCode::NO_CONTENT)
}

/// Handle webhooks from GitHub repository pushes
pub async fn github(raw_body: Bytes, raw_signature: String) -> Result<impl Reply, Rejection> {
    let cfg = config::instance();
    validators::github(&raw_body, raw_signature, cfg.webhooks.github.as_bytes())?;

    let body: Github =
        serde_json::from_slice(&raw_body).map_err(|_| reject::custom(BodyDeserializeError))?;
    info!("got new {} hook", body.name());

    let (before, after, reference, repository) = match body {
        Github::Ping { zen, hook_id } => {
            info!("received ping from hook {}: {}", hook_id, zen);
            return Ok(StatusCode::NO_CONTENT);
        }
        Github::Push {
            after,
            before,
            reference,
            repository,
        } => (before, after, reference, repository),
    };

    // Check if the repository is allowed to be pulled
    if repository.name != cfg.git.repository {
        return Err(reject::custom(UndeployableError));
    }

    // Pull the repository
    git::instance()
        .pull(repository.clone_url, reference, after.clone())
        .await
        .map_err(|e| reject::custom(GitError(e)))?;

    // Start the update
    jobs::dispatch(PlanUpdate::new(&cfg.git.clone_to, before, after));

    Ok(StatusCode::NO_CONTENT)
}

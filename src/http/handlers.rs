use super::{
    errors::{BodyDeserializeError, GitError, UndeployableError},
    webhooks::{validators, Docker, Github},
    SharedConfig,
};
use bytes::Bytes;
use tracing::info;
use warp::{http::StatusCode, reject, Rejection, Reply};

/// Handle webhooks from Docker image pushes
pub async fn docker(
    body: Docker,
    authorization: String,
    config: SharedConfig,
) -> Result<impl Reply, Rejection> {
    validators::docker(authorization, &config.docker.token)?;

    // TODO: check if image is allowed to be deployed

    // TODO: spawn container update job to update any containers using the specified image

    Ok(StatusCode::NO_CONTENT)
}

/// Handle webhooks from GitHub repository pushes
pub async fn github(
    raw_body: Bytes,
    raw_signature: String,
    config: SharedConfig,
) -> Result<impl Reply, Rejection> {
    validators::github(&raw_body, raw_signature, config.github.secret.as_bytes())?;

    let body: Github =
        serde_json::from_slice(&raw_body).map_err(|_| reject::custom(BodyDeserializeError))?;
    info!("got new {} hook", body.name());

    let (after, reference, repository) = match body {
        Github::Ping { zen, hook_id } => {
            info!("received ping from hook {}: {}", hook_id, zen);
            return Ok(StatusCode::NO_CONTENT);
        }
        Github::Push {
            after,
            reference,
            repository,
        } => (after, reference, repository),
    };

    // Check if the repository is allowed to be pulled
    if &repository.name != &config.github.repository {
        return Err(reject::custom(UndeployableError));
    }

    // TODO: spawn deployment job to diff repository and update necessary containers

    Ok(StatusCode::NO_CONTENT)
}
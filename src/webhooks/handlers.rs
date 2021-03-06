use super::{
    models::{Docker, Github},
    validators,
};
use crate::{
    config, git,
    http::{BodyDeserializeError, GitError, UndeployableError},
    processor::jobs::{self, PlanUpdate, UpdateService},
    service::registry,
};
use bytes::Bytes;
use tracing::{error, info};
use warp::{http::StatusCode, reject, Rejection, Reply};

/// Handle webhooks from Docker image pushes
pub async fn docker(body: Docker, authorization: String) -> Result<impl Reply, Rejection> {
    let cfg = config::instance();
    validators::docker(authorization, &cfg.webhooks.docker)?;

    info!(image = %body.repository.repo_name, tag = %body.push_data.tag, "got new image update hook");

    sentry::configure_scope(|scope| {
        scope.set_tag("hook.repository", &body.repository.repo_name);
        scope.set_tag("hook.tag", &body.push_data.tag);
    });

    let reg = registry::REGISTRY.read().await;
    for (name, service) in reg.iter() {
        // Skip if the image does not match or automatic updates are off
        if service.docker.image != body.repository.repo_name || !service.docker.update.automatic {
            continue;
        }

        // Check if tag is allowed
        let tags = match service.docker.allowed_tags() {
            Ok(t) => t,
            Err(e) => {
                error!(error = %e, "failed to compile tag glob");
                continue;
            }
        };
        if !tags.is_match(&body.push_data.tag) {
            continue;
        }

        let mut updated = service.clone();
        updated.docker.tag = body.push_data.tag.clone();

        jobs::dispatch(UpdateService::new(updated, name.into()));
        info!("updating service \"{}\"", name);
    }

    Ok(StatusCode::NO_CONTENT)
}

/// Handle webhooks from GitHub repository pushes
pub async fn github(raw_body: Bytes, raw_signature: String) -> Result<impl Reply, Rejection> {
    let cfg = config::instance();
    validators::github(&raw_body, raw_signature, cfg.webhooks.github.as_bytes())?;

    let body: Github =
        serde_json::from_slice(&raw_body).map_err(|_| reject::custom(BodyDeserializeError))?;
    info!("got new {} hook", body.name());

    sentry::configure_scope(|scope| {
        scope.set_tag("hook.type", body.name());
    });

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

    sentry::configure_scope(|scope| {
        scope.set_tag("hook.repository", &repository.name);
        scope.set_tag("hook.after", &after);
        scope.set_tag("hook.before", &before);
        scope.set_tag("hook.reference", &reference);
    });

    // Check if the repository is allowed to be pulled
    if repository.name != cfg.git.repository || !reference.ends_with(&cfg.git.branch) {
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

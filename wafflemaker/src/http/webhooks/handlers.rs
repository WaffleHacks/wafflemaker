use super::{
    models::{Docker, Github, Repository},
    validators, Error, Result,
};
use crate::{
    git,
    processor::jobs::{self, PlanUpdate, UpdateService},
    service::registry,
    Config,
};
use axum::{
    body::Bytes,
    extract::TypedHeader,
    headers::{authorization::Basic, Authorization, HeaderMap},
    http::StatusCode,
    Extension, Json,
};
use std::sync::Arc;
use tracing::{error, info};

/// Handle webhooks from Docker image pushes
pub async fn docker(
    Json(body): Json<Docker>,
    TypedHeader(authorization): TypedHeader<Authorization<Basic>>,
    Extension(config): Extension<Arc<Config>>,
) -> Result<StatusCode> {
    validators::docker(authorization, &config.http.webhooks.docker)?;

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
pub async fn github(
    raw_body: Bytes,
    headers: HeaderMap,
    Extension(config): Extension<Arc<Config>>,
) -> Result<StatusCode> {
    validators::github(
        &raw_body,
        headers.get("X-Hub-Signature-256"),
        config.http.webhooks.github.as_bytes(),
    )?;

    let body: Github = serde_json::from_slice(&raw_body)?;
    info!("got new {} hook", body.name());

    sentry::configure_scope(|scope| {
        scope.set_tag("hook.type", body.name());
    });

    match body {
        Github::Ping { zen, hook_id } => Ok(github_ping_event(zen, hook_id)),
        Github::Push {
            after,
            before,
            reference,
            repository,
        } => github_push_event(after, before, reference, repository, config).await,
    }
}

/// Handle a GitHub ping event
fn github_ping_event(zen: String, hook_id: i64) -> StatusCode {
    info!(%zen, %hook_id, "received ping");
    StatusCode::NO_CONTENT
}

/// Handle a GitHub push event
async fn github_push_event(
    after: String,
    before: String,
    reference: String,
    repository: Repository,
    config: Arc<Config>,
) -> Result<StatusCode> {
    sentry::configure_scope(|scope| {
        scope.set_tag("hook.repository", &repository.name);
        scope.set_tag("hook.after", &after);
        scope.set_tag("hook.before", &before);
        scope.set_tag("hook.reference", &reference);
    });

    // Check if the repository is allowed to be pulled
    if repository.name != config.git.repository || !reference.ends_with(&config.git.branch) {
        return Err(Error::DisallowedRepository);
    }

    // Pull the repository
    git::instance()
        .pull(repository.clone_url, reference, after.clone())
        .await?;

    // Start the update
    jobs::dispatch(PlanUpdate::new(before, after));

    Ok(StatusCode::NO_CONTENT)
}

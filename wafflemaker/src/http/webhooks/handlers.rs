use super::{
    models::{Docker, Github, Repository},
    validators,
};
use crate::{
    config, git,
    processor::jobs::{self, PlanUpdate, UpdateService},
    service::registry,
};
use axum::{
    body::Bytes,
    extract::TypedHeader,
    headers::{authorization::Basic, Authorization, HeaderMap},
    http::StatusCode,
    Json,
};
use tracing::{error, info};

/// Handle webhooks from Docker image pushes
pub async fn docker(
    Json(body): Json<Docker>,
    TypedHeader(authorization): TypedHeader<Authorization<Basic>>,
) -> Result<StatusCode, StatusCode> {
    let cfg = config::instance();
    validators::docker(authorization, &cfg.http.webhooks.docker)?;

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
pub async fn github(raw_body: Bytes, headers: HeaderMap) -> Result<StatusCode, StatusCode> {
    let cfg = config::instance();
    validators::github(
        &raw_body,
        headers.get("X-Hub-Signature-256"),
        cfg.http.webhooks.github.as_bytes(),
    )?;

    let body: Github = serde_json::from_slice(&raw_body).map_err(|_| StatusCode::BAD_REQUEST)?;
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
        } => github_push_event(after, before, reference, repository).await,
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
) -> Result<StatusCode, StatusCode> {
    let cfg = config::instance();
    sentry::configure_scope(|scope| {
        scope.set_tag("hook.repository", &repository.name);
        scope.set_tag("hook.after", &after);
        scope.set_tag("hook.before", &before);
        scope.set_tag("hook.reference", &reference);
    });

    // Check if the repository is allowed to be pulled
    if repository.name != cfg.git.repository || !reference.ends_with(&cfg.git.branch) {
        return Err(StatusCode::FORBIDDEN);
    }

    // Pull the repository
    if let Err(e) = git::instance()
        .pull(repository.clone_url, reference, after.clone())
        .await
    {
        error!(
            "error while interacting with local repo: ({:?}, {:?}) {}",
            e.class(),
            e.code(),
            e.message()
        );
        return Err(StatusCode::INTERNAL_SERVER_ERROR);
    }

    // Start the update
    jobs::dispatch(PlanUpdate::new(before, after));

    Ok(StatusCode::NO_CONTENT)
}

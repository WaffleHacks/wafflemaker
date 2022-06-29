use crate::{
    config, deployer, git,
    processor::jobs::{self, PlanUpdate},
    service::registry::REGISTRY,
};
use axum::{
    extract::Path,
    routing::{get, post},
    Json, Router,
};
use serde::Serialize;
use warp::http::StatusCode;

/// Build the routes for deployments
pub fn routes() -> Router {
    Router::new()
        .route("/", get(info))
        .route("/:before", post(rerun))
}

#[derive(Debug, Serialize)]
struct Response {
    commit: String,
    services: usize,
    running: usize,
}

/// Get the most recently deployed version, number of running deployments,
/// and number of known services
async fn info() -> Result<Json<Response>, StatusCode> {
    let running = deployer::instance()
        .list()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?
        .len();
    let services = {
        let reg = REGISTRY.read().await;
        reg.len()
    };
    let commit = git::instance()
        .head()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(Response {
        commit,
        services,
        running,
    }))
}

/// Re-run a deployment given the commit hash of the before state
async fn rerun(Path(before): Path<String>) -> Result<StatusCode, StatusCode> {
    let current = git::instance()
        .head()
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    let path = &config::instance().git.clone_to;
    jobs::dispatch(PlanUpdate::new(path, before, current));

    Ok(StatusCode::NO_CONTENT)
}

use super::Result;
use crate::{
    deployer, git,
    processor::jobs::{self, PlanUpdate},
    service::registry::REGISTRY,
};
use axum::{
    extract::Path,
    http::StatusCode,
    routing::{get, post},
    Json, Router,
};
use serde::Serialize;

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
async fn info() -> Result<Json<Response>> {
    let running = deployer::instance().list().await?.len();
    let services = {
        let reg = REGISTRY.read().await;
        reg.len()
    };
    let commit = git::instance().head().await?;

    Ok(Json(Response {
        commit,
        services,
        running,
    }))
}

/// Re-run a deployment given the commit hash of the before state
async fn rerun(Path(before): Path<String>) -> Result<StatusCode> {
    let current = git::instance().head().await?;

    jobs::dispatch(PlanUpdate::new(before, current));

    Ok(StatusCode::NO_CONTENT)
}

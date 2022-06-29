use crate::{
    deployer,
    processor::jobs::{self, DeleteService, UpdateService},
    registry::REGISTRY,
};
use axum::{
    extract::Path,
    http::StatusCode,
    response::{IntoResponse, Response as HttpResponse},
    routing::get,
    Json, Router,
};
use serde::Serialize;

/// Build the routes for services
pub fn routes() -> Router {
    Router::new().route("/*service", get(root_dispatch).put(redeploy).delete(delete))
}

/// Overcomes a limitation of axum where you cannot have a route with a bare / along with a route
/// consuming the entire path after a / on the same router.
async fn root_dispatch(Path(path): Path<String>) -> Result<HttpResponse, StatusCode> {
    // The will always be a leading /
    let service = path.strip_prefix("/").unwrap().to_owned();

    if service.is_empty() {
        Ok(list().await.into_response())
    } else {
        read(service).await.map(IntoResponse::into_response)
    }
}

/// Get a list of all the currently deployed services
async fn list() -> Json<Vec<String>> {
    let reg = REGISTRY.read().await;

    Json(reg.keys().map(String::to_owned).collect())
}

#[derive(Debug, Serialize)]
struct Response {
    dependencies: Vec<String>,
    image: String,
    automatic_updates: bool,
    domain: Option<String>,
    deployment_id: Option<String>,
}

/// Get the configuration for a service
async fn read(service: String) -> Result<Json<Response>, StatusCode> {
    let service = service.as_str();

    let reg = REGISTRY.read().await;
    let cfg = reg.get(service).ok_or_else(|| StatusCode::NOT_FOUND)?;

    let deployment_id = deployer::instance()
        .service_id(service)
        .await
        .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR)?;

    Ok(Json(Response {
        dependencies: cfg.dependencies.all(),
        image: format!("{}:{}", cfg.docker.image, cfg.docker.tag),
        automatic_updates: cfg.docker.update.automatic,
        domain: cfg.web.domain.clone(),
        deployment_id,
    }))
}

/// Re-deploy a service
async fn redeploy(Path(service): Path<String>) -> Result<StatusCode, StatusCode> {
    let service = service.strip_prefix("/").unwrap();

    let reg = REGISTRY.read().await;
    let config = reg.get(service).ok_or_else(|| StatusCode::NOT_FOUND)?;

    jobs::dispatch(UpdateService::new(config.clone(), service.into()));

    Ok(StatusCode::NO_CONTENT)
}

/// Delete a service
async fn delete(Path(service): Path<String>) -> StatusCode {
    let service = service.strip_prefix("/").unwrap();
    jobs::dispatch(DeleteService::new(service.into()));

    StatusCode::NO_CONTENT
}

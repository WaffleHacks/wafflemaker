use super::{Error, Result};
use axum::{
    http::StatusCode,
    routing::{get, post},
    Router,
};

mod handlers;
mod models;
mod validators;

/// Build the routes for the webhook receivers
pub fn routes() -> Router {
    Router::new()
        .route("/docker", post(handlers::docker))
        .route("/github", post(handlers::github))
        .route("/health", get(health))
}

async fn health() -> StatusCode {
    StatusCode::NO_CONTENT
}

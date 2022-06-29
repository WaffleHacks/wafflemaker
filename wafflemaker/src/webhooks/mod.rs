use crate::http;
use axum::{
    http::StatusCode,
    routing::{get, post},
    Router,
};

mod handlers;
mod models;
mod validators;

/// Build the routes for the API
pub fn routes() -> Router {
    Router::new()
        .route("/docker", post(handlers::docker))
        .route("/github", post(handlers::github))
        .route("/health", get(health))
        .layer(http::logging())
}

async fn health() -> StatusCode {
    StatusCode::NO_CONTENT
}

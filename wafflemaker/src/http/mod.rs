use crate::Config;
use axum::{Extension, Router};
use std::sync::Arc;

mod error;
mod logging;
mod management;
mod webhooks;

use error::{Error, Result};

/// Build all the routes for the service
pub fn routes(config: Arc<Config>) -> Router {
    Router::new()
        .merge(management::routes(config.http.management_token.clone()))
        .merge(webhooks::routes())
        .layer(Extension(config))
        .layer(logging::layer())
}

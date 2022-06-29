use crate::config;
use axum::Router;

mod logging;
mod management;
mod webhooks;

/// Build all the routes for the service
pub fn routes() -> Router {
    let cfg = config::instance();
    Router::new()
        .merge(management::routes(cfg.agent.management_token.clone()))
        .merge(webhooks::routes())
        .layer(logging::layer())
}

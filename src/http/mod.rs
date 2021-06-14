use crate::config::SharedConfig;
use std::convert::Infallible;
use tracing::info;
use warp::{http::StatusCode, Filter, Rejection, Reply};

mod errors;
mod handlers;
mod webhooks;

pub use errors::recover;

/// Allow a single instance of the config to be shared between
/// any number of handlers
fn with_config(
    config: SharedConfig,
) -> impl Filter<Extract = (SharedConfig,), Error = Infallible> + Clone {
    warp::any().map(move || config.clone())
}

/// Build the routes for the API
pub fn routes(
    config: SharedConfig,
) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    // Docker webhook route
    let docker = warp::path("docker")
        .and(warp::post())
        .and(warp::body::content_length_limit(1024 * 64))
        .and(warp::body::json())
        .and(warp::header::<String>("Authorization"))
        .and(with_config(config.clone()))
        .and_then(handlers::docker)
        .with(warp::trace::named("docker"));

    // Github webhook route
    let github = warp::path("github")
        .and(warp::post())
        .and(warp::body::content_length_limit(1024 * 64))
        .and(warp::body::bytes())
        .and(warp::header::<String>("X-Hub-Signature-256"))
        .and(with_config(config))
        .and_then(handlers::github)
        .with(warp::trace::named("docker"));

    // Health check route
    let health = warp::path("health")
        .and(warp::get())
        .map(|| {
            info!("alive and healthy!");
            StatusCode::NO_CONTENT
        })
        .with(warp::trace::named("health"));

    docker.or(github).or(health)
}

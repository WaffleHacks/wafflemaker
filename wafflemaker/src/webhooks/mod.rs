use crate::http::named_trace;
use warp::{http::StatusCode, Filter, Rejection, Reply};

mod handlers;
mod models;
mod validators;

/// Build the routes for the API
pub fn routes() -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    // Docker webhook route
    let docker = warp::path("docker")
        .and(warp::post())
        .and(warp::body::content_length_limit(1024 * 64))
        .and(warp::body::json())
        .and(warp::header::<String>("Authorization"))
        .and_then(handlers::docker)
        .with(named_trace("docker"));

    // Github webhook route
    let github = warp::path("github")
        .and(warp::post())
        .and(warp::body::content_length_limit(1024 * 64))
        .and(warp::body::bytes())
        .and(warp::header::<String>("X-Hub-Signature-256"))
        .and_then(handlers::github)
        .with(named_trace("docker"));

    // Health check route
    let health = warp::path("health")
        .and(warp::get())
        .map(|| StatusCode::NO_CONTENT);

    health.or(docker).or(github)
}

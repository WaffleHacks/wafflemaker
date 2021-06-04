use crate::{config::Config, git::Repository, processor::jobs::SharedJobQueue};
use std::{convert::Infallible, sync::Arc};
use tracing::info;
use warp::{http::StatusCode, Filter, Rejection, Reply};

mod errors;
mod handlers;
mod webhooks;

pub use errors::recover;

type SharedConfig = Arc<Config>;

/// Allow a single instance of the config to be shared between
/// any number of handlers
fn with_config(
    config: SharedConfig,
) -> impl Filter<Extract = (SharedConfig,), Error = Infallible> + Clone {
    warp::any().map(move || config.clone())
}

/// Allow the repository to be cloned between handlers
fn with_repository(
    repo: Repository,
) -> impl Filter<Extract = (Repository,), Error = Infallible> + Clone {
    warp::any().map(move || repo.clone())
}

/// Allow job queue to be cloned between handlers
fn with_queue(
    queue: SharedJobQueue,
) -> impl Filter<Extract = (SharedJobQueue,), Error = Infallible> + Clone {
    warp::any().map(move || queue.clone())
}

/// Build the routes for the API
pub fn routes(
    config: Config,
    repo: Repository,
    queue: SharedJobQueue,
) -> impl Filter<Extract = impl Reply, Error = Rejection> + Clone {
    let shared_config = Arc::new(config);

    // Docker webhook route
    let docker = warp::path("docker")
        .and(warp::post())
        .and(warp::body::content_length_limit(1024 * 64))
        .and(warp::body::json())
        .and(warp::header::<String>("Authorization"))
        .and(with_config(shared_config.clone()))
        .and(with_queue(queue.clone()))
        .and_then(handlers::docker)
        .with(warp::trace::named("docker"));

    // Github webhook route
    let github = warp::path("github")
        .and(warp::post())
        .and(warp::body::content_length_limit(1024 * 64))
        .and(warp::body::bytes())
        .and(warp::header::<String>("X-Hub-Signature-256"))
        .and(with_config(shared_config))
        .and(with_repository(repo))
        .and(with_queue(queue))
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

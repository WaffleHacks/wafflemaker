use crate::{
    config, deployer, git,
    http::{named_trace, GitError},
    processor::jobs::{self, PlanUpdate},
    service::registry::REGISTRY,
};
use serde::Serialize;
use warp::http::StatusCode;
use warp::{Filter, Rejection, Reply};

/// Build the routes for deployments
pub fn routes() -> impl Filter<Extract = (impl Reply,), Error = Rejection> + Clone {
    let get = warp::get()
        .and(warp::path::end())
        .and_then(get)
        .with(named_trace("get"));

    let post = warp::put()
        .and(warp::path::param())
        .and(warp::path::end())
        .and_then(rerun)
        .with(named_trace("rerun"));

    warp::path("deployments").and(get.or(post))
}

#[derive(Debug, Serialize)]
struct Response<'c> {
    commit: &'c str,
    services: usize,
    running: usize,
}

/// Get the most recently deployed version, number of running deployments,
/// and number of known services
async fn get() -> Result<impl Reply, Rejection> {
    let running = deployer::instance().list().await?.len();
    let services = {
        let reg = REGISTRY.read().await;
        reg.len()
    };
    let commit = git::instance().head().await.map_err(GitError)?;

    Ok(warp::reply::json(&Response {
        commit: &commit,
        services,
        running,
    }))
}

/// Re-run a deployment given the commit hash of the before state
async fn rerun(before: String) -> Result<impl Reply, Rejection> {
    let current = git::instance().head().await.map_err(GitError)?;

    let path = &config::instance().git.clone_to;
    jobs::dispatch(PlanUpdate::new(path, before, current));

    Ok(StatusCode::NO_CONTENT)
}

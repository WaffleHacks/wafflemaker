use crate::{
    deployer,
    http::named_trace,
    processor::jobs::{self, DeleteService, UpdateService},
    registry::REGISTRY,
    service::Service,
};
use serde::Serialize;
use warp::{http::StatusCode, Filter, Rejection, Reply};

/// Build the routes for services
pub fn routes() -> impl Filter<Extract = (impl Reply,), Error = Rejection> + Clone {
    let list = warp::get()
        .and(warp::path::end())
        .and_then(list)
        .with(named_trace("list"));

    let get = warp::get()
        .and(warp::path::param())
        .and(warp::path::end())
        .and_then(get)
        .with(named_trace("get"));
    let redeploy = warp::put()
        .and(warp::path::param())
        .and(warp::path::end())
        .and_then(redeploy)
        .with(named_trace("redeploy"));
    let delete = warp::delete()
        .and(warp::path::param())
        .and(warp::path::end())
        .and_then(delete)
        .with(named_trace("delete"));

    warp::path("services").and(list.or(get).or(redeploy).or(delete))
}

/// Get a list of all the currently deployed services
async fn list() -> Result<impl Reply, Rejection> {
    let reg = REGISTRY.read().await;
    let services = reg.keys().map(String::as_str).collect::<Vec<&str>>();

    Ok(warp::reply::json(&services))
}

#[derive(Debug, Serialize)]
struct Response<'c> {
    config: &'c Service,
    deployment_id: Option<String>,
}

/// Get the configuration for a service
async fn get(service: String) -> Result<impl Reply, Rejection> {
    let reg = REGISTRY.read().await;
    let config = reg.get(&service).ok_or(warp::reject::not_found())?;

    let deployment_id = deployer::instance().service_id(&service).await?;

    Ok(warp::reply::json(&Response {
        config,
        deployment_id,
    }))
}

/// Re-deploy a service
async fn redeploy(service: String) -> Result<impl Reply, Rejection> {
    let reg = REGISTRY.read().await;
    let config = reg.get(&service).ok_or(warp::reject::not_found())?;

    jobs::dispatch(UpdateService::new(config.clone(), service));

    Ok(StatusCode::NO_CONTENT)
}

/// Delete a service
async fn delete(service: String) -> Result<impl Reply, Rejection> {
    jobs::dispatch(DeleteService::new(service));
    Ok(StatusCode::NO_CONTENT)
}

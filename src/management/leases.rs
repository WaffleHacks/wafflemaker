use crate::{
    deployer,
    http::named_trace,
    vault::{Lease, LEASES},
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
use warp::{http::StatusCode, Filter, Rejection, Reply};

pub fn routes() -> impl Filter<Extract = (impl Reply,), Error = Rejection> + Clone {
    let list = warp::get().and_then(list).with(named_trace("list"));

    let update = warp::put()
        .and(warp::body::content_length_limit(1024 * 8))
        .and(warp::body::json())
        .and_then(add)
        .with(named_trace("put"));

    let delete = warp::delete()
        .and(warp::query())
        .and_then(delete)
        .with(named_trace("delete"));

    warp::path("leases").and(list.or(update).or(delete))
}

#[derive(Debug, Serialize)]
struct LeaseResponse<'l> {
    lease_id: &'l str,
    lease_duration: u64,
    updated_at: u64,
}

impl<'l> From<&'l Lease> for LeaseResponse<'l> {
    fn from(lease: &'l Lease) -> LeaseResponse<'l> {
        LeaseResponse {
            lease_id: &lease.lease_id,
            lease_duration: lease.lease_duration,
            updated_at: lease.updated_at,
        }
    }
}

#[derive(Debug, Serialize)]
struct Response<'l> {
    leases: HashMap<&'l str, Vec<LeaseResponse<'l>>>,
    services: HashMap<String, String>,
}

/// Get all the currently registered leases
async fn list() -> Result<impl Reply, Rejection> {
    let leases = LEASES.read().await;

    let mut response = HashMap::new();
    for (id, lease_set) in leases.iter() {
        response.insert(
            id.as_str(),
            lease_set
                .iter()
                .map(LeaseResponse::from)
                .collect::<Vec<_>>(),
        );
    }

    let services = deployer::instance().list().await?;

    Ok(warp::reply::json(&Response {
        leases: response,
        services,
    }))
}

#[derive(Debug, Deserialize)]
struct Add {
    service: String,
    lease: Lease,
}

/// Add a lease to track for a service
async fn add(body: Add) -> Result<impl Reply, Rejection> {
    let mut leases = LEASES.write().await;

    if let Some(id) = deployer::instance().service_id(&body.service).await? {
        leases.entry(id).or_default().push(body.lease);
    }

    Ok(StatusCode::NO_CONTENT)
}

#[derive(Debug, Deserialize)]
struct Delete {
    id: String,
    service: String,
}

/// Remove a lease from tracking for a service
async fn delete(params: Delete) -> Result<impl Reply, Rejection> {
    let mut leases = LEASES.write().await;

    if let Some(id) = deployer::instance().service_id(&params.service).await? {
        let mut empty = false;

        // Remove any entries matching the id
        if let Some(lease_set) = leases.get_mut(&id) {
            lease_set.retain(|l| l.lease_id != params.id);
            empty = lease_set.is_empty();
        }

        // Remove the entire entry if its empty
        if empty {
            leases.remove(&id);
        }
    }

    Ok(StatusCode::NO_CONTENT)
}
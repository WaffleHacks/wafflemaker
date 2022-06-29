use super::Result;
use crate::{
    deployer,
    vault::{Lease, LEASES},
};
use axum::{
    extract::{Path, Query},
    http::StatusCode,
    routing::{get, put},
    Json, Router,
};
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub fn routes() -> Router {
    Router::new()
        .route("/", get(list))
        .route("/:id", put(add).delete(delete))
}

#[derive(Debug, Deserialize, Serialize)]
struct HttpLease {
    id: String,
    ttl: u64,
    updated_at: u64,
}

impl From<&Lease> for HttpLease {
    fn from(lease: &Lease) -> HttpLease {
        HttpLease {
            id: lease.id.clone(),
            ttl: lease.ttl,
            updated_at: lease.updated_at,
        }
    }
}

impl HttpLease {
    fn into_lease(self) -> Lease {
        Lease {
            id: self.id,
            ttl: self.ttl,
            updated_at: self.updated_at,
        }
    }
}

#[derive(Debug, Serialize)]
struct Response {
    leases: HashMap<String, Vec<HttpLease>>,
    services: HashMap<String, String>,
}

/// Get all the currently registered leases
async fn list() -> Result<Json<Response>> {
    let leases = LEASES.read().await;
    let leases = leases
        .iter()
        .map(|(id, lease_set)| {
            (
                id.to_owned(),
                lease_set.iter().map(HttpLease::from).collect(),
            )
        })
        .collect();

    let services = deployer::instance().list().await?;

    Ok(Json(Response { leases, services }))
}

/// Add a lease to track for a service
async fn add(Path(service): Path<String>, Json(body): Json<HttpLease>) -> Result<StatusCode> {
    let mut leases = LEASES.write().await;

    if let Some(id) = deployer::instance().service_id(&service).await? {
        leases.entry(id).or_default().push(body.into_lease());
    }

    Ok(StatusCode::NO_CONTENT)
}

#[derive(Debug, Deserialize)]
struct Delete {
    id: String,
}

/// Remove a lease from tracking for a service
async fn delete(Path(service): Path<String>, params: Query<Delete>) -> Result<StatusCode> {
    let mut leases = LEASES.write().await;

    if let Some(id) = deployer::instance().service_id(&service).await? {
        let mut empty = false;

        // Remove any entries matching the id
        if let Some(lease_set) = leases.get_mut(&id) {
            lease_set.retain(|l| l.id != params.id);
            empty = lease_set.is_empty();
        }

        // Remove the entire entry if its empty
        if empty {
            leases.remove(&id);
        }
    }

    Ok(StatusCode::NO_CONTENT)
}

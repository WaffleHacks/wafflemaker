use tracing::{info, Span};
use warp::{
    http::StatusCode,
    trace::{trace, Info, Trace},
    Filter, Rejection, Reply,
};

mod errors;
mod handlers;
mod webhooks;

pub use errors::recover;

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
        .map(|| {
            info!("alive and healthy!");
            StatusCode::NO_CONTENT
        })
        .with(named_trace("health"));

    docker.or(github).or(health)
}

/// Wrap the request with some information allowing it
/// to be traced through the logs. Built off of the
/// `warp::trace::request` implementation
fn named_trace(name: &'static str) -> Trace<impl Fn(Info) -> Span + Clone> {
    use tracing::field::{display, Empty};

    trace(move |info: Info| {
        let span = tracing::info_span!(
            "request",
            %name,
            remote.addr = Empty,
            method = %info.method(),
            path = %info.path(),
            version = ?info.version(),
            referrer = Empty,
            id = %uuid::Uuid::new_v4(),
        );

        // Record optional fields
        if let Some(remote_addr) = info.remote_addr() {
            span.record("remote.addr", &display(remote_addr));
        }
        if let Some(referrer) = info.referer() {
            span.record("referrer", &display(referrer));
        }

        tracing::debug!(parent: &span, "received request");

        span
    })
}

use tracing::Span;
use warp::trace::{trace, Info, Trace};

mod errors;

pub use errors::*;

/// Wrap the request with some information allowing it
/// to be traced through the logs. Built off of the
/// `warp::trace::request` implementation
pub fn named_trace(name: &'static str) -> Trace<impl Fn(Info) -> Span + Clone> {
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

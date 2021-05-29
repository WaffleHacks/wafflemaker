use anyhow::{Context, Result};
use structopt::StructOpt;
use tokio::fs;
use tracing::Span;
use tracing_subscriber::fmt::format::FmtSpan;
use warp::{
    trace::{trace, Info, Trace},
    Filter,
};

mod args;
mod config;
mod http;

use args::Args;

#[tokio::main]
async fn main() -> Result<()> {
    // Parse the cli
    let cli = Args::from_args();

    // Get the configuration
    let configuration = config::parse(cli.config)
        .await
        .context("Failed to load configuration")?;
    let address = cli.address.unwrap_or(configuration.server.address);
    let log_filter = cli
        .log_level
        .unwrap_or_else(|| configuration.server.log.clone());

    // Ensure the clone directory exists
    if !configuration.github.clone_to.exists() {
        fs::create_dir_all(&configuration.github.clone_to)
            .await
            .context("Failed to create configuration clone directory")?;
    }

    // Setup logging
    tracing_subscriber::fmt()
        .with_env_filter(log_filter)
        .with_span_events(FmtSpan::CLOSE)
        .init();

    // Setup the routes and launch the server
    let routes = http::routes().with(trace_request());
    warp::serve(routes).run(address).await;

    Ok(())
}

/// Wrap the request with some information allowing it
/// to be traced through the logs. Built off of the
/// `warp::trace::request` implementation
fn trace_request() -> Trace<impl Fn(Info) -> Span + Clone> {
    use tracing::field::{display, Empty};

    trace(|info: Info| {
        let span = tracing::info_span!(
            "request",
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

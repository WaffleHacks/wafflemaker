use anyhow::{Context, Result};
use structopt::StructOpt;
use tokio::{
    fs,
    signal::unix::{signal, SignalKind},
    sync::broadcast,
    task,
};
use tracing::{info, Span};
use tracing_subscriber::fmt::format::FmtSpan;
use warp::{
    trace::{trace, Info, Trace},
    Filter,
};

use args::Args;

mod args;
mod config;
mod deployer;
mod git;
mod http;
mod notifier;
mod processor;
mod service;
mod vault;

use service::registry;

#[tokio::main]
async fn main() -> Result<()> {
    // Parse the cli
    let cli = Args::from_args();

    // Get the configuration
    config::parse(cli.config)
        .await
        .context("Failed to load configuration")?;
    let configuration = config::instance();
    let address = cli.address.unwrap_or(configuration.agent.address);
    let log_filter = cli
        .log_level
        .unwrap_or_else(|| configuration.agent.log.clone());

    // Ensure the clone directory exists
    if !configuration.git.clone_to.exists() {
        fs::create_dir_all(&configuration.git.clone_to)
            .await
            .context("Failed to create configuration clone directory")?;
    }

    // Setup logging
    tracing_subscriber::fmt()
        .with_env_filter(log_filter)
        .with_span_events(FmtSpan::CLOSE)
        .init();

    let (stop_tx, mut stop_rx) = broadcast::channel(1);

    // Initialize the service registry
    registry::init().await?;

    // Connect to the repository service
    let repository_handle = git::initialize(&configuration.git.clone_to);

    // Connect to the deployment service
    deployer::initialize(&configuration.deployment, stop_tx.subscribe()).await?;

    // Connect to Vault (secrets service)
    vault::initialize(&configuration.secrets, stop_tx.clone()).await?;

    // Setup the notifier service
    notifier::initialize()?;

    // Start the job processor
    processor::spawn(stop_tx.clone());

    // Setup the routes
    let routes = http::routes().recover(http::recover).with(trace_request());

    // Bind the server
    let (addr, server) = warp::serve(routes)
        .try_bind_with_graceful_shutdown(address, async move {
            stop_rx.recv().await.ok();
        })
        .with_context(|| format!("failed to bind to {}", address))?;

    // Start the server
    task::spawn(server);
    info!("listening on {}", addr);

    // Wait for shutdown
    wait_for_exit()
        .await
        .context("failed to listen for event")?;
    info!("signal received, shutting down...");

    // Shutdown the services
    stop_tx.send(()).unwrap();

    // Shutdown the repository service
    git::instance().shutdown();
    repository_handle.join().unwrap();

    info!("successfully shutdown, good bye!");
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

/// Wait for a SIGINT or SIGTERM and then exit
async fn wait_for_exit() -> Result<()> {
    let mut int = signal(SignalKind::interrupt())?;
    let mut term = signal(SignalKind::terminate())?;

    tokio::select! {
        _ = int.recv() => return Ok(()),
        _ = term.recv() => return Ok(()),
    }
}

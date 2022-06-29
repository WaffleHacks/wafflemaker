use anyhow::{Context, Result};
use axum::Server;
use console_subscriber::ConsoleLayer;
use sentry::{
    integrations::{anyhow::capture_anyhow, tracing as sentry_tracing},
    ClientOptions, IntoDsn,
};
use std::{net::SocketAddr, sync::Arc};
use structopt::StructOpt;
use tokio::{
    fs,
    signal::unix::{signal, SignalKind},
    sync::broadcast,
};
use tracing::info;
use tracing_subscriber::{
    fmt::format::FmtSpan, layer::SubscriberExt, util::SubscriberInitExt, EnvFilter, Layer,
};

use args::Args;

mod args;
mod config;
mod deployer;
mod dns;
mod git;
mod http;
mod notifier;
mod processor;
mod service;
mod vault;

use config::Config;
use service::registry;

#[tokio::main]
async fn main() -> Result<()> {
    // Parse the cli
    let cli = Args::from_args();

    // Get the configuration
    let config = config::parse(cli.config)
        .await
        .context("Failed to load configuration")?;
    let address = cli.address.unwrap_or(config.http.address);

    // Ensure the clone directory exists
    if !config.git.clone_to.exists() {
        fs::create_dir_all(&config.git.clone_to)
            .await
            .context("Failed to create configuration clone directory")?;
    }

    // Setup logging
    init_tracing(
        cli.log_level.unwrap_or_else(|| config.agent.log.clone()),
        config.agent.tokio_console,
    );

    // Initialize sentry
    let _guard = sentry::init(sentry_config(&config.agent.sentry)?);

    match run_server(address, config).await {
        Ok(()) => Ok(()),
        Err(e) => {
            capture_anyhow(&e);
            Err(e)
        }
    }
}

/// Connect to the services and start the server
async fn run_server(address: SocketAddr, config: Arc<Config>) -> Result<()> {
    let (stop_tx, _) = broadcast::channel(1);

    // Initialize the service registry
    registry::init(&config.git.clone_to).await?;

    // Connect to the repository service
    let repository_handle = git::initialize(&config.git.clone_to);

    // Connect to the deployment service
    deployer::initialize(&config.deployment, &config.dns.server, stop_tx.subscribe()).await?;

    // Connect to Vault (secrets service)
    vault::initialize(&config.secrets, stop_tx.clone()).await?;

    // Connect to the DNS management service
    dns::initialize(&config.dns).await?;

    // Setup the notifier service
    notifier::initialize(&config)?;

    // Start the job processor
    processor::spawn(config.clone(), stop_tx.clone());

    // Start the server
    info!(%address, "listening and ready to handle requests");
    let routes = http::routes(config);
    Server::bind(&address)
        .serve(routes.into_make_service())
        .with_graceful_shutdown(wait_for_exit())
        .await?;

    info!("signal received, shutting down...");

    // Shutdown the services
    stop_tx.send(()).unwrap();

    // Shutdown the repository service
    git::instance().shutdown();
    repository_handle.join().unwrap();

    info!("successfully shutdown, good bye!");
    Ok(())
}

/// Wait for a SIGINT or SIGTERM and then exit
async fn wait_for_exit() {
    let mut int = signal(SignalKind::interrupt()).unwrap();
    let mut term = signal(SignalKind::terminate()).unwrap();

    tokio::select! {
        _ = int.recv() => {},
        _ = term.recv() => {},
    }
}

/// Generate a registry for tracing
fn init_tracing(raw_filter: String, enable_console: bool) {
    let filter = EnvFilter::builder().parse_lossy(raw_filter);

    if enable_console {
        tracing_subscriber::registry()
            .with(ConsoleLayer::builder().with_default_env().spawn())
            .with(
                tracing_subscriber::fmt::layer()
                    .with_span_events(FmtSpan::CLOSE)
                    .with_filter(filter),
            )
            .with(sentry_tracing::layer())
            .init();
    } else {
        tracing_subscriber::registry()
            .with(
                tracing_subscriber::fmt::layer()
                    .with_span_events(FmtSpan::CLOSE)
                    .with_filter(filter),
            )
            .with(sentry_tracing::layer())
            .init();
    }
}

/// Generate configuration for Sentry
fn sentry_config(url: &Option<String>) -> Result<ClientOptions> {
    let dsn = url
        .as_ref()
        .map(String::as_str)
        .map(IntoDsn::into_dsn)
        .transpose()
        .context("failed to parse Sentry DSN")?
        .flatten();

    let options = ClientOptions {
        dsn,
        release: sentry::release_name!(),
        attach_stacktrace: true,
        ..Default::default()
    };

    Ok(options)
}

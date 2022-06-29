use anyhow::{Context, Result};
use axum::Server;
use console_subscriber::ConsoleLayer;
use sentry::{
    integrations::{anyhow::capture_anyhow, tracing as sentry_tracing},
    ClientOptions, IntoDsn,
};
use std::net::SocketAddr;
use structopt::StructOpt;
use tokio::{
    fs,
    signal::unix::{signal, SignalKind},
    sync::broadcast,
    task,
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
mod management;
mod notifier;
mod processor;
mod service;
mod vault;
mod webhooks;

use config::Config;
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

    // Ensure the clone directory exists
    if !configuration.git.clone_to.exists() {
        fs::create_dir_all(&configuration.git.clone_to)
            .await
            .context("Failed to create configuration clone directory")?;
    }

    // Setup logging
    init_tracing(
        cli.log_level
            .unwrap_or_else(|| configuration.agent.log.clone()),
        configuration.agent.tokio_console,
    );

    // Initialize sentry
    let _guard = sentry::init(sentry_config(&configuration.agent.sentry)?);

    match run_server(address, configuration).await {
        Ok(()) => Ok(()),
        Err(e) => {
            capture_anyhow(&e);
            Err(e)
        }
    }
}

/// Connect to the services and start the server
async fn run_server(address: SocketAddr, configuration: &Config) -> Result<()> {
    let (stop_tx, mut stop_rx) = broadcast::channel(1);

    // Initialize the service registry
    registry::init().await?;

    // Connect to the repository service
    let repository_handle = git::initialize(&configuration.git.clone_to);

    // Connect to the deployment service
    deployer::initialize(
        &configuration.deployment,
        &configuration.dns.server,
        stop_tx.subscribe(),
    )
    .await?;

    // Connect to Vault (secrets service)
    vault::initialize(&configuration.secrets, stop_tx.clone()).await?;

    // Connect to the DNS management service
    dns::initialize(&configuration.dns).await?;

    // Setup the notifier service
    notifier::initialize()?;

    // Start the job processor
    processor::spawn(stop_tx.clone());

    // Start the management interface
    management::start(stop_tx.clone());

    // Bind the server
    let server = Server::bind(&address)
        .serve(webhooks::routes().into_make_service())
        .with_graceful_shutdown(async move {
            stop_rx.recv().await.ok();
        });

    // Start the server
    task::spawn(server);
    info!("listening on {}", address);

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

/// Wait for a SIGINT or SIGTERM and then exit
async fn wait_for_exit() -> Result<()> {
    let mut int = signal(SignalKind::interrupt())?;
    let mut term = signal(SignalKind::terminate())?;

    tokio::select! {
        _ = int.recv() => Ok(()),
        _ = term.recv() => Ok(()),
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

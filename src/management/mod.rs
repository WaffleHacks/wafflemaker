use super::config;
use tokio::sync::broadcast::Sender;
use tokio::task;
use tracing::{info, instrument};
use warp::{Error, Filter};

/// Start the management interface
#[instrument(skip(stop_tx))]
pub fn start(stop_tx: Sender<()>) -> Result<(), Error> {
    let config = &config::instance().management;

    // Don't start if disabled
    if !config.enabled {
        return Ok(());
    }

    // Build the routes
    let routes = warp::path("test").map(|| "test");
    let (address, server) =
        warp::serve(routes).try_bind_with_graceful_shutdown(config.address, async move {
            stop_tx.subscribe().recv().await.ok();
        })?;

    // Start the server
    task::spawn(server);
    info!("management interface listening on {}", address);

    Ok(())
}

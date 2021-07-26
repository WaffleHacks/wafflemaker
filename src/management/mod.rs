use super::config;
use tokio::sync::broadcast::Sender;
use tokio::task;
use tracing::{info, instrument};
use warp::{Error, Filter, Rejection};

/// Start the management interface
#[instrument(skip(stop_tx))]
pub fn start(stop_tx: Sender<()>) -> Result<(), Error> {
    let config = &config::instance().management;

    // Don't start if disabled
    if !config.enabled {
        return Ok(());
    }

    // Build the routes
    let routes = authentication(&config.token).and(warp::path("test").map(|| "test"));
    let (address, server) =
        warp::serve(routes).try_bind_with_graceful_shutdown(config.address, async move {
            stop_tx.subscribe().recv().await.ok();
        })?;

    // Start the server
    task::spawn(server);
    info!("management interface listening on {}", address);

    Ok(())
}

/// Check the authentication header
fn authentication(
    expected_token: &'static str,
) -> impl Filter<Extract = (), Error = Rejection> + Copy {
    warp::header::<String>("Authorization")
        .and_then(move |header: String| async move {
            let token = header
                .strip_prefix("Bearer")
                .ok_or(warp::reject::not_found())?; // TODO: better errors

            if token != expected_token {
                Err(warp::reject::not_found()) // TODO: better errors
            } else {
                Ok(())
            }
        })
        .untuple_one()
}

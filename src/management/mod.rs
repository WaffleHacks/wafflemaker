use crate::{
    config,
    http::{recover, AuthorizationError},
};
use tokio::sync::broadcast::Sender;
use tokio::task;
use tracing::{info, instrument};
use warp::{Error, Filter, Rejection};

mod deployments;
mod leases;
mod services;

/// Start the management interface
#[instrument(skip(stop_tx))]
pub fn start(stop_tx: Sender<()>) -> Result<(), Error> {
    let config = &config::instance().management;

    // Don't start if disabled
    if !config.enabled {
        return Ok(());
    }

    // Build the routes
    let routes = deployments::routes()
        .or(leases::routes())
        .or(services::routes());
    let with_middleware = warp::any()
        .and(authentication(&config.token).and(routes))
        .recover(recover);
    let (address, server) = warp::serve(with_middleware).try_bind_with_graceful_shutdown(
        config.address,
        async move {
            stop_tx.subscribe().recv().await.ok();
        },
    )?;

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
            if let Some(token) = header.strip_prefix("Bearer ") {
                if token == expected_token {
                    return Ok(());
                }
            }

            Err(Rejection::from(AuthorizationError))
        })
        .untuple_one()
}

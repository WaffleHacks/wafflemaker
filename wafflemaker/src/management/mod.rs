use crate::config;
use axum::{
    headers::{authorization::Bearer, Authorization, Header},
    http::{Request, StatusCode},
    middleware::{self, Next},
    response::Response,
    Extension, Router, Server,
};
use tokio::sync::broadcast::Sender;
use tokio::task;
use tracing::{info, instrument};

mod deployments;
mod leases;
mod services;

#[derive(Clone)]
struct AuthenticationToken(String);

/// Start the management interface
#[instrument(skip(stop_tx))]
pub fn start(stop_tx: Sender<()>) {
    let config = &config::instance().management;

    // Don't start if disabled
    if !config.enabled {
        return;
    }

    // Build the routes
    let router = Router::new()
        .nest("/deployments", deployments::routes())
        .nest("/leases", leases::routes())
        .nest("/services", services::routes())
        .route_layer(middleware::from_fn(authentication))
        .layer(Extension(AuthenticationToken(config.token.clone())));

    let server = Server::bind(&config.address)
        .serve(router.into_make_service())
        .with_graceful_shutdown(async move {
            stop_tx.subscribe().recv().await.ok();
        });

    // Start the server
    task::spawn(server);
    info!("management interface listening on {}", config.address);
}

/// Check the authentication header
async fn authentication<B>(req: Request<B>, next: Next<B>) -> Result<Response, StatusCode> {
    let AuthenticationToken(expected_token) = req.extensions().get().unwrap();

    let header = req
        .headers()
        .get(Authorization::<Bearer>::name())
        .ok_or(StatusCode::UNAUTHORIZED)?;
    let authorization = Authorization::<Bearer>::decode(&mut [header].into_iter())
        .map_err(|_| StatusCode::UNAUTHORIZED)?;

    if authorization.token() == expected_token {
        Ok(next.run(req).await)
    } else {
        Err(StatusCode::UNAUTHORIZED)
    }
}

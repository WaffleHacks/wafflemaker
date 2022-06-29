use super::{Error, Result};
use axum::{
    headers::{authorization::Bearer, Authorization, Header},
    http::Request,
    middleware::{self, Next},
    response::Response,
    Extension, Router,
};

mod deployments;
mod leases;
mod services;

#[derive(Clone)]
struct AuthenticationToken(String);

/// Build the routes for the management API
pub fn routes(token: String) -> Router {
    Router::new()
        .nest("/deployments", deployments::routes())
        .nest("/leases", leases::routes())
        .nest("/services", services::routes())
        .route_layer(middleware::from_fn(authentication))
        .layer(Extension(AuthenticationToken(token)))
}

/// Check the authentication header
async fn authentication<B>(req: Request<B>, next: Next<B>) -> Result<Response> {
    let AuthenticationToken(expected_token) = req.extensions().get().unwrap();

    let header = req
        .headers()
        .get(Authorization::<Bearer>::name())
        .ok_or(Error::Unauthorized)?;
    let authorization = Authorization::<Bearer>::decode(&mut [header].into_iter())
        .map_err(|_| Error::Unauthorized)?;

    if authorization.token() == expected_token {
        Ok(next.run(req).await)
    } else {
        Err(Error::Unauthorized)
    }
}

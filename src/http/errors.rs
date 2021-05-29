use serde::Serialize;
use std::convert::Infallible;
use warp::http::StatusCode;
use warp::{
    body::BodyDeserializeError,
    reject::{MethodNotAllowed, MissingHeader, PayloadTooLarge, Reject},
    reply, Rejection, Reply,
};

/// An API error serializable to JSON
#[derive(Serialize)]
struct Error<'a> {
    pub code: u16,
    pub message: &'a str,
}

/// Raised when the signature is invalid or cannot be processed
#[derive(Debug)]
pub struct AuthorizationError;
impl Reject for AuthorizationError {}

/// Raised when the repo is not allowed to be deployed
#[derive(Debug)]
pub struct UndeployableError;
impl Reject for UndeployableError {}

/// Convert a `Rejection` to an API error, otherwise simply passes
/// the rejection along.
pub async fn recover(error: Rejection) -> Result<impl Reply, Infallible> {
    let code;
    let message;

    if error.is_not_found() {
        code = StatusCode::NOT_FOUND;
        message = "not found";
    } else if error.find::<MissingHeader>().is_some()
        || error.find::<BodyDeserializeError>().is_some()
    {
        code = StatusCode::BAD_REQUEST;
        message = "bad request";
    } else if error.find::<MethodNotAllowed>().is_some() {
        code = StatusCode::METHOD_NOT_ALLOWED;
        message = "method not allowed";
    } else if error.find::<PayloadTooLarge>().is_some() {
        code = StatusCode::PAYLOAD_TOO_LARGE;
        message = "payload too large";
    } else if error.find::<AuthorizationError>().is_some() {
        code = StatusCode::UNAUTHORIZED;
        message = "unauthorized";
    } else if error.find::<UndeployableError>().is_some() {
        code = StatusCode::FORBIDDEN;
        message = "forbidden";
    } else {
        code = StatusCode::INTERNAL_SERVER_ERROR;
        message = "internal server error";
    }

    // Build response
    let json = reply::json(&Error {
        code: code.as_u16(),
        message,
    });
    Ok(reply::with_status(json, code))
}

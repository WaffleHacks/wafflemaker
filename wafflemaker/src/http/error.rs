use crate::deployer::Error as DeployerError;
use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
    Json,
};
use git2::Error as Git2Error;
use hex::FromHexError;
use ring::error::Unspecified;
use serde_json::{error::Error as SerdeError, json};
use std::error::Error as StdError;
use tracing::error;

pub(crate) type Result<T> = std::result::Result<T, Error>;

pub enum Error {
    Unauthorized,
    DisallowedRepository,
    Git(Git2Error),
    Deployer(DeployerError),
    InvalidJson,
    ServiceNotFound,
}

impl IntoResponse for Error {
    fn into_response(self) -> Response {
        let (status, message) = match self {
            Error::Unauthorized => (StatusCode::UNAUTHORIZED, "unauthorized"),
            Error::DisallowedRepository => (StatusCode::FORBIDDEN, "disallowed repository"),
            Error::InvalidJson => (StatusCode::BAD_REQUEST, "invalid JSON"),
            Error::ServiceNotFound => (StatusCode::NOT_FOUND, "service not found"),
            Error::Git(e) => {
                error!(
                    class = ?e.class(), code = ?e.code(),
                    message = %e.message(),
                    "error while interacting with local repo",
                );
                (StatusCode::INTERNAL_SERVER_ERROR, "internal server error")
            }
            Error::Deployer(e) => {
                error!(error = %e, source = ?e.source(), "error while fetching deployment information");
                (StatusCode::INTERNAL_SERVER_ERROR, "internal server error")
            }
        };

        let body = Json(json!({ "message": message }));
        (status, body).into_response()
    }
}

impl From<SerdeError> for Error {
    fn from(_: SerdeError) -> Self {
        Error::InvalidJson
    }
}

impl From<DeployerError> for Error {
    fn from(e: DeployerError) -> Self {
        Error::Deployer(e)
    }
}

impl From<Git2Error> for Error {
    fn from(e: Git2Error) -> Self {
        Error::Git(e)
    }
}

impl From<FromHexError> for Error {
    fn from(_: FromHexError) -> Self {
        Error::Unauthorized
    }
}

impl From<Unspecified> for Error {
    fn from(_: Unspecified) -> Self {
        Error::Unauthorized
    }
}

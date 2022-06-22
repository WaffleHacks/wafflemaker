use reqwest::{header::InvalidHeaderValue, Error as ReqwestError};
use std::num::ParseIntError;
use thiserror::Error as ThisError;
use url::ParseError;

pub(crate) type Result<T> = std::result::Result<T, Error>;

/// The possible errors raised by Vault
#[derive(Debug, ThisError)]
pub enum Error {
    #[error("check your secrets configuration is correct")]
    Config(#[source] ConfigSource),
    #[error("failed to parse response body")]
    Deserialize(#[source] ReqwestError),
    #[error("token has incorrect permissions")]
    InvalidPermissions,
    #[error("failed to serialize request body")]
    Serialize(#[source] ReqwestError),
    #[error("unexpected status code {code}")]
    Status { code: u16, source: ReqwestError },
    #[error("request timed out")]
    Timeout(#[source] ReqwestError),
    #[error("an unknown error occurred while sending the request")]
    Unknown(#[source] ReqwestError),
}

/// A wrapper around configuration errors
#[derive(Debug, ThisError)]
pub enum ConfigSource {
    #[error("failed to build client")]
    Client(#[from] ReqwestError),
    #[error("invalid duration format")]
    Duration(#[from] ParseIntError),
    #[error("invalid token format")]
    Token(#[from] InvalidHeaderValue),
    #[error("invalid Vault address")]
    Url(#[from] ParseError),
}

impl From<InvalidHeaderValue> for Error {
    fn from(error: InvalidHeaderValue) -> Error {
        Error::Config(error.into())
    }
}

impl From<ReqwestError> for Error {
    fn from(error: ReqwestError) -> Error {
        if error.is_builder() {
            Error::Config(error.into())
        } else if error.is_timeout() {
            Error::Timeout(error)
        } else if error.is_status() {
            Error::Status {
                code: error.status().unwrap_or_default().as_u16(),
                source: error,
            }
        } else if error.is_decode() {
            Error::Deserialize(error)
        } else if error.is_body() {
            Error::Serialize(error)
        } else {
            Error::Unknown(error)
        }
    }
}

impl From<ParseError> for Error {
    fn from(error: ParseError) -> Error {
        Error::Config(error.into())
    }
}

impl From<ParseIntError> for Error {
    fn from(error: ParseIntError) -> Error {
        Error::Config(error.into())
    }
}

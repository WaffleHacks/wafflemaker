use jsonwebtoken::errors::Error as JwtError;
use reqwest::{header::InvalidHeaderValue, Error as ReqwestError};
use std::io::Error as IoError;
use thiserror::Error as ThisError;
use url::ParseError;

pub type Result<T> = std::result::Result<T, Error>;

/// Possible errors that can arise when sending a notification
#[derive(Debug, ThisError)]
pub enum Error {
    // initialization errors
    #[error("invalid repository format, must be `<owner>/<repo>`")]
    InvalidRepository,
    #[error("could not parse URL")]
    InvalidUrl(#[from] ParseError),
    #[error("the provided credentials could not be encoded")]
    InvalidCredentials(#[from] InvalidHeaderValue),
    #[error("could not find key at specified path")]
    InvalidKeyPath,

    // runtime errors
    #[error("failed to deserialize response body")]
    Deserialize(#[source] ReqwestError),
    #[error("failed to serialize request body")]
    Serialize(#[source] ReqwestError),
    #[error("unexpected status code {code}")]
    Status { code: u16, source: ReqwestError },
    #[error("request timed out")]
    Timeout(#[source] ReqwestError),
    #[error("failed to read file")]
    IO(#[from] IoError),
    #[error("failed to process JWT")]
    Jwt(#[from] JwtError),
    #[error("an unknown error occurred while sending the request")]
    Unknown(#[source] ReqwestError),
}

impl From<ReqwestError> for Error {
    fn from(error: ReqwestError) -> Error {
        if error.is_timeout() {
            Error::Timeout(error)
        } else if error.is_status() {
            Error::Status {
                code: error.status().unwrap_or_default().as_u16(),
                source: error,
            }
        } else if error.is_body() {
            Error::Serialize(error)
        } else if error.is_decode() {
            Error::Deserialize(error)
        } else {
            Error::Unknown(error)
        }
    }
}

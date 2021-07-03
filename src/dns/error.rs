use anyhow::Error as AnyError;
use cloudflare::framework::response::ApiFailure;
use reqwest::Error as ReqwestError;
use thiserror::Error as ThisError;

pub(crate) type Result<T> = ::std::result::Result<T, Error>;

/// The possible errors raised by the DNS client
#[derive(Debug, ThisError)]
pub enum Error {
    #[error("unknown zone {0}")]
    NonExistentZone(String),
    #[error("{0} DNS zone(s) not found in account")]
    MissingZones(usize),
    #[error("failed to build DNS client")]
    ClientError(#[from] AnyError),
    #[error("{canonical} ({status}): {message}")]
    Http {
        canonical: String,
        status: u16,
        message: String,
    },
    #[error("failed to send request")]
    RequestError(#[from] ReqwestError),
    #[error("the DNS client has not been initialized")]
    Uninitialized,
}

impl From<ApiFailure> for Error {
    fn from(error: ApiFailure) -> Error {
        match error {
            ApiFailure::Invalid(e) => e.into(),
            ApiFailure::Error(status, errors) => Error::Http {
                status: status.as_u16(),
                canonical: status
                    .canonical_reason()
                    .map(String::from)
                    .unwrap_or_default(),
                message: errors
                    .errors
                    .get(0)
                    .map(|e| format!("{} ({})", e.message, e.code))
                    .unwrap_or_default(),
            },
        }
    }
}

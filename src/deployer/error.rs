use bollard::errors::Error as BollardError;
use thiserror::Error as ThisError;

pub(crate) type Result<T> = std::result::Result<T, Error>;

/// The possible errors raised by the deployer
#[derive(Debug, ThisError)]
pub enum Error {
    #[error("unable to connect to deployer")]
    Connection(#[source] ErrorSource),
    #[error("failed to parse response")]
    Parsing(#[source] ErrorSource),
    #[error("failed to serialize data")]
    Serialization(#[source] ErrorSource),
    #[error("unable to request resource")]
    Http(#[source] ErrorSource),
    #[error("resource could not be found")]
    NotFound(#[source] ErrorSource),
    #[error("resource already exists")]
    Exists(#[source] ErrorSource),
    #[error("request timed out")]
    Timeout(#[source] ErrorSource),
    #[error("an unknown error occurred")]
    Unknown(#[source] ErrorSource),
}

/// A wrapper around the deployer specific error types to allow source errors.
#[derive(Debug, ThisError)]
pub enum ErrorSource {
    #[error(transparent)]
    Docker(#[from] BollardError),
}

impl From<BollardError> for Error {
    fn from(error: BollardError) -> Error {
        match error {
            BollardError::NoCertPathError
            | BollardError::CertPathError { .. }
            | BollardError::CertMultipleKeys { .. }
            | BollardError::CertParseError { .. }
            | BollardError::APIVersionParseError { .. } => Self::Connection(error.into()),
            BollardError::JsonDataError { .. }
            | BollardError::JsonSerdeError { .. }
            | BollardError::StrParseError { .. } => Self::Parsing(error.into()),
            BollardError::StrFmtError { .. } | BollardError::URLEncodedError { .. } => {
                Self::Serialization(error.into())
            }
            BollardError::DockerResponseServerError { .. }
            | BollardError::DockerResponseBadParameterError { .. }
            | BollardError::DockerResponseNotModifiedError { .. }
            | BollardError::HttpClientError { .. }
            | BollardError::HyperResponseError { .. } => Self::Http(error.into()),
            BollardError::DockerResponseNotFoundError { .. } => Self::NotFound(error.into()),
            BollardError::DockerResponseConflictError { .. } => Self::Exists(error.into()),
            BollardError::RequestTimeoutError => Self::Timeout(error.into()),
            e => Self::Unknown(e.into()),
        }
    }
}

use std::fmt::{Display, Formatter};

pub type Result<T> = std::result::Result<T, Error>;

/// A wrapper around possible errors that can occur during storage
#[derive(Debug)]
pub enum Error {
    Database(sqlx::Error),
    Encoding(serde_json::Error),
}

impl std::error::Error for Error {}

impl Display for Error {
    fn fmt(&self, f: &mut Formatter<'_>) -> std::fmt::Result {
        match self {
            Self::Database(e) => write!(f, "{e}"),
            Self::Encoding(e) => write!(f, "{e}"),
        }
    }
}

impl From<serde_json::Error> for Error {
    fn from(e: serde_json::Error) -> Self {
        Self::Encoding(e)
    }
}

impl From<sqlx::Error> for Error {
    fn from(e: sqlx::Error) -> Self {
        Self::Database(e)
    }
}

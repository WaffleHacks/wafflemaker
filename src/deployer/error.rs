use thiserror::Error as ThisError;

pub(crate) type Result<T> = std::result::Result<T, Error>;

/// The possible errors raised by the deployer
#[derive(Debug, ThisError)]
pub enum Error {
    // TODO: figure out what errors should be raised
}

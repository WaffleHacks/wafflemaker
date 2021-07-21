use super::{
    error::Result,
    events::{Event, State},
};

mod discord;
mod github;

pub use discord::dispatch as discord;
pub use github::dispatch as github;

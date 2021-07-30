use crate::http::Client;
use eyre::Result;
use serde::Deserialize;
use structopt::StructOpt;
use tabled::{Table, Tabled};

mod add;
mod delete;
mod get;
mod run;

pub use add::Add;
pub use delete::Delete;
pub use get::Get;
pub use run::Run;

pub trait Subcommand {
    fn execute(&self, client: Client) -> Result<Option<Table>>;
}

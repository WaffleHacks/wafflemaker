use eyre::Result;
use reqwest::blocking::Client;
use serde::Deserialize;
use structopt::StructOpt;
use tabled::{Table, Tabled};
use url::Url;

mod add;
mod delete;
mod get;
mod run;

pub use add::Add;
pub use delete::Delete;
pub use get::Get;
pub use run::Run;

pub trait Subcommand {
    fn execute(&self, client: Client, url: Url) -> Result<Table>;
}

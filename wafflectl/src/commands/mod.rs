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

pub use add::AddSubcommand;
pub use delete::DeleteSubcommand;
pub use get::GetSubcommand;
pub use run::RunSubcommand;

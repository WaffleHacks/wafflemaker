use eyre::Result;
use reqwest::blocking::Client;
use structopt::StructOpt;
use url::Url;

mod add;
mod delete;
mod get;
mod run;

pub use add::AddSubcommand;
pub use delete::DeleteSubcommand;
pub use get::GetSubcommand;
pub use run::RunSubcommand;

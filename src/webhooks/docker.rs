use serde::Deserialize;

#[derive(Debug, Deserialize)]
pub struct Docker {
    callback_url: String,
    push_data: PushData,
    repository: Repository,
}

#[derive(Debug, Deserialize)]
pub struct PushData {
    tag: String,
}

#[derive(Debug, Deserialize)]
pub struct Repository {
    repo_name: String,
}

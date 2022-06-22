use super::Result;
use git2::Repository;
use tracing::instrument;

/// Get the current head of the repository
#[instrument(name = "head", skip(repo))]
pub(crate) fn run(repo: &Repository) -> Result<String> {
    let head = repo.head()?;
    let commit = head.peel_to_commit()?.id();
    Ok(hex::encode(commit.as_bytes()))
}

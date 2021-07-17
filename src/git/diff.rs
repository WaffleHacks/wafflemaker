use super::Result;
use git2::{Delta, DiffDelta, DiffOptions, Object, ObjectType, Repository};
use std::path::{Path, PathBuf};
use tracing::{debug, info, instrument};

/// The diff action that happened to the file
#[derive(Debug)]
pub enum Action {
    Deleted,
    Modified,
    Unknown,
}

impl From<Delta> for Action {
    fn from(delta: Delta) -> Action {
        match delta {
            Delta::Deleted => Action::Deleted,
            Delta::Added | Delta::Modified | Delta::Copied | Delta::Renamed => Action::Modified,
            _ => Action::Unknown,
        }
    }
}

/// A diff file
#[derive(Debug)]
pub struct DiffFile {
    pub action: Action,
    pub path: PathBuf,
    pub binary: bool,
}

impl From<DiffDelta<'_>> for DiffFile {
    fn from(diff: DiffDelta) -> DiffFile {
        let file = if diff.status() == Delta::Deleted {
            diff.old_file()
        } else {
            diff.new_file()
        };

        DiffFile {
            action: Action::from(diff.status()),
            path: file.path().map_or_else(PathBuf::new, Path::to_path_buf),
            binary: file.is_binary(),
        }
    }
}

/// Calculate the diff between two commits
#[instrument(name = "diff", skip(repo))]
pub(crate) fn run(repo: &Repository, before: &str, after: &str) -> Result<Vec<DiffFile>> {
    // Convert the commit references to objects
    let before = to_object(repo, before)?;
    let after = to_object(repo, after)?;
    debug!("converted references to tree objects");

    // Compute the diff
    let mut options = DiffOptions::new();
    options
        .ignore_whitespace(true)
        .ignore_whitespace_change(true);
    let diff = repo.diff_tree_to_tree(before.as_tree(), after.as_tree(), Some(&mut options))?;
    info!("computed diff between trees");

    debug!("converting to custom representation");
    Ok(diff.deltas().map(DiffFile::from).collect())
}

/// Convert a commit to an object
fn to_object<'r>(repo: &'r Repository, commit: &str) -> Result<Object<'r>> {
    let obj = repo.revparse_single(commit)?;
    obj.peel(ObjectType::Tree)
}

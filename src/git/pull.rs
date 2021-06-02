use super::Result;
use git2::{
    build::CheckoutBuilder, AnnotatedCommit, AutotagOption, FetchOptions, Reference, Remote,
    RemoteCallbacks, Repository,
};
use tracing::{debug, error, info, info_span};

/// Pull a reference from the given remote URL
pub(crate) fn run(repo: &Repository, clone_url: &str, refspec: &str) -> Result<()> {
    let span = info_span!("pull", clone_url = clone_url, refspec = refspec);
    let _ = span.enter();

    // Get the remote
    repo.remote_set_url("origin", clone_url)?;
    let remote = repo.find_remote("origin").unwrap();

    // Pull from the remote
    info!(parent: &span, "pulling from {}", refspec);
    let fetch_commit = fetch(repo, refspec, remote)?;

    // Merge into the local head
    info!(
        parent: &span,
        "merging into {}",
        fetch_commit.refname().unwrap_or(&refspec)
    );
    merge(repo, refspec, fetch_commit)?;

    Ok(())
}

/// Fetch all the data in the given refspec
fn fetch<'r>(
    repo: &'r Repository,
    refspec: &str,
    mut remote: Remote,
) -> Result<AnnotatedCommit<'r>> {
    let span = info_span!("fetch", refspec = refspec);
    let _ = span.enter();

    // Log transfer progress
    let mut callback = RemoteCallbacks::new();
    callback.transfer_progress(|stats| {
        if stats.received_objects() == stats.total_objects() {
            debug!(
                parent: &span,
                "resolving deltas {}/{}",
                stats.indexed_deltas(),
                stats.total_deltas()
            );
        } else if stats.total_objects() > 0 {
            debug!(
                parent: &span,
                "received {}/{} objects ({}) in {} bytes",
                stats.received_objects(),
                stats.total_objects(),
                stats.indexed_objects(),
                stats.received_bytes()
            );
        }
        true
    });

    // Fetch from the remote
    // Always fetch tags
    remote.fetch(
        &[refspec],
        Some(
            FetchOptions::new()
                .remote_callbacks(callback)
                .download_tags(AutotagOption::All),
        ),
        None,
    )?;

    // Log the stats of the fetch
    let stats = remote.stats();
    if stats.local_objects() > 0 {
        info!(
            parent: &span,
            "received {}/{} objects in {} bytes (used {} local objects",
            stats.indexed_objects(),
            stats.total_objects(),
            stats.received_bytes(),
            stats.local_objects()
        );
    } else {
        info!(
            parent: &span,
            "received {}/{} objects in {} bytes",
            stats.indexed_objects(),
            stats.total_objects(),
            stats.received_bytes()
        );
    }

    let fetch_head = repo.find_reference("FETCH_HEAD")?;
    repo.reference_to_annotated_commit(&fetch_head)
}

/// Merge the pulled branch and the current history
/// Supports normal merge and fast-forwarding, but will not try to resolve conflicts.
fn merge(repo: &Repository, refname: &str, fetch_commit: AnnotatedCommit) -> Result<()> {
    let span = info_span!(
        "merge",
        refname = refname,
        commit = fetch_commit.refname().unwrap_or_default()
    );
    let _ = span.enter();

    // Run a merge analysis
    let analysis = repo.merge_analysis(&[&fetch_commit])?;

    // Do the appropriate merge (fast-forward/normal/none)
    if analysis.0.is_fast_forward() {
        info!(parent: &span, "merging with fast-forward");

        match repo.find_reference(&refname) {
            Ok(mut r) => fast_forward(repo, &mut r, &fetch_commit)?,
            Err(_) => {
                // Set reference to commit directly
                repo.reference(
                    &refname,
                    fetch_commit.id(),
                    true,
                    &format!("setting {} to {}", refname, fetch_commit.id()),
                )?;
                repo.set_head(&refname)?;

                // Checkout the head
                repo.checkout_head(Some(
                    CheckoutBuilder::default()
                        .allow_conflicts(true)
                        .conflict_style_merge(true)
                        .force(),
                ))?;
            }
        }
    } else if analysis.0.is_normal() {
        info!(parent: &span, "merging normally");

        let head_reference = repo.head()?;
        let head = repo.reference_to_annotated_commit(&head_reference)?;
        normal_merge(repo, &head, &fetch_commit)?;
    } else {
        info!(parent: &span, "no merge necessary");
    }

    Ok(())
}

/// Perform a fast forward merge
fn fast_forward(
    repo: &Repository,
    local_branch: &mut Reference,
    remote_commit: &AnnotatedCommit,
) -> Result<()> {
    let span = info_span!(
        "fast_forward",
        local_branch = local_branch.name().unwrap_or_default(),
        commit = remote_commit.refname().unwrap_or_default()
    );
    let _ = span.enter();

    // Get the name of the branch
    let name = match local_branch.name() {
        Some(s) => s.to_string(),
        None => String::from_utf8_lossy(local_branch.name_bytes()).to_string(),
    };

    // Re-target the current branch
    local_branch.set_target(
        remote_commit.id(),
        &format!(
            "Fast-forward: setting {} to id {}",
            name,
            remote_commit.id()
        ),
    )?;

    // Set the head
    repo.set_head(&name)?;
    repo.checkout_head(Some(CheckoutBuilder::default().force()))?;

    info!(
        parent: &span,
        "successfully fast-forwarded {} to {}",
        name,
        remote_commit.id()
    );

    Ok(())
}

/// Perform a normal merge
fn normal_merge(
    repo: &Repository,
    local: &AnnotatedCommit,
    remote: &AnnotatedCommit,
) -> Result<()> {
    let span = info_span!(
        "normal_merge",
        local = local.refname().unwrap_or_default(),
        remote = local.refname().unwrap_or_default()
    );
    let _ = span.enter();

    // Find the common ancestor between the two commits
    let local_tree = repo.find_commit(local.id())?.tree()?;
    let remote_tree = repo.find_commit(remote.id())?.tree()?;
    let ancestor = repo
        .find_commit(repo.merge_base(local.id(), remote.id())?)?
        .tree()?;

    // Merge the two trees
    let mut index = repo.merge_trees(&ancestor, &local_tree, &remote_tree, None)?;

    // Don't attempt to resolve conflicts
    if index.has_conflicts() {
        error!(
            parent: &span,
            "merge conflicts detected, cannot resolve automatically"
        );
        repo.checkout_index(Some(&mut index), None)?;
        return Ok(());
    }

    // Finalize the merge
    let result_tree = repo.find_tree(index.write_tree_to(&repo)?)?;
    let signature = repo.signature()?;
    let local_commit = repo.find_commit(local.id())?;
    let remote_commit = repo.find_commit(remote.id())?;
    repo.commit(
        Some("HEAD"),
        &signature,
        &signature,
        &format!("Merge: {} into {}", remote.id(), local.id()),
        &result_tree,
        &[&local_commit, &remote_commit],
    )?;

    info!(
        parent: &span,
        "successfully merged from {} to {}",
        remote.id(),
        local.id()
    );

    Ok(())
}

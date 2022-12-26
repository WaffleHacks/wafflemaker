use super::Result;
use serde::{Deserialize, Serialize};
use sqlx::{query_as, PgPool};
use std::path::Path;

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Deployment {
    pub commit: String,
}

impl Deployment {
    /// Get all the deployments
    pub async fn all(pool: &PgPool) -> Result<Vec<Deployment>> {
        let deployments = query_as!(Deployment, "SELECT * FROM deployments")
            .fetch_all(pool)
            .await?;

        Ok(deployments)
    }

    /// Find a deployment by its commit
    pub async fn find<S>(commit: S, pool: &PgPool) -> Result<Option<Deployment>>
    where
        S: AsRef<str>,
    {
        let commit = commit.as_ref();
        let deployment = query_as!(
            Deployment,
            "SELECT * FROM deployments WHERE commit = $1",
            commit
        )
        .fetch_optional(pool)
        .await?;

        Ok(deployment)
    }

    /// Create a new deployment
    pub async fn create(commit: String, pool: &PgPool) -> Result<Deployment> {
        let deployment = query_as!(
            Deployment,
            "INSERT INTO deployments VALUES ($1) RETURNING commit",
            commit
        )
        .fetch_one(pool)
        .await?;

        Ok(deployment)
    }

    /// Add a change to deployment
    pub async fn add_change<P>(&self, path: P, action: Action, pool: &PgPool) -> Result<Change>
    where
        P: AsRef<Path>,
    {
        let path = path.as_ref().display().to_string();
        let change = query_as!(
            Change,
            "INSERT INTO changes VALUES ($1, $2, $3) RETURNING commit, path, action AS \"action: _\"",
            self.commit,
            path,
            action as _
        )
        .fetch_one(pool)
        .await?;

        Ok(change)
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize, sqlx::Type)]
#[serde(rename_all = "lowercase")]
#[sqlx(rename_all = "lowercase", type_name = "change_action")]
pub enum Action {
    Modified,
    Deleted,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
pub struct Change {
    pub commit: String,
    pub path: String,
    pub action: Action,
}

impl Change {
    pub async fn for_deployment<S>(commit: S, pool: &PgPool) -> Result<Vec<Change>>
    where
        S: AsRef<str>,
    {
        let commit = commit.as_ref();
        let changes = query_as!(
            Change,
            "SELECT commit, path, action AS \"action: _\" FROM changes WHERE commit = $1",
            commit
        )
        .fetch_all(pool)
        .await?;

        Ok(changes)
    }
}

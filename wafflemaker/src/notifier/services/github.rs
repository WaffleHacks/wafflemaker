use super::*;
use jsonwebtoken::{Algorithm, EncodingKey, Header};
use reqwest::{header::AUTHORIZATION, Client};
use serde::{Deserialize, Serialize};
use std::path::Path;
use tokio::fs;
use tracing::{debug, error, instrument};

/// Dispatch an event to GitHub
#[instrument(skip(client, event, key, app_id, installation_id), fields(event = %event))]
pub async fn dispatch(
    client: &Client,
    owner: &str,
    repo: &str,
    key: &Path,
    app_id: &str,
    installation_id: &str,
    event: &Event<'_, '_>,
) -> Result<()> {
    // Exit if key doesn't exist
    if !key.exists() {
        error!(path = %key.display(), "private key no longer exists");
        return Ok(());
    }

    // Get an authentication token
    let jwt = generate_jwt(key, app_id).await?;
    let token = retrieve_token(client, jwt, installation_id).await?;

    match event {
        Event::Deployment { commit, state } => {
            deployment(client, owner, repo, commit, &token, state).await
        }

        // Ignore any service events
        Event::ServiceUpdate { .. } | Event::ServiceDelete { .. } => {
            debug!("unsupported event, no message sent");
            Ok(())
        }
    }
}

/// Generate a short-lived JWT for the GitHub API
#[instrument(skip(key, app_id))]
async fn generate_jwt(key: &Path, app_id: &str) -> Result<String> {
    let contents = fs::read(key).await?;
    let key = EncodingKey::from_rsa_pem(&contents)?;
    debug!("loaded RSA key");

    let now = now();
    let claims = Claims {
        iss: app_id,
        iat: now,
        exp: now + 30, // expire in 30s
    };

    let token = jsonwebtoken::encode(&Header::new(Algorithm::RS256), &claims, &key)?;
    debug!("generated JWT");

    Ok(token)
}

/// Retrieve an authentication token from the API
#[instrument(skip(client, jwt, installation_id))]
async fn retrieve_token(client: &Client, jwt: String, installation_id: &str) -> Result<String> {
    let url = format!(
        "https://api.github.com/app/installations/{installation_id}/access_tokens",
        installation_id = installation_id
    );
    let response: TokenResponse = client
        .post(url)
        .header(AUTHORIZATION, format!("Bearer {}", jwt))
        .send()
        .await?
        .error_for_status()?
        .json()
        .await?;

    Ok(response.token)
}

/// Send a notification for a deployment
#[instrument(skip(client, state), fields(state = %state))]
async fn deployment(
    client: &Client,
    owner: &str,
    repo: &str,
    commit: &str,
    token: &str,
    state: &State,
) -> Result<()> {
    let url = format!(
        "https://api.github.com/repos/{owner}/{repo}/statuses/{sha}",
        owner = owner,
        repo = repo,
        sha = commit
    );
    client
        .post(url)
        .json(&Request::new(state))
        .header(AUTHORIZATION, format!("token {}", token))
        .send()
        .await?
        .error_for_status()?;

    Ok(())
}

/// Get the current unix timestamp
fn now() -> u64 {
    use std::time::{SystemTime, UNIX_EPOCH};

    SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap()
        .as_secs()
}

/// Claims to be embedded in the JWT
#[derive(Serialize)]
struct Claims<'iss> {
    iat: u64,
    exp: u64,
    iss: &'iss str,
}

/// The response containing the installation authentication token.
#[derive(Deserialize)]
struct TokenResponse {
    token: String,
}

/// The request body to send to GitHub
#[derive(Serialize)]
struct Request<'state, 'description, 'context> {
    state: &'state str,
    #[serde(skip_serializing_if = "Option::is_none")]
    description: Option<&'description str>,
    context: &'context str,
}

impl<'s, 'd, 'r> Request<'s, 'd, 'r> {
    fn new(state: &'d State) -> Self {
        let (state, description) = match state {
            State::InProgress => ("pending", None),
            State::Success => ("success", None),
            State::Failure(e) => ("failure", Some(e.as_str())),
        };

        Self {
            state,
            description,
            context: "wafflemaker",
        }
    }
}

use super::webhooks::Docker;
use bytes::Bytes;
use warp::{http::StatusCode, Rejection, Reply};

/// Handle webhooks from Docker image pushes
pub async fn docker(body: Docker, authorization: String) -> Result<impl Reply, Rejection> {
    Ok(StatusCode::NO_CONTENT)
}

/// Handle webhooks from GitHub repository pushes
pub async fn github(raw_body: Bytes, raw_signature: String) -> Result<impl Reply, Rejection> {
    Ok(StatusCode::NO_CONTENT)
}

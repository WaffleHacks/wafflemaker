use axum::{
    headers::{authorization::Basic, Authorization, HeaderValue},
    http::StatusCode,
};
use ring::hmac;

/// Ensure that the authorization header is correct
pub fn docker(header: Authorization<Basic>, token: &str) -> Result<(), StatusCode> {
    let joined = [header.username(), header.password()].join(":");

    // Check the tokens match
    if joined == token {
        Ok(())
    } else {
        Err(StatusCode::UNAUTHORIZED)
    }
}

/// Ensure that the provided signature from GitHub is valid
pub fn github(
    raw_body: &[u8],
    header: Option<&HeaderValue>,
    secret: &[u8],
) -> Result<(), StatusCode> {
    // Get the header value
    let raw_signature = header
        .map(|h| h.to_str().ok())
        .flatten()
        .ok_or_else(|| StatusCode::UNAUTHORIZED)?;

    // Remove the `sha256=` prefix from the hash
    let signature_hex = raw_signature
        .strip_prefix("sha256=")
        .ok_or_else(|| StatusCode::UNAUTHORIZED)?;
    let signature = hex::decode(signature_hex).map_err(|_| StatusCode::UNAUTHORIZED)?;

    let key = hmac::Key::new(hmac::HMAC_SHA256, secret);

    // Display the expected signature in debug builds
    #[cfg(debug_assertions)]
    tracing::debug!(
        expected = hex::encode(hmac::sign(&key, raw_body).as_ref()),
        got = signature_hex,
        "signature validation"
    );

    // Verify the signature
    hmac::verify(&key, raw_body, &signature).map_err(|_| StatusCode::UNAUTHORIZED)
}

#[cfg(test)]
mod tests {
    use super::{docker, github};
    use axum::headers::Authorization;
    use axum::http::HeaderValue;
    use ring::hmac;
    use std::fs;

    #[test]
    fn validate_docker_authentication() {
        let token = "the-amazing:test-token";

        assert!(docker(Authorization::basic("the-amazing", "test-token"), token).is_ok());
    }

    #[test]
    fn validate_github_signature() {
        let secret = "the-amazing-test-secret".as_bytes();
        let body = fs::read("testdata/webhooks/github-ping.json")
            .expect("failed to read github-ping.json test data");

        let key = hmac::Key::new(hmac::HMAC_SHA256, secret);
        let signature_bytes = hmac::sign(&key, &body);
        let signature = format!("sha256={}", hex::encode(signature_bytes.as_ref()));
        let header = HeaderValue::from_str(&signature).unwrap();

        assert!(github(&body, Some(&header), secret).is_ok());
    }
}

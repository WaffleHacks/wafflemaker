use ring::hmac;

// TODO: use real error type

/// Ensure that the authorization header is correct
pub fn docker(raw_header: String, token: String) -> Result<(), String> {
    // Extract the base 64 portion
    let b64 = raw_header
        .strip_prefix("Basic ")
        .ok_or_else(|| String::from("unauthorized"))?;

    // Parse the base64 encoded header
    let decoded = base64::decode(b64).map_err(|e| e.to_string())?;
    let header = String::from_utf8(decoded).map_err(|e| e.to_string())?;

    // Check the tokens match
    if header != token {
        Err(String::from("unauthorized"))
    } else {
        Ok(())
    }
}

/// Ensure that the provided signature from GitHub is valid
pub fn github(raw_body: &[u8], raw_signature: String, secret: &[u8]) -> Result<(), String> {
    // Remove the `sha256=` prefix from the hash
    let signature_hex = raw_signature
        .strip_prefix("sha256=")
        .ok_or_else(|| String::from("unauthorized"))?;
    let signature = hex::decode(signature_hex).map_err(|e| e.to_string())?;

    let key = hmac::Key::new(hmac::HMAC_SHA256, secret);

    // Display the expected signature in debug builds
    #[cfg(debug_assertions)]
    println!(
        "signature validation: expected \"{}\", got \"{}\"",
        hex::encode(hmac::sign(&key, raw_body).as_ref()),
        signature_hex
    );

    // Verify the signature
    hmac::verify(&key, raw_body, &signature).map_err(|e| e.to_string())
}

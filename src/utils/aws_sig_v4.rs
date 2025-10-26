use axum::http::request::Parts;
use bytes::Bytes;
use itertools::Itertools;
use ring::hmac::{self, Tag};
use sha2::{Digest, Sha256};
use std::fmt::Write;

/// Compute the "Canonical Request"
///
/// https://docs.aws.amazon.com/IAM/latest/UserGuide/reference_sigv-create-signed-request.html#create-canonical-request
pub fn create_canonical_request(signed_headers: &[String], parts: &Parts, body: &Bytes) -> String {
    let method = &parts.method;
    let path = parts.uri.path();
    let canonical_uri = aws_uri_encode(path, false);

    let canonical_query = parts
        .uri
        .query()
        .map(canonicalize_query)
        .unwrap_or_default();

    let mut headers: Vec<(String, String)> = Vec::new();

    for (name, value) in parts.headers.iter() {
        let key = name.as_str().to_ascii_lowercase();
        if !signed_headers.contains(&key) {
            continue;
        }

        let value = match value.to_str() {
            Ok(value) => value.trim().to_string(),
            Err(_error) => continue,
        };

        let existing = headers.iter_mut().find(|(header, _)| header.eq(&key));

        if let Some(existing) = existing {
            existing.1 = value;
        } else {
            headers.push((key, value));
        }
    }

    headers.sort_by(|a, b| a.0.as_bytes().cmp(b.0.as_bytes()));

    let mut canonical_headers = String::new();
    let mut signed_headers = String::new();

    for (name, value) in headers {
        _ = writeln!(&mut canonical_headers, "{name}:{value}");
        _ = write!(&mut signed_headers, "{name};");
    }

    // Get rid of the last separator
    signed_headers.pop();

    let payload_hash = hash_hex(body);

    format!(
        "{method}\n{canonical_uri}\n{canonical_query}\n{canonical_headers}\n{signed_headers}\n{payload_hash}"
    )
}

/// Create a AWS Sigv4 signature
pub fn aws_sig_v4(
    date_yyyymmdd: &str,
    amz_date: &str,
    region: &str,
    service: &str,
    canonical_request: &str,
    access_key_secret: &str,
) -> String {
    let k_secret = format!("AWS4{access_key_secret}");
    let k_date = hmac_sha256(k_secret.as_bytes(), date_yyyymmdd.as_bytes());
    let k_region = hmac_sha256(k_date.as_ref(), region.as_bytes());
    let k_service = hmac_sha256(k_region.as_ref(), service.as_bytes());
    let k_signing = hmac_sha256(k_service.as_ref(), b"aws4_request");

    let credential_scope = format!("{date_yyyymmdd}/{region}/{service}/aws4_request");
    let hashed_canonical_request = hash_hex(canonical_request.as_bytes());

    let string_to_sign =
        format!("AWS4-HMAC-SHA256\n{amz_date}\n{credential_scope}\n{hashed_canonical_request}");

    let signature = hmac_sha256(k_signing.as_ref(), string_to_sign.as_bytes());
    hex::encode(signature)
}

/// Perform a SHA256 hash on a payload returning the hex encoded result
fn hash_hex(payload: &[u8]) -> String {
    let mut hasher = Sha256::new();
    hasher.update(payload);
    hex::encode(hasher.finalize())
}

/// Perform a HMAC + SHA256 on the provided message using the provided key
fn hmac_sha256(key: &[u8], msg: &[u8]) -> Tag {
    let signing_key = hmac::Key::new(hmac::HMAC_SHA256, key);
    hmac::sign(&signing_key, msg)
}

/// Create a "CanonicalQueryString" from the provided query string
fn canonicalize_query(query: &str) -> String {
    let mut pairs: Vec<(&str, &str)> = query
        .split('&')
        .filter_map(|kv| {
            let mut parts = kv.splitn(2, '=');
            let k = parts.next()?;
            let v = parts.next().unwrap_or("");
            Some((k, v))
        })
        .collect();

    pairs.sort_by(|a, b| a.0.as_bytes().cmp(b.0.as_bytes()));

    pairs
        .into_iter()
        .map(|(k, v)| {
            let k = aws_uri_encode(k, true);
            let v = aws_uri_encode(v, true);
            format!("{k}={v}")
        })
        .join("&")
}

/// URL encode using the custom AWS url encoding
fn aws_uri_encode(input: &str, encode_slash: bool) -> String {
    let mut output = String::new();
    for b in input.bytes() {
        match b {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'-' | b'.' | b'_' | b'~' => {
                output.push(b as char)
            }
            b'/' if !encode_slash => output.push('/'),
            _ => output.push_str(&format!("%{:02X}", b)), // Uppercase hex
        }
    }
    output
}

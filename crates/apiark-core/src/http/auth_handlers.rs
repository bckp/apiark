use hmac::{Hmac, Mac};
use sha2::Digest;
use sha2::Sha256;

/// Compute AWS Signature v4 Authorization header.
pub fn compute_aws_v4_auth(
    access_key: &str,
    secret_key: &str,
    session_token: &str,
    region: &str,
    service: &str,
    method: &str,
    url: &url::Url,
    headers: &[(String, String)],
    body: &str,
    now: &chrono::DateTime<chrono::Utc>,
) -> (String, Vec<(String, String)>) {
    let date_stamp = now.format("%Y%m%d").to_string();
    let amz_date = now.format("%Y%m%dT%H%M%SZ").to_string();

    let host = url.host_str().unwrap_or("");
    let canonical_uri = url.path();
    let canonical_querystring = url.query().unwrap_or("");

    // Build signed headers
    let mut signed_headers: Vec<(String, String)> = vec![
        ("host".to_string(), host.to_string()),
        ("x-amz-date".to_string(), amz_date.clone()),
    ];
    if !session_token.is_empty() {
        signed_headers.push((
            "x-amz-security-token".to_string(),
            session_token.to_string(),
        ));
    }
    // Include user headers
    for (k, v) in headers {
        let lower = k.to_lowercase();
        if lower != "host"
            && lower != "x-amz-date"
            && lower != "authorization"
            && lower != "x-amz-security-token"
        {
            signed_headers.push((lower, v.clone()));
        }
    }
    signed_headers.sort_by(|a, b| a.0.cmp(&b.0));

    let signed_headers_str = signed_headers
        .iter()
        .map(|(k, _)| k.as_str())
        .collect::<Vec<_>>()
        .join(";");

    let canonical_headers = signed_headers
        .iter()
        .map(|(k, v)| format!("{k}:{}\n", v.trim()))
        .collect::<String>();

    let payload_hash = sha256_hex(body);

    let canonical_request = format!(
        "{method}\n{canonical_uri}\n{canonical_querystring}\n{canonical_headers}\n{signed_headers_str}\n{payload_hash}"
    );

    let credential_scope = format!("{date_stamp}/{region}/{service}/aws4_request");
    let string_to_sign = format!(
        "AWS4-HMAC-SHA256\n{amz_date}\n{credential_scope}\n{}",
        sha256_hex(&canonical_request)
    );

    // Derive signing key
    let k_date = hmac_sha256(
        format!("AWS4{secret_key}").as_bytes(),
        date_stamp.as_bytes(),
    );
    let k_region = hmac_sha256(&k_date, region.as_bytes());
    let k_service = hmac_sha256(&k_region, service.as_bytes());
    let k_signing = hmac_sha256(&k_service, b"aws4_request");

    let signature = hex::encode(hmac_sha256(&k_signing, string_to_sign.as_bytes()));

    let auth_header = format!(
        "AWS4-HMAC-SHA256 Credential={access_key}/{credential_scope}, SignedHeaders={signed_headers_str}, Signature={signature}"
    );

    // Extra headers to add
    let mut extra_headers = vec![
        ("x-amz-date".to_string(), amz_date),
        ("x-amz-content-sha256".to_string(), payload_hash),
    ];
    if !session_token.is_empty() {
        extra_headers.push((
            "x-amz-security-token".to_string(),
            session_token.to_string(),
        ));
    }

    (auth_header, extra_headers)
}

/// Generate a JWT token.
pub fn generate_jwt(secret: &str, algorithm: &str, payload_json: &str) -> Result<String, String> {
    use jsonwebtoken::{encode, Algorithm, EncodingKey, Header};

    let alg = match algorithm.to_uppercase().as_str() {
        "HS256" => Algorithm::HS256,
        "HS384" => Algorithm::HS384,
        "HS512" => Algorithm::HS512,
        "RS256" => Algorithm::RS256,
        "RS384" => Algorithm::RS384,
        "RS512" => Algorithm::RS512,
        "ES256" => Algorithm::ES256,
        "ES384" => Algorithm::ES384,
        other => return Err(format!("Unsupported JWT algorithm: {other}")),
    };

    let claims: serde_json::Value =
        serde_json::from_str(payload_json).map_err(|e| format!("Invalid JWT payload JSON: {e}"))?;

    let header = Header::new(alg);

    let key = match alg {
        Algorithm::HS256 | Algorithm::HS384 | Algorithm::HS512 => {
            EncodingKey::from_secret(secret.as_bytes())
        }
        Algorithm::RS256 | Algorithm::RS384 | Algorithm::RS512 => {
            EncodingKey::from_rsa_pem(secret.as_bytes())
                .map_err(|e| format!("Invalid RSA PEM key: {e}"))?
        }
        Algorithm::ES256 | Algorithm::ES384 => EncodingKey::from_ec_pem(secret.as_bytes())
            .map_err(|e| format!("Invalid EC PEM key: {e}"))?,
        _ => return Err(format!("Unsupported algorithm: {algorithm}")),
    };

    encode(&header, &claims, &key).map_err(|e| format!("Failed to generate JWT: {e}"))
}

fn sha256_hex(input: &str) -> String {
    let mut hasher = Sha256::new();
    hasher.update(input.as_bytes());
    hex::encode(hasher.finalize())
}

fn hmac_sha256(key: &[u8], data: &[u8]) -> Vec<u8> {
    let mut mac = Hmac::<Sha256>::new_from_slice(key).expect("HMAC can take key of any size");
    mac.update(data);
    mac.finalize().into_bytes().to_vec()
}

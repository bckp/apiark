use crate::models::error::HttpEngineError;
use crate::models::request::KeyValuePair;
use crate::models::response::{CookieData, ResponseData};

/// Maximum response body size we'll load into memory: 10MB.
const MAX_BODY_SIZE: u64 = 10 * 1024 * 1024;

/// Parse a reqwest::Response into our ResponseData struct.
pub async fn parse_response(
    response: reqwest::Response,
    elapsed_ms: u64,
) -> Result<ResponseData, HttpEngineError> {
    let status = response.status().as_u16();
    let status_text = response
        .status()
        .canonical_reason()
        .unwrap_or("Unknown")
        .to_string();

    // Collect headers
    let headers: Vec<KeyValuePair> = response
        .headers()
        .iter()
        .map(|(name, value)| {
            KeyValuePair::new(
                name.to_string(),
                value.to_str().unwrap_or("<binary>").to_string(),
                true,
            )
        })
        .collect();

    let cookies = response
        .headers()
        .get_all("set-cookie")
        .iter()
        .filter_map(|h| h.to_str().ok())
        .filter_map(|h| parse_set_cookie_header(h, response.url()))
        .collect();

    // Check content-length before downloading
    if let Some(len) = response.content_length() {
        if len > MAX_BODY_SIZE {
            return Err(HttpEngineError::ResponseTooLarge(MAX_BODY_SIZE));
        }
    }

    // Enforce the cap while receiving chunked/unknown-length responses.
    let mut response = response;
    let mut bytes = Vec::new();
    while let Some(chunk) = response
        .chunk()
        .await
        .map_err(|e| HttpEngineError::BodyDecodeError(e.to_string()))?
    {
        if chunk.len() > MAX_BODY_SIZE as usize - bytes.len() {
            return Err(HttpEngineError::ResponseTooLarge(MAX_BODY_SIZE));
        }
        bytes.extend_from_slice(&chunk);
    }

    let size_bytes = bytes.len() as u64;

    let content_type = headers
        .iter()
        .find(|h| h.key.eq_ignore_ascii_case("content-type"))
        .map(|h| {
            h.value
                .split(';')
                .next()
                .unwrap_or("")
                .trim()
                .to_ascii_lowercase()
        })
        .unwrap_or_default();
    let is_text = content_type.is_empty()
        || content_type.starts_with("text/")
        || content_type.ends_with("json")
        || content_type.ends_with("xml")
        || content_type == "application/javascript"
        || content_type == "application/x-www-form-urlencoded";
    let (body, body_base64) = match String::from_utf8(bytes) {
        Ok(text) if is_text => (text, None),
        result => {
            use base64::Engine;
            let bytes = match result {
                Ok(text) => text.into_bytes(),
                Err(error) => error.into_bytes(),
            };
            (
                format!("<binary data: {size_bytes} bytes>"),
                Some(base64::engine::general_purpose::STANDARD.encode(bytes)),
            )
        }
    };

    Ok(ResponseData {
        status,
        status_text,
        headers,
        cookies,
        body,
        body_base64,
        time_ms: elapsed_ms,
        size_bytes,
        truncated: None,
        full_size: None,
        temp_path: None,
    })
}

/// Parse a raw Set-Cookie header string into a CookieData.
pub fn parse_set_cookie_header(header: &str, url: &url::Url) -> Option<CookieData> {
    let c = cookie::Cookie::parse(header).ok()?;
    let host = url.host_str()?;
    let domain = c
        .domain()
        .unwrap_or(host)
        .trim_start_matches('.')
        .to_ascii_lowercase();
    let host_only = c.domain().is_none();
    if domain.is_empty()
        || (host != domain
            && (host.parse::<std::net::IpAddr>().is_ok()
                || !host.strip_suffix(&domain).is_some_and(|p| p.ends_with('.'))))
    {
        return None;
    }
    if !host_only && psl::suffix_str(&domain) == Some(domain.as_str()) && domain != host {
        return None;
    }
    // A public-suffix Domain exactly equal to the origin is treated as host-only.
    let host_only = host_only || psl::suffix_str(&domain) == Some(domain.as_str());
    let default_path = url
        .path()
        .rsplit_once('/')
        .map(|(p, _)| if p.is_empty() { "/" } else { p })
        .unwrap_or("/");
    let path = c
        .path()
        .filter(|p| p.starts_with('/'))
        .unwrap_or(default_path);
    let expires = if let Some(age) = c.max_age() {
        Some(
            chrono::Utc::now()
                .timestamp()
                .saturating_add(age.whole_seconds().max(-1)),
        )
    } else {
        c.expires_datetime().map(|t| t.unix_timestamp())
    };
    Some(CookieData {
        name: c.name().to_string(),
        value: c.value().to_string(),
        domain: Some(domain),
        host_only,
        path: Some(path.to_string()),
        expires: expires
            .and_then(|t| chrono::DateTime::from_timestamp(t, 0))
            .map(|t| t.to_rfc3339()),
        http_only: c.http_only().unwrap_or(false),
        secure: c.secure().unwrap_or(false),
    })
}

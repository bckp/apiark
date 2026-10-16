use super::{
    cookies::{CookieEntry, CookieJarManager},
    error_classifier::classify_reqwest_error,
    request_builder::{build_client, build_request},
    response_parser::{parse_response, parse_set_cookie_header},
};
use crate::models::{
    collection::CollectionDefaults,
    error::HttpEngineError,
    request::{HttpMethod, SendRequestParams},
    response::ResponseData,
};
use std::time::Instant;

pub struct HttpEngine;

impl HttpEngine {
    pub async fn send(params: SendRequestParams) -> Result<ResponseData, HttpEngineError> {
        Self::send_with_jar(
            params,
            &CookieJarManager::new(),
            "",
            &CollectionDefaults::default(),
        )
        .await
    }

    pub async fn send_with_jar(
        mut params: SendRequestParams,
        jar: &CookieJarManager,
        key: &str,
        defaults: &CollectionDefaults,
    ) -> Result<ResponseData, HttpEngineError> {
        let timeout_ms = params.timeout_ms.unwrap_or(30_000);
        let follow = params.follow_redirects;
        params.follow_redirects = false;
        let client = build_client(&params)?;
        let start = Instant::now();
        let mut all_cookies = Vec::new();
        let work = async {
            for hop in 0..=10 {
                let url = super::request_builder::build_url_with_params(
                    &params.url,
                    &params.params,
                    &params.auth,
                )?;
                if !matches!(url.scheme(), "http" | "https")
                    || !url.username().is_empty()
                    || url.password().is_some()
                {
                    return Err(HttpEngineError::InvalidUrl(
                        "Only HTTP(S) URLs without credentials are supported".into(),
                    ));
                }
                let mut outgoing = params.clone();
                let mut explicit = std::collections::HashMap::new();
                for header in outgoing
                    .headers
                    .iter()
                    .filter(|h| h.enabled && h.key.eq_ignore_ascii_case("cookie"))
                {
                    for part in header.value.split(';') {
                        if let Some((name, value)) = part.trim().split_once('=') {
                            explicit.insert(name.to_string(), value.to_string());
                        }
                    }
                }
                explicit.extend(outgoing.cookies.take().unwrap_or_default());
                outgoing
                    .headers
                    .retain(|h| !h.key.eq_ignore_ascii_case("cookie"));
                let mut pairs = Vec::new();
                if defaults.send_cookies {
                    let mut stored = jar
                        .get_cookies(key)
                        .map_err(HttpEngineError::RequestError)?;
                    stored.retain(|c| c.matches(&url) && !explicit.contains_key(&c.name));
                    stored.sort_by_key(|c| std::cmp::Reverse(c.path.len()));
                    pairs.extend(
                        stored
                            .into_iter()
                            .map(|c| format!("{}={}", c.name, c.value)),
                    );
                }
                pairs.extend(explicit.into_iter().map(|(k, v)| format!("{k}={v}")));
                if !pairs.is_empty() {
                    outgoing
                        .headers
                        .push(crate::models::request::KeyValuePair::new(
                            "Cookie".into(),
                            pairs.join("; "),
                            true,
                        ));
                }
                let response = build_request(&client, &outgoing)?
                    .send()
                    .await
                    .map_err(|e| classify_reqwest_error(e, timeout_ms))?;
                let cookies: Vec<_> = response
                    .headers()
                    .get_all("set-cookie")
                    .iter()
                    .filter_map(|v| v.to_str().ok())
                    .filter_map(|s| parse_set_cookie_header(s, response.url()))
                    .collect();
                if defaults.store_cookies {
                    jar.store_cookies(key, cookies.iter().map(CookieEntry::from).collect())
                        .map_err(HttpEngineError::RequestError)?;
                }
                all_cookies.extend(cookies);
                let status = response.status().as_u16();
                let redirect = matches!(status, 301 | 302 | 303 | 307 | 308) && follow;
                let location = response
                    .headers()
                    .get("location")
                    .and_then(|h| h.to_str().ok());
                if !redirect || location.is_none() {
                    let mut result =
                        parse_response(response, start.elapsed().as_millis() as u64).await?;
                    result.time_ms = start.elapsed().as_millis() as u64;
                    result.cookies = all_cookies;
                    return Ok(result);
                }
                if hop == 10 {
                    return Err(HttpEngineError::RequestError("Too many redirects".into()));
                }
                let next = response
                    .url()
                    .join(location.unwrap())
                    .map_err(|e| HttpEngineError::InvalidUrl(e.to_string()))?;
                if next.origin() != url.origin() {
                    params.auth = None;
                    params.cookies = None;
                    params.headers.clear();
                }
                if status == 303 && !matches!(params.method, HttpMethod::HEAD)
                    || matches!(status, 301 | 302) && matches!(params.method, HttpMethod::POST)
                {
                    params.method = HttpMethod::GET;
                    params.body = None;
                    params.headers.retain(|h| {
                        !matches!(
                            h.key.to_ascii_lowercase().as_str(),
                            "content-type" | "content-length" | "transfer-encoding"
                        )
                    });
                }
                // Rebuild typed bodies, including multipart, for 307/308.
                params.url = next.to_string();
                params.params.clear();
            }
            unreachable!()
        };
        let result = tokio::time::timeout(std::time::Duration::from_millis(timeout_ms), work)
            .await
            .map_err(|_| HttpEngineError::Timeout(timeout_ms))?;
        if defaults.persist_cookies && defaults.store_cookies {
            jar.save_to_disk(key)
                .map_err(HttpEngineError::RequestError)?;
        }
        result
    }
}

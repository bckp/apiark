//! History stores request structure, never credential-bearing field values.
use crate::models::request::SendRequestParams;

pub fn history_url(input: &str) -> String {
    url::Url::parse(input)
        .map(|u| format!("{}/[REDACTED]", u.origin().ascii_serialization()))
        .unwrap_or_else(|_| "[REDACTED]".into())
}

pub fn history_request(params: &SendRequestParams) -> String {
    let mut value = serde_json::to_value(params).unwrap_or(serde_json::Value::Null);
    fn scrub(value: &mut serde_json::Value) {
        match value {
            serde_json::Value::Object(map) => map.values_mut().for_each(scrub),
            serde_json::Value::Array(items) => items.iter_mut().for_each(scrub),
            serde_json::Value::String(s) => *s = "[REDACTED]".into(),
            _ => {}
        }
    }
    scrub(&mut value);
    value["method"] = serde_json::json!(params.method);
    value["url"] = serde_json::json!(history_url(&params.url));
    serde_json::to_string(&value).unwrap_or_else(|_| "{}".into())
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn masks_headers_custom_api_keys_bodies_cookies_and_all_url_values() {
        let request: SendRequestParams = serde_json::from_value(serde_json::json!({
            "method":"POST", "url":"https://user:pass@example.test/path-secret?custom=query-secret#fragment-secret",
            "headers":[{"key":"Authorization","value":"Bearer header-secret"},{"key":"X-Custom","value":"custom-secret"}],
            "params":[{"key":"anything","value":"param-secret"}],
            "body":{"type":"json","content":"body-secret"},
            "auth":{"type":"api-key","key":"X-Anything","value":"api-secret","addTo":"header"},
            "cookies":{"sid":"cookie-secret","id":"id-secret","key":"key-secret"}
        })).unwrap();
        let json = history_request(&request);
        for secret in [
            "user:",
            "pass@",
            "path-secret",
            "query-secret",
            "fragment-secret",
            "header-secret",
            "custom-secret",
            "param-secret",
            "body-secret",
            "api-secret",
            "cookie-secret",
            "id-secret",
            "key-secret",
        ] {
            assert!(!json.contains(secret), "{secret}");
        }
        assert_eq!(history_url(&request.url), "https://example.test/[REDACTED]");
        assert!(serde_json::from_str::<serde_json::Value>(&json).is_ok());
    }
}

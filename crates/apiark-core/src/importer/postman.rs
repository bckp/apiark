use std::collections::HashMap;

use serde_json::Value;

use super::{ImportBody, ImportData, ImportEnvironment, ImportItem, ImportWarning};
use crate::models::auth::AuthConfig;

/// Parse a Postman Collection v2.1 JSON string into ImportData.
pub fn parse_postman(content: &str) -> Result<ImportData, String> {
    let root: Value = serde_json::from_str(content).map_err(|e| format!("Invalid JSON: {e}"))?;

    let info = root
        .get("info")
        .ok_or("Missing 'info' field — is this a Postman v2.1 collection?")?;
    if !root.get("item").is_some_and(Value::is_array) {
        return Err("Postman collection requires an item array".into());
    }

    let collection_name = info
        .get("name")
        .and_then(|v| v.as_str())
        .unwrap_or("Imported Collection")
        .to_string();

    let mut warnings = Vec::new();
    audit(&root, &mut warnings)?;
    let mut metadata = root.clone();
    metadata
        .as_object_mut()
        .ok_or("Collection must be an object")?
        .remove("item");

    let items = root
        .get("item")
        .and_then(|v| v.as_array())
        .map(|arr| parse_items(arr, &mut warnings))
        .unwrap_or_default();

    // Collection-level variables → environment
    let mut environments = Vec::new();
    if let Some(vars) = root.get("variable").and_then(|v| v.as_array()) {
        let mut variables = HashMap::new();
        for var in vars
            .iter()
            .filter(|v| v.get("disabled").and_then(Value::as_bool) != Some(true))
        {
            if let (Some(key), Some(value)) = (
                var.get("key").and_then(|v| v.as_str()),
                var.get("value").map(|v| {
                    v.as_str()
                        .map(str::to_string)
                        .unwrap_or_else(|| v.to_string())
                }),
            ) {
                variables.insert(key.to_string(), value.to_string());
            }
        }
        if !variables.is_empty() {
            environments.push(ImportEnvironment {
                name: "Collection Variables".to_string(),
                variables,
            });
        }
    }

    Ok(ImportData {
        postman: Some(metadata),
        collection_name,
        items,
        environments,
        warnings,
    })
}

fn parse_items(items: &[Value], warnings: &mut Vec<ImportWarning>) -> Vec<ImportItem> {
    items
        .iter()
        .filter_map(|item| parse_item(item, warnings))
        .collect()
}

fn parse_item(item: &Value, warnings: &mut Vec<ImportWarning>) -> Option<ImportItem> {
    let name = item
        .get("name")
        .and_then(|v| v.as_str())
        .unwrap_or("Untitled")
        .to_string();

    // If it has nested "item", it's a folder
    if let Some(children) = item.get("item").and_then(|v| v.as_array()) {
        return Some(ImportItem::Folder {
            postman: Some(item.clone()),
            name,
            items: parse_items(children, warnings),
        });
    }

    // Otherwise it should have a "request" field
    let raw_request = item.get("request")?;
    let normalized;
    let req = if raw_request.is_string() {
        normalized = serde_json::json!({"method":"GET","url":raw_request});
        &normalized
    } else {
        raw_request
    };

    let method = req
        .get("method")
        .and_then(|v| v.as_str())
        .unwrap_or("GET")
        .to_uppercase();

    let (url, path_variables) = parse_url(req.get("url"));

    let headers = parse_headers(req.get("header"));

    let body = parse_body(req.get("body"));

    let auth = parse_auth(req.get("auth"));

    let description = req
        .get("description")
        .and_then(|v| v.as_str())
        .map(|s| s.to_string());

    // Scripts from event array
    let mut pre_request_script = None;
    let post_response_script = None;
    let mut tests = None;

    if let Some(events) = item.get("event").and_then(|v| v.as_array()) {
        for event in events {
            let listen = event.get("listen").and_then(|v| v.as_str()).unwrap_or("");
            let script_src = event
                .get("script")
                .and_then(|s| s.get("exec"))
                .and_then(|e| {
                    if let Some(arr) = e.as_array() {
                        Some(
                            arr.iter()
                                .filter_map(|l| l.as_str())
                                .collect::<Vec<_>>()
                                .join("\n"),
                        )
                    } else {
                        e.as_str().map(|s| s.to_string())
                    }
                });

            if let Some(src) = script_src {
                if src.trim().is_empty() {
                    continue;
                }
                // Warn about pm.* API usage
                if src.contains("pm.") {
                    warnings.push(ImportWarning {
                        item_name: name.clone(),
                        message: "Script uses Postman's pm.* API. Replace with ark.* equivalents."
                            .to_string(),
                    });
                }
                match listen {
                    "prerequest" => {
                        let previous = pre_request_script.take().unwrap_or_default();
                        pre_request_script = Some(format!("{previous}\n{src}"));
                    }
                    "test" => {
                        let previous = tests.take().unwrap_or_default();
                        tests = Some(format!("{previous}\n{src}"));
                    }
                    _ => {}
                }
            }
        }
    }

    Some(ImportItem::Request {
        postman: Some(item.clone()),
        name,
        method,
        url,
        headers,
        params: if path_variables.is_empty() {
            None
        } else {
            Some(path_variables)
        },
        body: Box::new(body),
        auth: Box::new(auth),
        description,
        pre_request_script,
        post_response_script,
        tests,
    })
}

pub(crate) fn parse_url(url_val: Option<&Value>) -> (String, HashMap<String, String>) {
    let empty_vars = HashMap::new();
    match url_val {
        None => (String::new(), empty_vars),
        Some(v) if v.is_string() => (v.as_str().unwrap_or("").to_string(), empty_vars),
        Some(v) => {
            // Extract path variables from url.variable[]
            let path_variables: HashMap<String, String> = v
                .get("variable")
                .and_then(|vars| vars.as_array())
                .map(|arr| {
                    let mut map = HashMap::new();
                    for var in arr {
                        if let Some(key) = var.get("key").and_then(|k| k.as_str()) {
                            let value = var
                                .get("value")
                                .and_then(|v| v.as_str())
                                .unwrap_or("")
                                .to_string();
                            map.insert(key.to_string(), value);
                        }
                    }
                    map
                })
                .unwrap_or_default();

            // Postman URL object: { raw, host, path, query, ... }
            let url = if let Some(raw) = v.get("raw").and_then(|r| r.as_str()) {
                raw.to_string()
            } else {
                // Reconstruct from parts
                let host = v
                    .get("host")
                    .and_then(Value::as_str)
                    .map(str::to_string)
                    .unwrap_or_else(|| {
                        v.get("host")
                            .and_then(|h| h.as_array())
                            .map(|parts| {
                                parts
                                    .iter()
                                    .filter_map(|p| p.as_str())
                                    .collect::<Vec<_>>()
                                    .join(".")
                            })
                            .unwrap_or_default()
                    });

                let path = v
                    .get("path")
                    .and_then(Value::as_str)
                    .map(str::to_string)
                    .unwrap_or_else(|| {
                        v.get("path")
                            .and_then(|p| p.as_array())
                            .map(|parts| {
                                parts
                                    .iter()
                                    .filter_map(|p| p.as_str())
                                    .collect::<Vec<_>>()
                                    .join("/")
                            })
                            .unwrap_or_default()
                    });

                let protocol = v
                    .get("protocol")
                    .and_then(|p| p.as_str())
                    .unwrap_or("https");

                let port = v
                    .get("port")
                    .and_then(Value::as_str)
                    .map(|p| format!(":{p}"))
                    .unwrap_or_default();
                format!("{protocol}://{host}{port}/{}", path.trim_start_matches('/'))
            };

            // Keep :paramName placeholders in the URL — path variable values are
            // stored separately in the params field so the UI can display and edit them.
            let url = if let Some(query) = v.get("query").and_then(|q| q.as_array()) {
                let mut serializer = url::form_urlencoded::Serializer::new(String::new());
                for entry in query
                    .iter()
                    .filter(|e| e.get("disabled").and_then(Value::as_bool) != Some(true))
                {
                    if let Some(key) = entry.get("key").and_then(Value::as_str) {
                        serializer.append_pair(
                            key,
                            entry.get("value").and_then(Value::as_str).unwrap_or(""),
                        );
                    }
                }
                let query = serializer
                    .finish()
                    .replace("%7B%7B", "{{")
                    .replace("%7D%7D", "}}");
                let raw = url.split_once('?').map(|(base, _)| base).unwrap_or(&url);
                if query.is_empty() {
                    raw.to_string()
                } else {
                    format!("{raw}?{query}")
                }
            } else {
                url
            };
            (url, path_variables)
        }
    }
}

pub(crate) fn parse_headers(headers_val: Option<&Value>) -> HashMap<String, String> {
    let mut map = HashMap::new();
    if let Some(arr) = headers_val.and_then(|v| v.as_array()) {
        for h in arr {
            if let (Some(key), Some(value)) = (
                h.get("key").and_then(|v| v.as_str()),
                h.get("value").and_then(|v| v.as_str()),
            ) {
                // Skip disabled headers
                if h.get("disabled").and_then(|v| v.as_bool()).unwrap_or(false) {
                    continue;
                }
                map.retain(|existing: &String, _| !existing.eq_ignore_ascii_case(key));
                map.insert(key.to_string(), value.to_string());
            }
        }
    }
    map
}

pub(crate) fn parse_body(body_val: Option<&Value>) -> Option<ImportBody> {
    let body = body_val?;
    if body.get("disabled").and_then(Value::as_bool) == Some(true) {
        return None;
    }
    let mode = body.get("mode").and_then(|v| v.as_str())?;

    match mode {
        "raw" => {
            let content = body.get("raw").and_then(|v| v.as_str()).unwrap_or("");
            // Check language option for body type
            let lang = body
                .get("options")
                .and_then(|o| o.get("raw"))
                .and_then(|r| r.get("language"))
                .and_then(|l| l.as_str())
                .unwrap_or("text");

            let body_type = match lang {
                "json" => "json",
                "xml" => "xml",
                _ => "raw",
            };

            Some(ImportBody {
                body_type: body_type.to_string(),
                content: content.to_string(),
            })
        }
        "urlencoded" => {
            let pairs = body.get("urlencoded").and_then(|v| v.as_array());
            let content = pairs
                .map(|arr| {
                    arr.iter()
                        .filter(|p| p.get("disabled").and_then(Value::as_bool) != Some(true))
                        .filter_map(|p| {
                            let key = p.get("key").and_then(|v| v.as_str())?;
                            let val = p.get("value").and_then(|v| v.as_str()).unwrap_or("");
                            Some((key.to_string(), val.to_string()))
                        })
                        .collect::<Vec<_>>()
                        .into_iter()
                        .fold(
                            url::form_urlencoded::Serializer::new(String::new()),
                            |mut s, (k, v)| {
                                s.append_pair(&k, &v);
                                s
                            },
                        )
                        .finish()
                })
                .unwrap_or_default();

            Some(ImportBody {
                body_type: "urlencoded".to_string(),
                content,
            })
        }
        "formdata" => {
            let content =
                serde_json::to_string(body.get("formdata").unwrap_or(&Value::Array(vec![])))
                    .unwrap_or_default();
            Some(ImportBody {
                body_type: "form-data".to_string(),
                content,
            })
        }
        _ => None,
    }
}

pub(crate) fn parse_auth(auth_val: Option<&Value>) -> Option<AuthConfig> {
    let auth = auth_val?;
    let auth_type = auth.get("type").and_then(|v| v.as_str())?;

    match auth_type {
        "bearer" => {
            let token = auth
                .get("bearer")
                .and_then(|b| b.as_array())
                .and_then(|arr| {
                    arr.iter().find_map(|item| {
                        if item.get("key").and_then(|v| v.as_str()) == Some("token") {
                            item.get("value")
                                .and_then(|v| v.as_str())
                                .map(|s| s.to_string())
                        } else {
                            None
                        }
                    })
                })
                .unwrap_or_default();

            Some(AuthConfig::Bearer { token })
        }
        "basic" => {
            let get_field = |field: &str| -> String {
                auth.get("basic")
                    .and_then(|b| b.as_array())
                    .and_then(|arr| {
                        arr.iter().find_map(|item| {
                            if item.get("key").and_then(|v| v.as_str()) == Some(field) {
                                item.get("value")
                                    .and_then(|v| v.as_str())
                                    .map(|s| s.to_string())
                            } else {
                                None
                            }
                        })
                    })
                    .unwrap_or_default()
            };

            Some(AuthConfig::Basic {
                username: get_field("username"),
                password: get_field("password"),
            })
        }
        "apikey" => {
            let get_field = |field: &str| -> String {
                auth.get("apikey")
                    .and_then(|b| b.as_array())
                    .and_then(|arr| {
                        arr.iter().find_map(|item| {
                            if item.get("key").and_then(|v| v.as_str()) == Some(field) {
                                item.get("value")
                                    .and_then(|v| v.as_str())
                                    .map(|s| s.to_string())
                            } else {
                                None
                            }
                        })
                    })
                    .unwrap_or_default()
            };

            let add_to = match get_field("in").as_str() {
                "query" => crate::models::auth::ApiKeyLocation::Query,
                _ => crate::models::auth::ApiKeyLocation::Header,
            };

            Some(AuthConfig::ApiKey {
                key: get_field("key"),
                value: get_field("value"),
                add_to,
            })
        }
        "noauth" | "" => Some(AuthConfig::None),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_path_variable_preserved() {
        let result = parse_postman(r#"{
            "info": { "name": "Test", "schema": "https://schema.getpostman.com/json/collection/v2.1.0/collection.json" },
            "item": [{
                "name": "Order by name",
                "request": {
                    "method": "GET",
                    "url": {
                        "raw": "{{hostUrl}}/order/name/:name",
                        "host": ["{{hostUrl}}"],
                        "path": ["order", "name", ":name"],
                        "variable": [{ "key": "name", "value": "Mujo Hasic" }]
                    }
                }
            }]
        }"#).unwrap();

        match &result.items[0] {
            ImportItem::Request { url, params, .. } => {
                assert_eq!(
                    url, "{{hostUrl}}/order/name/:name",
                    "URL should keep :placeholder"
                );
                let p = params
                    .as_ref()
                    .expect("params should contain path variables");
                assert_eq!(p.get("name").unwrap(), "Mujo Hasic");
            }
            _ => panic!("expected request"),
        }
    }

    #[test]
    fn test_multiple_path_variables() {
        let result = parse_postman(
            r#"{
            "info": { "name": "Test", "schema": "..." },
            "item": [{
                "name": "Get post",
                "request": {
                    "method": "GET",
                    "url": {
                        "raw": "{{hostUrl}}/users/:userId/posts/:postId",
                        "host": ["{{hostUrl}}"],
                        "path": ["users", ":userId", "posts", ":postId"],
                        "variable": [
                            { "key": "userId", "value": "123" },
                            { "key": "postId", "value": "456" }
                        ]
                    }
                }
            }]
        }"#,
        )
        .unwrap();

        match &result.items[0] {
            ImportItem::Request { url, params, .. } => {
                assert_eq!(
                    url, "{{hostUrl}}/users/:userId/posts/:postId",
                    "URL should keep :placeholders"
                );
                let p = params
                    .as_ref()
                    .expect("params should contain path variables");
                assert_eq!(p.get("userId").unwrap(), "123");
                assert_eq!(p.get("postId").unwrap(), "456");
            }
            _ => panic!("expected request"),
        }
    }

    #[test]
    fn test_empty_path_variable_keeps_placeholder() {
        let result = parse_postman(
            r#"{
            "info": { "name": "Test", "schema": "..." },
            "item": [{
                "name": "Get item",
                "request": {
                    "method": "GET",
                    "url": {
                        "raw": "{{hostUrl}}/items/:id",
                        "host": ["{{hostUrl}}"],
                        "path": ["items", ":id"],
                        "variable": [{ "key": "id", "value": "" }]
                    }
                }
            }]
        }"#,
        )
        .unwrap();

        match &result.items[0] {
            ImportItem::Request { url, params, .. } => {
                assert_eq!(
                    url, "{{hostUrl}}/items/:id",
                    "empty value should keep :placeholder"
                );
                // Empty path variable value is still stored
                let p = params
                    .as_ref()
                    .expect("params should contain path variables");
                assert_eq!(p.get("id").unwrap(), "");
            }
            _ => panic!("expected request"),
        }
    }

    #[test]
    fn test_no_path_variables() {
        let result = parse_postman(
            r#"{
            "info": { "name": "Test", "schema": "..." },
            "item": [{
                "name": "Health",
                "request": {
                    "method": "GET",
                    "url": {
                        "raw": "{{hostUrl}}/health",
                        "host": ["{{hostUrl}}"],
                        "path": ["health"]
                    }
                }
            }]
        }"#,
        )
        .unwrap();

        match &result.items[0] {
            ImportItem::Request { url, params, .. } => {
                assert_eq!(url, "{{hostUrl}}/health");
                assert!(params.is_none());
            }
            _ => panic!("expected request"),
        }
    }
}

fn audit(value: &Value, warnings: &mut Vec<ImportWarning>) -> Result<(), String> {
    let name = value
        .get("name")
        .or_else(|| value.get("info").and_then(|v| v.get("name")))
        .and_then(Value::as_str)
        .unwrap_or("Collection");
    let mut warn = |message: &str| {
        warnings.push(ImportWarning {
            item_name: name.into(),
            message: message.into(),
        })
    };
    let req = value.get("request").unwrap_or(value);
    if req.get("header").is_some_and(|v| !v.is_array()) {
        return Err(format!("{name}: Raw string headers are unsupported; convert to header objects before importing"));
    }
    if value.get("info").is_none() && value.get("item").is_none() && value.get("request").is_none()
    {
        return Err(format!(
            "{name}: Missing request or folder items; import aborted"
        ));
    }
    if let Some(body) = req.get("body") {
        if body
            .get("urlencoded")
            .and_then(Value::as_array)
            .is_some_and(|a| {
                a.iter()
                    .any(|v| v.get("disabled").and_then(Value::as_bool) == Some(true))
            })
        {
            warn("Disabled form fields are retained for export and are not sent");
        }
    }
    if let Some(auth) = req.get("auth") {
        let kind = auth.get("type").and_then(Value::as_str).unwrap_or("");
        if !matches!(
            kind,
            "" | "noauth" | "bearer" | "basic" | "apikey" | "digest" | "awsv4" | "oauth2" | "ntlm"
        ) {
            return Err(format!(
                "{name}: Unsupported authentication {kind}; import aborted without dropping it"
            ));
        }
        if matches!(kind, "digest" | "oauth2" | "ntlm" | "awsv4")
            && parse_auth(Some(auth)).is_none()
        {
            return Err(format!("{name}: Authentication {kind} is not supported by this importer; use Bearer/Basic/API key"));
        }
    }
    if let Some(method) = req.get("method").and_then(Value::as_str) {
        if !matches!(
            method,
            "GET" | "POST" | "PUT" | "PATCH" | "DELETE" | "HEAD" | "OPTIONS"
        ) {
            return Err(format!("{name}: Unsupported HTTP method {method}"));
        }
    }
    if let Some(body) = req.get("body") {
        let mode = body.get("mode").and_then(Value::as_str).unwrap_or("");
        if !matches!(mode, "raw" | "urlencoded" | "formdata" | "") {
            return Err(format!(
                "{name}: Unsupported body mode {mode}; import aborted"
            ));
        }
        if body
            .get("formdata")
            .and_then(Value::as_array)
            .is_some_and(|a| {
                a.iter()
                    .any(|v| v.get("type").and_then(Value::as_str) == Some("file"))
            })
        {
            return Err(format!(
                "{name}: File form fields require manual attachment; import aborted"
            ));
        }
    }
    if let Some(headers) = req.get("header").and_then(Value::as_array) {
        let mut seen = std::collections::HashSet::new();
        for h in headers {
            if h.get("disabled").and_then(Value::as_bool) == Some(true) {
                warn("Disabled headers are retained in Postman metadata; they are not sent");
            }
            if let Some(key) = h.get("key").and_then(Value::as_str) {
                if !seen.insert(key.to_ascii_lowercase()) {
                    warn("Duplicate headers are retained for export; the YAML runtime uses the last enabled value");
                }
            }
        }
    }
    if value
        .get("event")
        .and_then(Value::as_array)
        .is_some_and(|events| {
            events.iter().any(|e| {
                !matches!(
                    e.get("listen").and_then(Value::as_str),
                    Some("test" | "prerequest")
                )
            })
        })
    {
        warn("Unknown script events are retained for export but are unsupported at runtime");
    }
    if value.get("response").is_some()
        || req.get("protocolProfileBehavior").is_some()
        || value.get("protocolProfileBehavior").is_some()
    {
        warn("Saved responses/protocol profile metadata are retained for export and do not affect runtime");
    }
    if let Some(events) = value.get("event").and_then(Value::as_array) {
        for event in events {
            if event.get("disabled").and_then(Value::as_bool) == Some(true) {
                return Err(format!("{name}: Disabled script events are unsupported; import aborted so skipped tests cannot pass"));
            }
            if let Some(script) = event.get("script") {
                if script.get("src").is_some()
                    || script
                        .get("type")
                        .and_then(Value::as_str)
                        .is_some_and(|kind| kind != "text/javascript")
                    || script.get("exec").is_some_and(|exec| {
                        !exec.is_string()
                            && !exec
                                .as_array()
                                .is_some_and(|lines| lines.iter().all(Value::is_string))
                    })
                {
                    return Err(format!("{name}: External, non-JavaScript or malformed script source is unsupported; import aborted"));
                }
            }
            if event.to_string().contains("pm.") {
                warn("Scripts use pm.*. They are preserved; rewrite them to ark.* before running");
            }
        }
    }
    if let Some(items) = value.get("item").and_then(Value::as_array) {
        for item in items {
            audit(item, warnings)?;
        }
    }
    Ok(())
}

pub fn scope_script(metadata: &Value, phase: &str) -> Option<String> {
    let scripts: Vec<_> = metadata
        .get("event")?
        .as_array()?
        .iter()
        .filter(|e| e.get("listen").and_then(Value::as_str) == Some(phase))
        .filter_map(|e| e.get("script")?.get("exec"))
        .filter_map(|e| {
            if let Some(a) = e.as_array() {
                Some(
                    a.iter()
                        .filter_map(Value::as_str)
                        .collect::<Vec<_>>()
                        .join("\n"),
                )
            } else {
                e.as_str().map(str::to_string)
            }
        })
        .collect();
    if scripts.is_empty() {
        None
    } else {
        Some(scripts.join("\n"))
    }
}

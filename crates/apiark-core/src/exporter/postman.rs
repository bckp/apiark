use crate::{
    importer::postman::{parse_auth, parse_body, parse_headers, parse_url, scope_script},
    models::{
        auth::AuthConfig,
        collection::{CollectionNode, FolderConfig, RequestFile},
    },
    storage::collection::{load_collection_config, load_collection_tree, read_request},
};
use serde_json::{json, Value};
use std::path::Path;

pub fn export_to_postman(path: &Path) -> Result<String, String> {
    let tree = load_collection_tree(path)?;
    let config = load_collection_config(path)?;
    let CollectionNode::Collection { name, children, .. } = tree else {
        return Err("Expected collection".into());
    };
    let mut root = config.postman.unwrap_or_else(|| json!({"info":{}}));
    root["info"]["name"] = json!(name);
    root["info"]["schema"] =
        json!("https://schema.getpostman.com/json/collection/v2.1.0/collection.json");
    root["item"] = json!(export_nodes(&children)?);
    if let Some(auth) = config.defaults.auth {
        root["auth"] = auth_value(&auth)?;
    }
    serde_json::to_string_pretty(&root).map_err(|e| e.to_string())
}

fn export_nodes(nodes: &[CollectionNode]) -> Result<Vec<Value>, String> {
    nodes
        .iter()
        .map(|node| match node {
            CollectionNode::Request { name, path, .. } => {
                export_request(name, &read_request(Path::new(path))?)
            }
            CollectionNode::Folder {
                name,
                path,
                children,
            } => {
                let config = std::fs::read_to_string(Path::new(path).join("_folder.yaml"))
                    .ok()
                    .map(|s| serde_yaml::from_str::<FolderConfig>(&s))
                    .transpose()
                    .map_err(|e| e.to_string())?;
                let mut value = config
                    .as_ref()
                    .and_then(|c| c.postman.clone())
                    .unwrap_or_else(|| json!({}));
                value["name"] = json!(name);
                value["item"] = json!(export_nodes(children)?);
                if let Some(auth) = config.and_then(|c| c.auth) {
                    value["auth"] = auth_value(&auth)?;
                }
                Ok(value)
            }
            CollectionNode::Collection { name, children, .. } => {
                Ok(json!({"name":name,"item":export_nodes(children)?}))
            }
        })
        .collect()
}

fn auth_value(auth: &AuthConfig) -> Result<Value, String> {
    if matches!(auth, AuthConfig::None) {
        return Ok(json!({"type":"noauth"}));
    }
    export_auth(auth)
        .ok_or_else(|| "Authentication cannot be exported to Postman without loss".into())
}

fn export_request(name: &str, req: &RequestFile) -> Result<Value, String> {
    let mut item = req.postman.clone().unwrap_or_else(|| json!({}));
    let original = item.get("request").cloned().unwrap_or_else(|| json!({}));
    let mut request = if original.is_object() {
        original.clone()
    } else {
        json!({})
    };
    item["name"] = json!(name);
    request["method"] = json!(format!("{:?}", req.method));
    if req.url != parse_url(original.get("url")).0 || original.get("url").is_none() {
        request["url"] = json!({"raw":req.url});
    }
    if let Some(params) = &req.params {
        if !request["url"].is_object() {
            request["url"] = json!({"raw":req.url});
        }
        request["url"]["variable"] = json!(params
            .iter()
            .map(|(k, v)| json!({"key":k,"value":v}))
            .collect::<Vec<_>>());
    }
    if req.headers != parse_headers(original.get("header")) {
        request["header"] = json!(req
            .headers
            .iter()
            .map(|(k, v)| json!({"key":k,"value":v,"type":"text"}))
            .collect::<Vec<_>>());
    }
    let original_body = parse_body(original.get("body"));
    let same_body = match (&req.body, &original_body) {
        (None, None) => true,
        (Some(a), Some(b)) => a.body_type == b.body_type && a.content == b.content,
        _ => false,
    };
    if !same_body {
        if let Some(body) = &req.body {
            request["body"] = match body.body_type.as_str() {
                "json" | "xml" | "raw" => {
                    json!({"mode":"raw","raw":body.content,"options":{"raw":{"language":match body.body_type.as_str(){"json"=>"json","xml"=>"xml",_=>"text"}}}})
                }
                "urlencoded" => {
                    json!({"mode":"urlencoded","urlencoded":url::form_urlencoded::parse(body.content.as_bytes()).map(|(k,v)|json!({"key":k,"value":v,"type":"text"})).collect::<Vec<_>>()})
                }
                "form-data" => {
                    json!({"mode":"formdata","formdata":serde_json::from_str::<Value>(&body.content).map_err(|e|format!("Invalid form-data JSON: {e}"))?})
                }
                _ => {
                    return Err(format!(
                        "Unsupported Postman export body: {}",
                        body.body_type
                    ))
                }
            };
        } else {
            request.as_object_mut().unwrap().remove("body");
        }
    }
    if serde_json::to_value(&req.auth).unwrap()
        != serde_json::to_value(parse_auth(original.get("auth"))).unwrap()
    {
        if let Some(auth) = &req.auth {
            request["auth"] = auth_value(auth)?;
        } else {
            request.as_object_mut().unwrap().remove("auth");
        }
    }
    if let Some(description) = &req.description {
        request["description"] = json!(description);
    }
    let trim = |s: Option<String>| s.unwrap_or_default().trim().to_string();
    let same_scripts = trim(req.pre_request_script.clone())
        == trim(scope_script(&item, "prerequest"))
        && trim(req.tests.clone()) == trim(scope_script(&item, "test"))
        && req.post_response_script.is_none();
    if !same_scripts {
        let mut events = item
            .get("event")
            .and_then(Value::as_array)
            .cloned()
            .unwrap_or_default();
        events.retain(|e| {
            !matches!(
                e.get("listen").and_then(Value::as_str),
                Some("prerequest" | "test")
            )
        });
        for (phase, script) in [
            ("prerequest", req.pre_request_script.clone()),
            (
                "test",
                Some(
                    [
                        req.post_response_script.as_deref().unwrap_or(""),
                        req.tests.as_deref().unwrap_or(""),
                    ]
                    .join("\n"),
                ),
            ),
        ] {
            if let Some(script) = script.filter(|s| !s.trim().is_empty()) {
                events.push(json!({"listen":phase,"script":{"type":"text/javascript","exec":script.lines().collect::<Vec<_>>()}}));
            }
        }
        item["event"] = json!(events);
    }
    item["request"] = request;
    Ok(item)
}
fn export_auth(auth: &AuthConfig) -> Option<Value> {
    match auth {
        AuthConfig::None => None,
        AuthConfig::Bearer { token } => Some(json!({
            "type": "bearer",
            "bearer": [
                {"key": "token", "value": token, "type": "string"}
            ]
        })),
        AuthConfig::Basic { username, password } => Some(json!({
            "type": "basic",
            "basic": [
                {"key": "username", "value": username, "type": "string"},
                {"key": "password", "value": password, "type": "string"}
            ]
        })),
        AuthConfig::ApiKey { key, value, add_to } => {
            let in_val = match add_to {
                crate::models::auth::ApiKeyLocation::Header => "header",
                crate::models::auth::ApiKeyLocation::Query => "query",
            };
            Some(json!({
                "type": "apikey",
                "apikey": [
                    {"key": "key", "value": key, "type": "string"},
                    {"key": "value", "value": value, "type": "string"},
                    {"key": "in", "value": in_val, "type": "string"}
                ]
            }))
        }
        AuthConfig::Oauth2 { .. } => {
            // OAuth2 export is complex and Postman has its own format — skip
            None
        }
        AuthConfig::Digest { username, password } => Some(json!({
            "type": "digest",
            "digest": [
                {"key": "username", "value": username, "type": "string"},
                {"key": "password", "value": password, "type": "string"}
            ]
        })),
        AuthConfig::AwsV4 {
            access_key,
            secret_key,
            region,
            service,
            session_token,
        } => Some(json!({
            "type": "awsv4",
            "awsv4": [
                {"key": "accessKey", "value": access_key, "type": "string"},
                {"key": "secretKey", "value": secret_key, "type": "string"},
                {"key": "region", "value": region, "type": "string"},
                {"key": "service", "value": service, "type": "string"},
                {"key": "sessionToken", "value": session_token, "type": "string"}
            ]
        })),
        AuthConfig::JwtBearer { .. } => {
            // JWT Bearer has no direct Postman equivalent
            None
        }
        AuthConfig::Ntlm {
            username, password, ..
        } => Some(json!({
            "type": "ntlm",
            "ntlm": [
                {"key": "username", "value": username, "type": "string"},
                {"key": "password", "value": password, "type": "string"}
            ]
        })),
        AuthConfig::Saml { .. } => {
            // SAML has no direct Postman equivalent
            None
        }
    }
}

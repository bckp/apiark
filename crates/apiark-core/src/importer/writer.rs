use std::collections::HashMap;
use std::fs;
use std::path::Path;

use super::{ImportBody, ImportData, ImportItem};
use crate::models::auth::AuthConfig;
use crate::models::collection::{
    CollectionConfig, CollectionDefaults, RequestBodyFile, RequestFile,
};
use crate::models::request::HttpMethod;

/// Write ImportData to disk as an ApiArk collection.
/// Returns the path to the created collection directory.
pub fn write_import(data: &ImportData, target_dir: &Path) -> Result<String, String> {
    let collection_dir = target_dir.join(sanitize_filename(&data.collection_name));

    if collection_dir.exists() {
        return Err(format!(
            "Directory already exists: {}",
            collection_dir.display()
        ));
    }

    fs::create_dir(&collection_dir).map_err(|e| format!("Cannot create collection: {e}"))?;

    // Create collection structure
    let apiark_dir = collection_dir.join(".apiark");
    fs::create_dir_all(&apiark_dir)
        .map_err(|e| format!("Failed to create collection directory: {e}"))?;

    // Write apiark.yaml
    let config = CollectionConfig {
        postman: data.postman.clone(),
        name: data.collection_name.clone(),
        version: 1,
        defaults: CollectionDefaults {
            auth: data
                .postman
                .as_ref()
                .and_then(|v| super::postman::parse_auth(v.get("auth"))),
            ..CollectionDefaults::default()
        },
    };
    let config_yaml =
        serde_yaml::to_string(&config).map_err(|e| format!("Failed to serialize config: {e}"))?;
    atomic_write(&apiark_dir.join("apiark.yaml"), &config_yaml)?;

    // Write .gitignore
    atomic_write(&apiark_dir.join(".gitignore"), ".env\n")?;

    // Write environments
    if !data.environments.is_empty() {
        let env_dir = apiark_dir.join("environments");
        fs::create_dir_all(&env_dir)
            .map_err(|e| format!("Failed to create environments directory: {e}"))?;

        for env in &data.environments {
            let env_data = serde_yaml::to_string(
                &serde_yaml::to_value(&EnvFile {
                    name: &env.name,
                    variables: &env.variables,
                })
                .map_err(|e| format!("Failed to serialize environment: {e}"))?,
            )
            .map_err(|e| format!("Failed to serialize environment: {e}"))?;

            let env_filename = sanitize_filename(&env.name);
            atomic_write(&env_dir.join(format!("{env_filename}.yaml")), &env_data)?;
        }
    }

    // Write items recursively
    write_items(&data.items, &collection_dir)?;

    Ok(collection_dir.to_string_lossy().to_string())
}

fn write_items(items: &[ImportItem], parent_dir: &Path) -> Result<(), String> {
    // Track filenames for collision avoidance
    let mut used_names: HashMap<String, usize> = HashMap::new();

    let mut order = Vec::new();
    for item in items {
        match item {
            ImportItem::Folder {
                name,
                items,
                postman,
            } => {
                let dirname = unique_name(&sanitize_filename(name), &mut used_names);
                order.push(dirname.clone());
                let folder_path = parent_dir.join(&dirname);
                fs::create_dir_all(&folder_path)
                    .map_err(|e| format!("Failed to create folder: {e}"))?;
                let config = crate::models::collection::FolderConfig {
                    postman: postman.clone(),
                    name: Some(name.clone()),
                    auth: postman
                        .as_ref()
                        .and_then(|v| super::postman::parse_auth(v.get("auth"))),
                    order: vec![],
                };
                atomic_write(
                    &folder_path.join("_folder.yaml"),
                    &serde_yaml::to_string(&config).map_err(|e| e.to_string())?,
                )?;
                write_items(items, &folder_path)?;
            }
            ImportItem::Request {
                postman,
                name,
                method,
                url,
                headers,
                params,
                body,
                auth,
                description,
                pre_request_script,
                post_response_script,
                tests,
            } => {
                let filename = unique_name(&sanitize_filename(name), &mut used_names);
                order.push(filename.clone());

                let request_file = RequestFile {
                    postman: postman.clone(),
                    name: name.clone(),
                    method: parse_method(method),
                    url: url.clone(),
                    protocol: None,
                    description: description.clone(),
                    headers: headers.clone(),
                    auth: convert_auth(auth),
                    body: convert_body(body),
                    params: params.clone(),
                    assert: None,
                    tests: tests.clone(),
                    pre_request_script: pre_request_script.clone(),
                    post_response_script: post_response_script.clone(),
                    cookies: None,
                };

                let yaml = serde_yaml::to_string(&request_file)
                    .map_err(|e| format!("Failed to serialize request: {e}"))?;
                atomic_write(&parent_dir.join(format!("{filename}.yaml")), &yaml)?;
            }
        }
    }
    let path = parent_dir.join("_folder.yaml");
    let mut config = if path.exists() {
        serde_yaml::from_str::<crate::models::collection::FolderConfig>(
            &fs::read_to_string(&path).map_err(|e| e.to_string())?,
        )
        .map_err(|e| e.to_string())?
    } else {
        crate::models::collection::FolderConfig {
            postman: None,
            name: None,
            auth: None,
            order: vec![],
        }
    };
    config.order = order;
    atomic_write(
        &path,
        &serde_yaml::to_string(&config).map_err(|e| e.to_string())?,
    )?;
    Ok(())
}

fn parse_method(method: &str) -> HttpMethod {
    match method.to_uppercase().as_str() {
        "GET" => HttpMethod::GET,
        "POST" => HttpMethod::POST,
        "PUT" => HttpMethod::PUT,
        "PATCH" => HttpMethod::PATCH,
        "DELETE" => HttpMethod::DELETE,
        "HEAD" => HttpMethod::HEAD,
        "OPTIONS" => HttpMethod::OPTIONS,
        _ => HttpMethod::GET,
    }
}

fn convert_auth(auth: &Option<AuthConfig>) -> Option<AuthConfig> {
    match auth {
        Some(AuthConfig::None) => Some(AuthConfig::None),
        other => other.clone(),
    }
}

fn convert_body(body: &Option<ImportBody>) -> Option<RequestBodyFile> {
    body.as_ref().map(|b| RequestBodyFile {
        body_type: b.body_type.clone(),
        content: b.content.clone(),
    })
}

/// Sanitize a string for use as a filename.
fn sanitize_filename(name: &str) -> String {
    let sanitized: String = name
        .chars()
        .take(80)
        .map(|c| {
            if c.is_alphanumeric() || c == '-' || c == '_' || c == '.' {
                c
            } else if c == ' ' {
                '-'
            } else {
                '_'
            }
        })
        .collect();

    let sanitized = sanitized
        .trim_matches(|c: char| c == '-' || c == '_' || c == '.')
        .to_string();

    if sanitized.is_empty() {
        "untitled".to_string()
    } else {
        let lower = sanitized.to_lowercase();
        let stem = lower.split('.').next().unwrap_or("");
        if matches!(stem, "con" | "prn" | "aux" | "nul")
            || ["com", "lpt"].iter().any(|prefix| {
                stem.strip_prefix(prefix).is_some_and(|suffix| {
                    matches!(suffix, "1" | "2" | "3" | "4" | "5" | "6" | "7" | "8" | "9")
                })
            })
        {
            format!("_{lower}")
        } else {
            lower
        }
    }
}

/// Ensure unique name within a directory by appending -1, -2, etc.
fn unique_name(base: &str, used: &mut HashMap<String, usize>) -> String {
    let mut name = base.to_string();
    let mut index = 1;
    while used.contains_key(&name) {
        name = format!("{base}-{index}");
        index += 1;
    }
    used.insert(name.clone(), 1);
    name
}

fn atomic_write(path: &Path, content: &str) -> Result<(), String> {
    let tmp_path = path.with_extension("apiark.tmp");
    fs::write(&tmp_path, content).map_err(|e| format!("Failed to write temp file: {e}"))?;
    fs::rename(&tmp_path, path).map_err(|e| {
        let _ = fs::remove_file(&tmp_path);
        format!("Failed to rename temp file: {e}")
    })
}

#[derive(serde::Serialize)]
struct EnvFile<'a> {
    name: &'a str,
    variables: &'a HashMap<String, String>,
}

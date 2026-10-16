use crate::importer::{self, ImportPreview};
use std::path::Path;
#[tauri::command]
pub fn detect_import_format(file_path: &str) -> Result<String, String> {
    let content = std::fs::read_to_string(file_path).map_err(|e| e.to_string())?;
    importer::postman::parse_postman(&content)?;
    Ok("postman".into())
}
fn parse_import(file_path: &str, format: &str) -> Result<importer::ImportData, String> {
    if format != "postman" {
        return Err("Only Postman Collection v2.1 is supported".into());
    }
    importer::postman::parse_postman(
        &std::fs::read_to_string(file_path).map_err(|e| e.to_string())?,
    )
}
#[tauri::command]
pub fn import_preview(file_path: &str, format: &str) -> Result<ImportPreview, String> {
    Ok(parse_import(file_path, format)?.preview())
}
#[tauri::command]
pub fn import_collection(
    file_path: &str,
    format: &str,
    target_dir: &str,
) -> Result<String, String> {
    importer::writer::write_import(&parse_import(file_path, format)?, Path::new(target_dir))
}
#[tauri::command]
pub fn export_collection(collection_path: &str, format: &str) -> Result<String, String> {
    if format != "postman" {
        return Err("Only Postman Collection v2.1 is supported".into());
    }
    crate::exporter::postman::export_to_postman(Path::new(collection_path))
}
/// Import a Postman environment JSON file into a collection.
/// Parses the environment file and saves it as an ApiArk environment YAML.
#[tauri::command]
pub fn import_environment(file_path: &str, collection_path: &str) -> Result<String, String> {
    let content =
        std::fs::read_to_string(file_path).map_err(|e| format!("Failed to read file: {e}"))?;

    let root: serde_json::Value =
        serde_json::from_str(&content).map_err(|e| format!("Invalid JSON: {e}"))?;

    let name = root
        .get("name")
        .and_then(|v| v.as_str())
        .unwrap_or("Imported Environment")
        .to_string();

    let values = root
        .get("values")
        .and_then(|v| v.as_array())
        .ok_or("Not a valid Postman environment file: missing 'values' array")?;

    let mut variables = std::collections::HashMap::new();
    for val in values {
        let enabled = val.get("enabled").and_then(|v| v.as_bool()).unwrap_or(true);
        if !enabled {
            continue;
        }
        if let Some(key) = val.get("key").and_then(|k| k.as_str()) {
            let value = val
                .get("value")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .to_string();
            variables.insert(key.to_string(), value);
        }
    }

    let env = crate::models::environment::EnvironmentFile {
        name: name.clone(),
        variables,
        secrets: Vec::new(),
        scope: Default::default(),
    };

    crate::storage::safe_environment::save(Path::new(collection_path), &env, true)?;

    Ok(name)
}

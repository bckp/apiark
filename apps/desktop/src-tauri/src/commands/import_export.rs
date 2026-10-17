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
    let data = parse_import(file_path, format)?;
    let target = Path::new(target_dir);
    std::fs::create_dir_all(target)
        .map_err(|e| format!("Cannot create import directory {}: {e}", target.display()))?;
    importer::writer::write_import(&data, target)
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

#[cfg(test)]
mod tests {
    use super::import_collection;

    #[test]
    fn import_creates_destination_and_preserves_existing_collection_on_collision() {
        let root = tempfile::tempdir().unwrap();
        let source = root.path().join("synthetic.postman.json");
        std::fs::write(
            &source,
            serde_json::json!({
                "info": {
                    "name": "Synthetic",
                    "schema": "https://schema.getpostman.com/json/collection/v2.1.0/collection.json"
                },
                "item": [{"name": "Health", "request": {
                    "method": "GET", "url": "https://example.test/health"
                }}]
            })
            .to_string(),
        )
        .unwrap();
        let target = root.path().join("ApiArk");
        let collection = import_collection(
            source.to_str().unwrap(),
            "postman",
            target.to_str().unwrap(),
        )
        .unwrap();
        let request = std::path::Path::new(&collection).join("Health.yaml");
        let original = std::fs::read(&request).unwrap();
        assert!(String::from_utf8_lossy(&original).contains("https://example.test/health"));
        assert!(import_collection(
            source.to_str().unwrap(),
            "postman",
            target.to_str().unwrap()
        )
        .is_err());
        assert_eq!(std::fs::read(&request).unwrap(), original);

        let rejected_target = root.path().join("invalid-import");
        std::fs::write(&source, "{}").unwrap();
        assert!(import_collection(
            source.to_str().unwrap(),
            "postman",
            rejected_target.to_str().unwrap()
        )
        .is_err());
        assert!(!rejected_target.exists());
    }
}

use std::path::Path;
pub fn detect_format(path: &Path) -> anyhow::Result<String> {
    let value: serde_json::Value = serde_json::from_str(&std::fs::read_to_string(path)?)?;
    if value.get("info").is_some() && value.get("item").is_some() {
        Ok("postman".into())
    } else {
        anyhow::bail!("Supported import: Postman Collection v2.1")
    }
}
pub fn import_collection(path: &Path, target: &Path, format: &str) -> anyhow::Result<usize> {
    if format != "postman" {
        anyhow::bail!("Supported import: postman")
    }
    let data = apiark_core::importer::postman::parse_postman(&std::fs::read_to_string(path)?)
        .map_err(anyhow::Error::msg)?;
    for warning in &data.warnings {
        eprintln!("Warning: {}: {}", warning.item_name, warning.message);
    }
    let count = data.preview().request_count;
    let written =
        apiark_core::importer::writer::write_import(&data, target).map_err(anyhow::Error::msg)?;
    eprintln!("Collection: {written}");
    Ok(count)
}
pub fn export_collection(path: &Path, format: &str) -> anyhow::Result<String> {
    if format != "postman" {
        anyhow::bail!("Supported export: postman")
    }
    apiark_core::exporter::postman::export_to_postman(path).map_err(anyhow::Error::msg)
}

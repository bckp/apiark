use std::path::Path;
pub fn load_config(
    path: &Path,
) -> anyhow::Result<apiark_core::models::collection::CollectionConfig> {
    apiark_core::storage::collection::load_collection_config(path).map_err(anyhow::Error::msg)
}

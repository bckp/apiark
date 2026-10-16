use crate::{
    importer::postman::{parse_auth, scope_script},
    models::{auth::AuthConfig, collection::FolderConfig},
    storage::collection::load_collection_config,
};
use std::{collections::HashMap, path::Path};

#[derive(Default)]
pub struct Scopes {
    pub variables: HashMap<String, String>,
    pub pre: Vec<String>,
    pub post: Vec<String>,
    pub auth: Option<AuthConfig>,
}

pub fn load(root: Option<&str>, source: Option<&str>) -> Result<Scopes, String> {
    let mut result = Scopes::default();
    let Some(root) = root else { return Ok(result) };
    let root = Path::new(root).canonicalize().map_err(|e| e.to_string())?;
    let config = load_collection_config(&root)?;
    result.auth = config.defaults.auth;
    let mut metadata = vec![config.postman];
    if let Some(source) = source {
        let path = Path::new(source)
            .canonicalize()
            .map_err(|e| e.to_string())?;
        if !path.starts_with(&root) {
            return Err("Request source is outside collection".into());
        }
        let mut parents: Vec<_> = path
            .parent()
            .unwrap()
            .ancestors()
            .take_while(|p| *p != root)
            .collect();
        parents.reverse();
        for parent in parents {
            let path = parent.join("_folder.yaml");
            if path.exists() {
                let config: FolderConfig = serde_yaml::from_str(
                    &std::fs::read_to_string(path).map_err(|e| e.to_string())?,
                )
                .map_err(|e| e.to_string())?;
                if config.auth.is_some() {
                    result.auth = config.auth;
                }
                metadata.push(config.postman);
            }
        }
    }
    for value in metadata.into_iter().flatten() {
        if let Some(vars) = value.get("variable").and_then(|v| v.as_array()) {
            for var in vars
                .iter()
                .filter(|v| v.get("disabled").and_then(|v| v.as_bool()) != Some(true))
            {
                if let (Some(key), Some(value)) =
                    (var.get("key").and_then(|v| v.as_str()), var.get("value"))
                {
                    result.variables.insert(
                        key.into(),
                        value
                            .as_str()
                            .map(str::to_string)
                            .unwrap_or_else(|| value.to_string()),
                    );
                }
            }
        }
        if let Some(script) = scope_script(&value, "prerequest") {
            result.pre.push(script);
        }
        if let Some(script) = scope_script(&value, "test") {
            result.post.push(script);
        }
        if result.auth.is_none() {
            result.auth = parse_auth(value.get("auth"));
        }
    }
    Ok(result)
}

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

/// Completion metadata: serialize names only, never resolved values or scripts.
pub fn variable_names(
    root: Option<&str>,
    source: Option<&str>,
    environment: Option<&str>,
    include_scopes: bool,
    removed: &[String],
) -> Result<Vec<String>, String> {
    let mut names: std::collections::BTreeSet<String> =
        crate::http::interpolation::DYNAMIC_VARIABLE_NAMES
            .iter()
            .map(|name| (*name).to_string())
            .collect();
    if let Some(root) = root {
        if include_scopes {
            names.extend(load(Some(root), source)?.variables.into_keys());
        }
        let variables = if let Some(env) = environment {
            crate::storage::environment::get_resolved_variables(Path::new(root), env)?
        } else {
            crate::storage::environment::load_root_dotenv(Path::new(root))
        };
        names.extend(variables.into_keys().filter(|name| !removed.contains(name)));
    }
    Ok(names.into_iter().collect())
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn completion_returns_only_names_from_the_selected_scopes() {
        let dir = tempfile::tempdir().unwrap();
        let root = dir.path();
        std::fs::create_dir_all(root.join(".apiark/environments")).unwrap();
        std::fs::create_dir(root.join("folder")).unwrap();
        std::fs::write(root.join(".apiark/apiark.yaml"), "name: Synthetic\npostman:\n  variable:\n    - {key: collectionKey, value: synthetic-collection-value}\n    - {key: disabledKey, value: hidden, disabled: true}\n").unwrap();
        std::fs::write(
            root.join("folder/_folder.yaml"),
            "postman:\n  variable:\n    - {key: folderKey, value: synthetic-folder-value}\n",
        )
        .unwrap();
        std::fs::write(root.join("folder/request.yaml"), "name: Synthetic").unwrap();
        std::fs::write(root.join(".env"), "rootKey=synthetic-root-value\n").unwrap();
        std::fs::write(
            root.join(".apiark/.env"),
            "secretKey=synthetic-private-value\notherSecret=synthetic-other-value\n",
        )
        .unwrap();
        std::fs::write(
            root.join(".apiark/environments/test.yaml"),
            "name: test\nvariables: {envKey: synthetic-env-value}\nsecrets: [secretKey]\n",
        )
        .unwrap();
        let names = variable_names(
            root.to_str(),
            root.join("folder/request.yaml").to_str(),
            Some("test"),
            true,
            &[],
        )
        .unwrap();
        for name in [
            "rootKey",
            "envKey",
            "secretKey",
            "collectionKey",
            "folderKey",
            "$uuid",
        ] {
            assert!(names.contains(&name.to_string()), "{name}");
        }
        let serialized = serde_json::to_string(&names).unwrap();
        assert!(!serialized.contains("synthetic-"));
        assert!(!serialized.contains("disabledKey"));
        assert!(!serialized.contains("otherSecret"));
        let root_names = variable_names(root.to_str(), None, None, true, &[]).unwrap();
        assert!(!root_names.contains(&"envKey".into()));
        assert!(!root_names.contains(&"folderKey".into()));
        assert!(variable_names(root.to_str(), None, Some("missing"), true, &[]).is_err());
        let outside = tempfile::NamedTempFile::new().unwrap();
        assert!(variable_names(root.to_str(), outside.path().to_str(), None, true, &[]).is_err());
        let unbound =
            variable_names(root.to_str(), None, Some("test"), false, &["envKey".into()]).unwrap();
        assert!(!unbound.contains(&"collectionKey".into()));
        assert!(!unbound.contains(&"envKey".into()));
        let scoped =
            variable_names(root.to_str(), None, None, true, &["collectionKey".into()]).unwrap();
        assert!(scoped.contains(&"collectionKey".into()));
    }
}

use crate::models::environment::{EnvironmentFile, EnvironmentScope};
use cap_std::{
    ambient_authority,
    fs::{Dir, OpenOptions},
};
use std::{
    io::{Read, Write},
    path::Path,
};

pub fn filename(name: &str) -> Result<String, String> {
    if name.trim().is_empty()
        || name != name.trim()
        || name.chars().count() > 100
        || !name
            .chars()
            .all(|c| c.is_alphanumeric() || matches!(c, ' ' | '-' | '_'))
    {
        return Err("Environment name must contain 1–100 letters, numbers, spaces, - or _".into());
    }
    let stem = name.to_lowercase().replace(' ', "-");
    if matches!(
        stem.as_str(),
        "con"
            | "prn"
            | "aux"
            | "nul"
            | "com1"
            | "com2"
            | "com3"
            | "com4"
            | "com5"
            | "com6"
            | "com7"
            | "com8"
            | "com9"
            | "lpt1"
            | "lpt2"
            | "lpt3"
            | "lpt4"
            | "lpt5"
            | "lpt6"
            | "lpt7"
            | "lpt8"
            | "lpt9"
    ) {
        return Err("Reserved environment filename".into());
    }
    Ok(format!("{stem}.yaml"))
}

/// All operations are relative to directory capabilities. Symlinks cannot escape
/// the selected collection even if a component changes after validation.
pub fn save(root: &Path, env: &EnvironmentFile, create_only: bool) -> Result<(), String> {
    let name = filename(&env.name)?;
    let root = Dir::open_ambient_dir(root, ambient_authority()).map_err(|e| e.to_string())?;
    fn child(parent: &Dir, name: &str) -> Result<Dir, String> {
        match parent.create_dir(name) {
            Ok(()) => {}
            Err(e) if e.kind() == std::io::ErrorKind::AlreadyExists => {}
            Err(e) => return Err(e.to_string()),
        }
        if parent
            .symlink_metadata(name)
            .map_err(|e| e.to_string())?
            .file_type()
            .is_symlink()
        {
            return Err("Symlinked environment directories are unsupported".into());
        }
        parent.open_dir(name).map_err(|e| e.to_string())
    }
    let metadata = child(&root, ".apiark")?;
    let dir = child(
        &metadata,
        match env.scope {
            EnvironmentScope::Shared => "environments",
            EnvironmentScope::Personal => "environments.local",
        },
    )?;
    let exists = match dir.symlink_metadata(&name) {
        Ok(m) => {
            if m.file_type().is_symlink() || !m.is_file() {
                return Err("Environment target is not a regular file".into());
            }
            true
        }
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => false,
        Err(e) => return Err(e.to_string()),
    };
    if exists {
        if create_only {
            return Err(
                "Environment already exists; import does not overwrite existing data".into(),
            );
        }
        let mut existing = String::new();
        dir.open(&name)
            .map_err(|e| e.to_string())?
            .take(1024 * 1024)
            .read_to_string(&mut existing)
            .map_err(|e| e.to_string())?;
        let old: EnvironmentFile = serde_yaml::from_str(&existing).map_err(|e| e.to_string())?;
        if old.name != env.name {
            return Err("Environment filename collision".into());
        }
    }
    let yaml = serde_yaml::to_string(env).map_err(|e| e.to_string())?;
    let temp = format!(".import-{}.tmp", uuid::Uuid::new_v4());
    let mut options = OpenOptions::new();
    options.write(true).create_new(true);
    let mut file = dir.open_with(&temp, &options).map_err(|e| e.to_string())?;
    file.write_all(yaml.as_bytes())
        .and_then(|_| file.sync_all())
        .map_err(|e| e.to_string())?;
    drop(file);
    let result = if create_only || !exists {
        // hard_link is an atomic create-without-replacement on all supported OSes.
        dir.hard_link(&temp, &dir, &name)
    } else {
        dir.rename(&temp, &dir, &name)
    };
    let _ = dir.remove_file(&temp);
    result.map_err(|e| e.to_string())?;
    if matches!(env.scope, EnvironmentScope::Personal) {
        if let Ok(mut f) = dir.open_with(".gitignore", &options) {
            f.write_all(b"*\n!.gitignore\n")
                .map_err(|e| e.to_string())?;
        }
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;
    fn env(name: &str) -> EnvironmentFile {
        EnvironmentFile {
            name: name.into(),
            variables: Default::default(),
            secrets: vec![],
            scope: Default::default(),
        }
    }
    #[test]
    fn traversal_absolute_names_collisions_and_ordinary_updates() {
        let tmp = tempfile::tempdir().unwrap();
        for name in [
            "../outside",
            "/tmp/outside",
            "C:\\outside",
            "..",
            ".",
            "foo/bar",
            "foo\\bar",
            "CON",
            "",
        ] {
            assert!(save(tmp.path(), &env(name), true).is_err(), "{name}");
        }
        save(tmp.path(), &env("Development"), true).unwrap();
        assert!(save(tmp.path(), &env("Development"), true).is_err());
        assert!(save(tmp.path(), &env("development"), false).is_err());
        save(tmp.path(), &env("Development"), false).unwrap();
    }
    #[cfg(unix)]
    #[test]
    fn rejects_symlinked_directories_and_files() {
        let tmp = tempfile::tempdir().unwrap();
        let outside = tempfile::tempdir().unwrap();
        std::os::unix::fs::symlink(outside.path(), tmp.path().join(".apiark")).unwrap();
        assert!(save(tmp.path(), &env("Test"), true).is_err());
        assert_eq!(std::fs::read_dir(outside.path()).unwrap().count(), 0);
        let other = tempfile::tempdir().unwrap();
        save(other.path(), &env("Valid"), true).unwrap();
        std::os::unix::fs::symlink(
            outside.path().join("secret"),
            other.path().join(".apiark/environments/test.yaml"),
        )
        .unwrap();
        assert!(save(other.path(), &env("Test"), true).is_err());
        assert!(!outside.path().join("secret").exists());
    }
}

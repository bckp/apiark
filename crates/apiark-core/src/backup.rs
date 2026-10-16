use std::{
    collections::BTreeMap,
    io::{Read, Seek},
};

pub const MAX_BACKUP_BYTES: u64 = 64 * 1024 * 1024;

/// Validate and bound every member before the caller mutates any application data.
pub fn read_backup(input: impl Read + Seek) -> Result<BTreeMap<String, Vec<u8>>, String> {
    let mut zip = zip::ZipArchive::new(input).map_err(|e| e.to_string())?;
    if zip.len() > 3 {
        return Err("Backup has more than 3 members".into());
    }
    let mut result = BTreeMap::new();
    let mut total = 0u64;
    for i in 0..zip.len() {
        let mut entry = zip.by_index(i).map_err(|e| e.to_string())?;
        let name = entry.name().to_string();
        let limit = match name.as_str() {
            "settings.json" => 2 * 1024 * 1024,
            "state.json" => 8 * 1024 * 1024,
            "data.db" => MAX_BACKUP_BYTES,
            _ => return Err(format!("Unsupported backup member: {name}")),
        };
        if result.contains_key(&name) || entry.unix_mode().is_some_and(|m| m & 0o170000 == 0o120000)
        {
            return Err("Duplicate or symlinked backup member".into());
        }
        if entry.size() > limit || entry.size() > MAX_BACKUP_BYTES - total {
            return Err("Backup exceeds decompression limit".into());
        }
        let mut data = Vec::new();
        (&mut entry)
            .take(limit.min(MAX_BACKUP_BYTES - total) + 1)
            .read_to_end(&mut data)
            .map_err(|e| e.to_string())?;
        if data.len() as u64 > limit || data.len() as u64 > MAX_BACKUP_BYTES - total {
            return Err("Backup exceeds decompression limit".into());
        }
        if name.ends_with(".json") {
            serde_json::from_slice::<serde_json::Value>(&data).map_err(|e| e.to_string())?;
        }
        total += data.len() as u64;
        result.insert(name, data);
    }
    Ok(result)
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::io::{Cursor, Write};
    fn archive(name: &str, bytes: &[u8]) -> Cursor<Vec<u8>> {
        let mut zip = zip::ZipWriter::new(Cursor::new(Vec::new()));
        zip.start_file(
            name,
            zip::write::SimpleFileOptions::default()
                .compression_method(zip::CompressionMethod::Deflated),
        )
        .unwrap();
        zip.write_all(bytes).unwrap();
        zip.finish().unwrap()
    }
    #[test]
    fn bounded_backup_preserves_normal_data_and_rejects_bombs_and_traversal() {
        assert_eq!(
            read_backup(archive("settings.json", b"{\"theme\":\"dark\"}")).unwrap()
                ["settings.json"],
            b"{\"theme\":\"dark\"}"
        );
        assert!(read_backup(archive("../settings.json", b"{}")).is_err());
        assert!(read_backup(archive("settings.json", &vec![b' '; 2 * 1024 * 1024 + 1])).is_err());
        assert!(read_backup(archive("state.json", b"invalid JSON")).is_err());
    }
}

use std::collections::HashMap;
use std::sync::RwLock;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CookieEntry {
    #[serde(default = "host_only_default")]
    pub host_only: bool,
    pub name: String,
    pub value: String,
    pub domain: String,
    pub path: String,
    pub expires: Option<String>,
    pub http_only: bool,
    pub secure: bool,
    pub same_site: Option<String>,
}

fn host_only_default() -> bool {
    true
}

impl From<&crate::models::response::CookieData> for CookieEntry {
    fn from(c: &crate::models::response::CookieData) -> Self {
        Self {
            host_only: c.host_only,
            name: c.name.clone(),
            value: c.value.clone(),
            domain: c.domain.clone().unwrap_or_default(),
            path: c.path.clone().unwrap_or_else(|| "/".into()),
            expires: c.expires.clone(),
            http_only: c.http_only,
            secure: c.secure,
            same_site: None,
        }
    }
}

impl CookieEntry {
    pub fn matches(&self, url: &url::Url) -> bool {
        let Some(host) = url.host_str() else {
            return false;
        };
        if self.domain.is_empty() || (self.secure && url.scheme() != "https") {
            return false;
        }
        let domain = self.domain.trim_start_matches('.');
        let domain_match = host == domain
            || (!self.host_only
                && host.parse::<std::net::IpAddr>().is_err()
                && host
                    .strip_suffix(domain)
                    .is_some_and(|prefix| prefix.ends_with('.')));
        let path = if self.path.starts_with('/') {
            self.path.as_str()
        } else {
            "/"
        };
        let path_match = url.path() == path
            || (url.path().starts_with(path)
                && (path.ends_with('/') || url.path().as_bytes().get(path.len()) == Some(&b'/')));
        domain_match && path_match && !self.expired()
    }

    fn expired(&self) -> bool {
        self.expires.as_ref().is_some_and(|s| {
            chrono::DateTime::parse_from_rfc3339(s)
                .map(|t| t.timestamp() <= chrono::Utc::now().timestamp())
                .unwrap_or(true)
        })
    }
}

/// Per-collection cookie jar. Stores cookies in memory with optional persistence.
pub struct CookieJarManager {
    /// Map of collection_path -> list of cookies
    jars: RwLock<HashMap<String, Vec<CookieEntry>>>,
}

impl CookieJarManager {
    pub fn new() -> Self {
        Self {
            jars: RwLock::new(HashMap::new()),
        }
    }

    /// Get all cookies for a collection.
    pub fn get_cookies(&self, collection_path: &str) -> Result<Vec<CookieEntry>, String> {
        let jars = self.jars.read().map_err(|e| format!("Lock error: {e}"))?;
        Ok(jars.get(collection_path).cloned().unwrap_or_default())
    }

    pub fn cookie_header(&self, collection_path: &str, url: &url::Url) -> Result<String, String> {
        let mut cookies = self.get_cookies(collection_path)?;
        cookies.retain(|c| c.matches(url));
        cookies.sort_by_key(|c| std::cmp::Reverse(c.path.len()));
        Ok(cookies
            .iter()
            .map(|c| format!("{}={}", c.name, c.value))
            .collect::<Vec<_>>()
            .join("; "))
    }

    /// Store cookies from a response's Set-Cookie headers.
    pub fn store_cookies(
        &self,
        collection_path: &str,
        cookies: Vec<CookieEntry>,
    ) -> Result<(), String> {
        let mut jars = self.jars.write().map_err(|e| format!("Lock error: {e}"))?;
        let jar = jars.entry(collection_path.to_string()).or_default();

        for new_cookie in cookies {
            jar.retain(|c| {
                !(c.name == new_cookie.name
                    && c.domain == new_cookie.domain
                    && c.path == new_cookie.path)
            });
            if new_cookie.domain.is_empty() || new_cookie.expired() {
                continue;
            }
            // Replace existing cookie with same name+domain+path
            if let Some(existing) = jar.iter_mut().find(|c| {
                c.name == new_cookie.name
                    && c.domain == new_cookie.domain
                    && c.path == new_cookie.path
            }) {
                *existing = new_cookie;
            } else {
                jar.push(new_cookie);
            }
        }

        Ok(())
    }

    /// Delete a specific cookie.
    pub fn delete_cookie(
        &self,
        collection_path: &str,
        name: &str,
        domain: &str,
    ) -> Result<(), String> {
        let mut jars = self.jars.write().map_err(|e| format!("Lock error: {e}"))?;
        if let Some(jar) = jars.get_mut(collection_path) {
            jar.retain(|c| !(c.name == name && c.domain == domain));
        }
        Ok(())
    }

    /// Clear all cookies for a collection.
    pub fn clear_jar(&self, collection_path: &str) -> Result<(), String> {
        let mut jars = self.jars.write().map_err(|e| format!("Lock error: {e}"))?;
        jars.remove(collection_path);
        Ok(())
    }

    /// Save cookie jar to disk for persistence.
    pub fn save_to_disk(&self, collection_path: &str) -> Result<(), String> {
        let jars = self.jars.read().map_err(|e| format!("Lock error: {e}"))?;
        let cookies = jars.get(collection_path).cloned().unwrap_or_default();

        let cookies_dir = dirs::home_dir()
            .ok_or("Could not determine home directory")?
            .join(".apiark-internal")
            .join("cookies");

        std::fs::create_dir_all(&cookies_dir)
            .map_err(|e| format!("Failed to create cookies dir: {e}"))?;

        // Use a hash of collection path as filename
        let hash = simple_hash(collection_path);
        let path = cookies_dir.join(format!("{hash}.json"));

        let json = serde_json::to_string_pretty(&cookies)
            .map_err(|e| format!("Failed to serialize cookies: {e}"))?;

        std::fs::write(&path, json).map_err(|e| format!("Failed to write cookies: {e}"))?;

        Ok(())
    }

    /// Load cookie jar from disk.
    pub fn load_from_disk(&self, collection_path: &str) -> Result<(), String> {
        let mut jars = self.jars.write().map_err(|e| format!("Lock error: {e}"))?;
        if jars.contains_key(collection_path) {
            return Ok(());
        }
        let cookies_dir = dirs::home_dir()
            .ok_or("Could not determine home directory")?
            .join(".apiark-internal")
            .join("cookies");

        let hash = simple_hash(collection_path);
        let path = cookies_dir.join(format!("{hash}.json"));

        if !path.exists() {
            jars.insert(collection_path.to_string(), Vec::new());
            return Ok(());
        }

        let json =
            std::fs::read_to_string(&path).map_err(|e| format!("Failed to read cookies: {e}"))?;

        let cookies: Vec<CookieEntry> =
            serde_json::from_str(&json).map_err(|e| format!("Failed to parse cookies: {e}"))?;

        jars.insert(collection_path.to_string(), cookies);

        Ok(())
    }
}

fn simple_hash(s: &str) -> String {
    use std::collections::hash_map::DefaultHasher;
    use std::hash::{Hash, Hasher};
    let mut hasher = DefaultHasher::new();
    s.hash(&mut hasher);
    format!("{:016x}", hasher.finish())
}

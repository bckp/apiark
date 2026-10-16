use serde::{Deserialize, Serialize};

use super::request::KeyValuePair;

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ResponseData {
    pub status: u16,
    pub status_text: String,
    pub headers: Vec<KeyValuePair>,
    pub cookies: Vec<CookieData>,
    pub body: String,
    /// Response time in milliseconds
    pub time_ms: u64,
    /// Response size in bytes
    pub size_bytes: u64,
    /// True if the body was truncated (original > 1MB)
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub truncated: Option<bool>,
    /// Full response size when truncated
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub full_size: Option<u64>,
    /// Temp file path containing full response body when truncated
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub temp_path: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct CookieData {
    pub name: String,
    pub value: String,
    pub domain: Option<String>,
    pub path: Option<String>,
    pub expires: Option<String>,
    pub http_only: bool,
    pub secure: bool,
}

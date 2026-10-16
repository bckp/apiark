pub mod bruno;
pub mod har;
pub mod hoppscotch;
pub mod insomnia;
pub mod openapi;
pub mod postman;
pub mod writer;

use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use crate::models::auth::AuthConfig;

/// Intermediate representation for imported collections.
/// All importers produce this struct; the writer converts it to disk format.
#[derive(Debug, Clone)]
pub struct ImportData {
    pub collection_name: String,
    pub items: Vec<ImportItem>,
    pub environments: Vec<ImportEnvironment>,
    pub warnings: Vec<ImportWarning>,
}

#[derive(Debug, Clone)]
pub enum ImportItem {
    Folder {
        name: String,
        items: Vec<ImportItem>,
    },
    Request {
        name: String,
        method: String,
        url: String,
        headers: HashMap<String, String>,
        params: Option<HashMap<String, String>>,
        body: Box<Option<ImportBody>>,
        auth: Box<Option<AuthConfig>>,
        description: Option<String>,
        pre_request_script: Option<String>,
        post_response_script: Option<String>,
        tests: Option<String>,
    },
}

#[derive(Debug, Clone)]
pub struct ImportBody {
    pub body_type: String,
    pub content: String,
}

#[derive(Debug, Clone)]
pub struct ImportEnvironment {
    pub name: String,
    pub variables: HashMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportWarning {
    pub item_name: String,
    pub message: String,
}

/// Preview sent to frontend before actual import.
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ImportPreview {
    pub collection_name: String,
    pub request_count: usize,
    pub folder_count: usize,
    pub environment_count: usize,
    pub warnings: Vec<ImportWarning>,
}

impl ImportData {
    pub fn preview(&self) -> ImportPreview {
        ImportPreview {
            collection_name: self.collection_name.clone(),
            request_count: count_requests(&self.items),
            folder_count: count_folders(&self.items),
            environment_count: self.environments.len(),
            warnings: self.warnings.clone(),
        }
    }
}

fn count_requests(items: &[ImportItem]) -> usize {
    items.iter().fold(0, |acc, item| match item {
        ImportItem::Request { .. } => acc + 1,
        ImportItem::Folder { items, .. } => acc + count_requests(items),
    })
}

fn count_folders(items: &[ImportItem]) -> usize {
    items.iter().fold(0, |acc, item| match item {
        ImportItem::Request { .. } => acc,
        ImportItem::Folder { items, .. } => acc + 1 + count_folders(items),
    })
}

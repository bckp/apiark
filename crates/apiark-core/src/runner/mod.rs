pub mod data_reader;

use crate::{
    execution::{execute, request_file_to_params, Scripts},
    http::cookies::CookieJarManager,
    models::{collection::CollectionNode, request::SendRequestParams},
    storage::{collection, environment},
};
use std::path::Path;

fn paths(node: &CollectionNode) -> Vec<String> {
    match node {
        CollectionNode::Request { path, .. } => vec![path.clone()],
        CollectionNode::Folder { children, .. } | CollectionNode::Collection { children, .. } => {
            children.iter().flat_map(paths).collect()
        }
    }
}

/// Shared collection traversal, variable scope, scripts, tests and pass/fail semantics.
pub async fn run_collection(
    config: RunConfig,
    jar: &CookieJarManager,
    resolve_auth: impl Fn(&mut SendRequestParams) -> Result<(), String>,
    progress: impl Fn(RunProgress),
) -> Result<RunSummary, String> {
    let root = Path::new(&config.collection_path);
    let tree = collection::load_collection_tree(root)?;
    let mut request_paths = paths(&tree);
    if let Some(folder) = &config.folder_path {
        let folder = Path::new(folder)
            .canonicalize()
            .map_err(|e| e.to_string())?;
        let canonical_root = root.canonicalize().map_err(|e| e.to_string())?;
        if !folder.starts_with(canonical_root) {
            return Err("Folder is outside collection".into());
        }
        request_paths.retain(|p| Path::new(p).starts_with(&folder));
    }
    if request_paths.is_empty() {
        return Err("No requests found".into());
    }
    let base_vars = if let Some(env) = &config.environment_name {
        environment::get_resolved_variables(root, env)?
    } else {
        environment::load_root_dotenv(root)
    };
    let data = config
        .data_file
        .as_ref()
        .map(|p| data_reader::read_data_file(p))
        .transpose()?
        .unwrap_or_default();
    let iterations = if config.data_file.is_some() && config.iterations == 1 {
        data.len().max(1) as u32
    } else {
        config.iterations.max(1)
    };
    let mut summary = RunSummary {
        total_requests: 0,
        total_passed: 0,
        total_failed: 0,
        total_time_ms: 0,
        iterations: Vec::new(),
    };
    let run_id = uuid::Uuid::new_v4().to_string();
    for iteration in 0..iterations {
        let mut vars = base_vars.clone();
        if !data.is_empty() {
            vars.extend(data[iteration as usize % data.len()].clone());
        }
        let mut results = Vec::new();
        for (idx, path) in request_paths.iter().enumerate() {
            let file = collection::read_request(Path::new(path));
            let mut result = RequestRunResult {
                visualization_count: 0,
                name: path.clone(),
                method: "?".into(),
                url: String::new(),
                status: None,
                time_ms: None,
                passed: false,
                test_count: 0,
                test_passed: 0,
                assertion_count: 0,
                assertion_passed: 0,
                error: None,
            };
            match file {
                Err(e) => result.error = Some(e),
                Ok(file) => {
                    if file
                        .protocol
                        .as_deref()
                        .is_some_and(|p| !matches!(p, "http" | "graphql"))
                    {
                        return Err(format!(
                            "Unsupported protocol {} in {}",
                            file.protocol.as_deref().unwrap(),
                            path
                        ));
                    }
                    result.name = file.name.clone();
                    result.method = format!("{:?}", file.method);
                    result.url = file.url.clone();
                    let scripts = Scripts {
                        source_path: Some(path.clone()),
                        pre_request: file.pre_request_script.clone(),
                        post_response: file.post_response_script.clone(),
                        tests: file.tests.clone(),
                        assertions_yaml: file
                            .assert
                            .as_ref()
                            .map(serde_yaml::to_string)
                            .transpose()
                            .map_err(|e| e.to_string())?,
                    };
                    match execute(
                        request_file_to_params(&file),
                        &mut vars,
                        Some(&config.collection_path),
                        jar,
                        scripts,
                        &resolve_auth,
                    )
                    .await
                    {
                        Err(e) => result.error = Some(e),
                        Ok(response) => {
                            result.visualization_count = response.visualizations.len();
                            result.status = Some(response.status);
                            result.time_ms = Some(response.time_ms);
                            result.test_count = response.test_results.len();
                            result.test_passed =
                                response.test_results.iter().filter(|t| t.passed).count();
                            result.assertion_count = response.assertion_results.len();
                            result.assertion_passed = response
                                .assertion_results
                                .iter()
                                .filter(|t| t.passed)
                                .count();
                            result.passed = result.test_count == result.test_passed
                                && result.assertion_count == result.assertion_passed;
                            if !result.passed {
                                result.error = Some(
                                    response
                                        .test_results
                                        .iter()
                                        .filter(|t| !t.passed)
                                        .map(|t| {
                                            format!(
                                                "{}: {}",
                                                t.name,
                                                t.error.as_deref().unwrap_or("failed")
                                            )
                                        })
                                        .collect::<Vec<_>>()
                                        .join("; "),
                                );
                            }
                        }
                    }
                }
            }
            summary.total_requests += 1;
            if result.passed {
                summary.total_passed += 1;
            } else {
                summary.total_failed += 1;
            }
            summary.total_time_ms += result.time_ms.unwrap_or(0);
            progress(RunProgress {
                run_id: run_id.clone(),
                iteration,
                request_index: idx,
                total_requests: request_paths.len(),
                result: result.clone(),
            });
            let stop = config.stop_on_error && !result.passed;
            results.push(result);
            if stop {
                break;
            }
            if config.delay_ms > 0 && idx + 1 < request_paths.len() {
                tokio::time::sleep(std::time::Duration::from_millis(config.delay_ms)).await;
            }
        }
        summary
            .iterations
            .push(IterationResult { iteration, results });
    }
    Ok(summary)
}

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RunConfig {
    pub collection_path: String,
    pub folder_path: Option<String>,
    pub environment_name: Option<String>,
    pub delay_ms: u64,
    pub iterations: u32,
    pub data_file: Option<String>,
    pub stop_on_error: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RequestRunResult {
    pub visualization_count: usize,
    pub name: String,
    pub method: String,
    pub url: String,
    pub status: Option<u16>,
    pub time_ms: Option<u64>,
    pub passed: bool,
    pub test_count: usize,
    pub test_passed: usize,
    pub assertion_count: usize,
    pub assertion_passed: usize,
    pub error: Option<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct IterationResult {
    pub iteration: u32,
    pub results: Vec<RequestRunResult>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RunSummary {
    pub total_requests: usize,
    pub total_passed: usize,
    pub total_failed: usize,
    pub total_time_ms: u64,
    pub iterations: Vec<IterationResult>,
}

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct RunProgress {
    pub run_id: String,
    pub iteration: u32,
    pub request_index: usize,
    pub total_requests: usize,
    pub result: RequestRunResult,
}

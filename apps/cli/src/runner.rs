use apiark_core::{
    http::cookies::CookieJarManager,
    runner::{RunConfig, RunSummary},
};
use std::path::Path;
pub async fn run_collection(
    path: &Path,
    env: Option<&str>,
    delay: u64,
    iterations: u32,
    data: Option<&str>,
) -> anyhow::Result<RunSummary> {
    let config = RunConfig {
        collection_path: path.to_string_lossy().into(),
        folder_path: None,
        environment_name: env.map(str::to_string),
        delay_ms: delay,
        iterations,
        data_file: data.map(str::to_string),
        stop_on_error: false,
    };
    apiark_core::runner::run_collection(
        config,
        &CookieJarManager::new(),
        |_| Ok(()),
        |p| {
            eprintln!(
                "[{}] {} {}",
                if p.result.passed { "PASS" } else { "FAIL" },
                p.result.method,
                p.result.name
            );
        },
    )
    .await
    .map_err(anyhow::Error::msg)
}

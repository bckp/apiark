use super::{RunConfig, RunSummary};
use crate::{http::cookies::CookieJarManager, oauth::OAuthTokenStore, storage::history::HistoryDb};
use std::sync::Arc;
use tauri::{AppHandle, Emitter};
pub async fn run_collection(
    app: AppHandle,
    config: RunConfig,
    _history_db: Arc<HistoryDb>,
    oauth: &OAuthTokenStore,
    jar: &CookieJarManager,
) -> Result<RunSummary, String> {
    let result = apiark_core::runner::run_collection(
        config,
        jar,
        |p| crate::commands::http::resolve_oauth_token(p, oauth),
        |p| {
            let _ = app.emit("runner:progress", p);
        },
    )
    .await?;
    let _ = app.emit("runner:complete", &result);
    Ok(result)
}

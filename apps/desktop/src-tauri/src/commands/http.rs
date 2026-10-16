use crate::{
    commands::history::AppState,
    http::cookies::CookieJarManager,
    models::{auth::AuthConfig, request::SendRequestParams, response::ResponseData},
    oauth::OAuthTokenStore,
};
pub use apiark_core::execution::ScriptedResponseData;
use apiark_core::execution::{self, Scripts};
use std::collections::HashMap;
use tauri::State;

#[tauri::command]
pub async fn send_request(
    state: State<'_, AppState>,
    oauth_store: State<'_, OAuthTokenStore>,
    cookie_jar: State<'_, CookieJarManager>,
    params: SendRequestParams,
    variables: Option<HashMap<String, String>>,
    collection_path: Option<String>,
    request_name: Option<String>,
) -> Result<ResponseData, String> {
    let result = send_request_with_scripts(
        state,
        oauth_store,
        cookie_jar,
        params,
        variables,
        collection_path,
        request_name,
        None,
        None,
        None,
        None,
    )
    .await?;
    Ok(ResponseData {
        status: result.status,
        status_text: result.status_text,
        headers: result.headers,
        cookies: result.cookies,
        body: result.body,
        time_ms: result.time_ms,
        size_bytes: result.size_bytes,
        truncated: result.truncated,
        full_size: result.full_size,
        temp_path: result.temp_path,
    })
}

#[allow(clippy::too_many_arguments)]
#[tauri::command]
pub async fn send_request_with_scripts(
    state: State<'_, AppState>,
    oauth_store: State<'_, OAuthTokenStore>,
    cookie_jar: State<'_, CookieJarManager>,
    params: SendRequestParams,
    variables: Option<HashMap<String, String>>,
    collection_path: Option<String>,
    request_name: Option<String>,
    pre_request_script: Option<String>,
    post_response_script: Option<String>,
    test_script: Option<String>,
    assertions_yaml: Option<String>,
) -> Result<ScriptedResponseData, String> {
    let mut vars = variables.unwrap_or_default();
    let result = execution::execute(
        params.clone(),
        &mut vars,
        collection_path.as_deref(),
        &cookie_jar,
        Scripts {
            pre_request: pre_request_script,
            post_response: post_response_script,
            tests: test_script,
            assertions_yaml,
        },
        |p| resolve_oauth_token(p, &oauth_store),
    )
    .await;
    let response = result.as_ref().ok();
    let entry = crate::storage::history::HistoryEntry {
        id: 0,
        method: format!("{:?}", params.method),
        url: apiark_core::redaction::history_url(&params.url),
        status: response.map(|r| r.status as i32),
        status_text: response.map(|r| r.status_text.clone()),
        time_ms: response.map(|r| r.time_ms as i64),
        size_bytes: response.map(|r| r.size_bytes as i64),
        timestamp: chrono::Utc::now().to_rfc3339(),
        collection_path,
        request_name,
        request_json: apiark_core::redaction::history_request(&params),
    };
    state.history_db.insert(&entry)?;
    result
}

#[tauri::command]
pub async fn read_full_response(_path: String) -> Result<String, String> {
    Err("Responses are bounded and returned in full; arbitrary file reads are disabled".into())
}
pub(crate) fn resolve_oauth_token(
    params: &mut SendRequestParams,
    store: &OAuthTokenStore,
) -> Result<(), String> {
    if let Some(AuthConfig::Oauth2 {
        client_id,
        auth_url,
        ..
    }) = &params.auth
    {
        let key = OAuthTokenStore::cache_key(client_id, auth_url);
        match store.get(&key) {
            Some(token) if !token.is_expired() => {
                params.auth = Some(AuthConfig::Bearer {
                    token: token.access_token,
                });
                Ok(())
            }
            Some(_) => Err(
                "OAuth token has expired. Please click \"Get Token\" to re-authenticate."
                    .to_string(),
            ),
            None => Err(
                "No OAuth token found. Please click \"Get Token\" to authenticate first."
                    .to_string(),
            ),
        }
    } else {
        Ok(())
    }
}

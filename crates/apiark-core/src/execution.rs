use crate::http::{client::HttpEngine, cookies::CookieJarManager, interpolation};
use crate::models::collection::RequestFile;
use crate::models::{
    auth::AuthConfig,
    error::HttpError,
    request::{BodyType, KeyValuePair, RequestBody, SendRequestParams},
    response::ResponseData,
};
use crate::scripting::{assertions::evaluate_assertions_from_yaml, engine::execute_script, *};
use crate::storage::collection;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct ScriptedResponseData {
    pub visualizations: Vec<crate::scripting::visualization::Visualization>,
    // Base response fields (flattened)
    pub status: u16,
    pub status_text: String,
    pub headers: Vec<KeyValuePair>,
    pub cookies: Vec<crate::models::response::CookieData>,
    pub body: String,
    pub time_ms: u64,
    pub size_bytes: u64,

    // Truncation fields
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub truncated: Option<bool>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub full_size: Option<u64>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub temp_path: Option<String>,

    // Scripting results
    pub test_results: Vec<TestResult>,
    pub assertion_results: Vec<AssertionResult>,
    pub console_output: Vec<ConsoleEntry>,
    pub env_mutations: HashMap<String, Option<String>>,
}

#[derive(Default)]
pub struct Scripts {
    pub pre_request: Option<String>,
    pub post_response: Option<String>,
    pub tests: Option<String>,
    pub assertions_yaml: Option<String>,
}

/// The single execution pipeline used by desktop requests, collection runs and CLI.
pub async fn execute(
    params: SendRequestParams,
    variables: &mut HashMap<String, String>,
    collection_path: Option<&str>,
    cookie_jar: &CookieJarManager,
    scripts: Scripts,
    resolve_auth: impl Fn(&mut SendRequestParams) -> Result<(), String>,
) -> Result<ScriptedResponseData, String> {
    let Scripts {
        pre_request: pre_request_script,
        post_response: post_response_script,
        tests: test_script,
        assertions_yaml,
    } = scripts;
    let mut vars = variables.clone();
    let mut interpolated = interpolate_params(&params, &vars);

    let defaults = collection_path
        .and_then(|p| collection::load_collection_config(std::path::Path::new(p)).ok())
        .map(|c| c.defaults)
        .unwrap_or_default();
    if interpolated.auth.is_none() {
        interpolated.auth = defaults.auth.as_ref().map(|a| interpolate_auth(a, &vars));
    }
    resolve_auth(&mut interpolated)?;
    let mut all_console: Vec<ConsoleEntry> = Vec::new();
    let mut all_tests: Vec<TestResult> = Vec::new();
    let mut visualizations = Vec::new();
    let mut script_failed = false;
    let mut env_mutations: HashMap<String, Option<String>> = HashMap::new();

    // 1. Execute pre-request script (if any)
    if let Some(ref script) = pre_request_script {
        if !script.trim().is_empty() {
            let snapshot = request_snapshot_from_params(&interpolated);
            let ctx = ScriptContext {
                request: snapshot,
                response: None,
                env: vars.clone(),
                globals: HashMap::new(),
                variables: HashMap::new(),
            };

            let result = execute_script(script, ctx, ScriptPhase::PreRequest)
                .map_err(|e| format!("Pre-request script error: {e}"))?;

            // Apply request mutations from pre-request script
            interpolated.url = result.request.url;
            interpolated.method = match result.request.method.as_str() {
                "GET" => crate::models::request::HttpMethod::GET,
                "POST" => crate::models::request::HttpMethod::POST,
                "PUT" => crate::models::request::HttpMethod::PUT,
                "PATCH" => crate::models::request::HttpMethod::PATCH,
                "DELETE" => crate::models::request::HttpMethod::DELETE,
                "HEAD" => crate::models::request::HttpMethod::HEAD,
                "OPTIONS" => crate::models::request::HttpMethod::OPTIONS,
                _ => interpolated.method,
            };
            // Apply header mutations
            interpolated.headers = result
                .request
                .headers
                .iter()
                .map(|(k, v)| KeyValuePair::new(k.clone(), v.clone(), true))
                .collect();
            // Apply body mutation
            if let Some(body_str) = result.request.body {
                if let Some(ref mut body) = interpolated.body {
                    body.content = body_str;
                }
            }

            // Apply env mutations
            apply_env_mutations(&mut vars, &result.env_mutations);
            env_mutations.extend(result.env_mutations);
            all_console.extend(result.console_output);
            all_tests.extend(result.test_results);
        }
    }

    interpolated = interpolate_params(&interpolated, &vars);
    // 2. Send the HTTP request
    let response = HttpEngine::send_with_jar(
        interpolated.clone(),
        cookie_jar,
        collection_path.unwrap_or(""),
        &defaults,
    )
    .await
    .map_err(|e| {
        let http_error: HttpError = e.into();
        serde_json::to_string(&http_error).unwrap_or(http_error.message)
    })?;

    // Build response snapshot for scripts
    let resp_snapshot = response_snapshot_from_data(&response);

    // Collect env mutations from all post-response scripts

    // 3. Execute post-response script (if any)
    if let Some(ref script) = post_response_script {
        if !script.trim().is_empty() {
            let snapshot = request_snapshot_from_params(&interpolated);
            let ctx = ScriptContext {
                request: snapshot,
                response: Some(resp_snapshot.clone()),
                env: vars.clone(),
                globals: HashMap::new(),
                variables: HashMap::new(),
            };

            match execute_script(script, ctx, ScriptPhase::PostResponse) {
                Ok(result) => {
                    visualizations.extend(result.visualizations);
                    apply_env_mutations(&mut vars, &result.env_mutations);
                    env_mutations.extend(result.env_mutations);
                    all_console.extend(result.console_output);
                    all_tests.extend(result.test_results);
                }
                Err(e) => {
                    script_failed = true;
                    all_tests.push(TestResult {
                        name: "Script execution".into(),
                        passed: false,
                        error: Some(e.to_string()),
                    });
                    all_console.push(ConsoleEntry {
                        level: "error".to_string(),
                        message: format!("Post-response script error: {e}"),
                    });
                }
            }
        }
    }

    // 4. Evaluate declarative assertions (if any)
    let mut assertion_results = Vec::new();
    if let Some(ref yaml_str) = assertions_yaml {
        if !yaml_str.trim().is_empty() {
            match serde_yaml::from_str(yaml_str) {
                Ok(yaml_value) => {
                    assertion_results = evaluate_assertions_from_yaml(&yaml_value, &resp_snapshot);
                    if assertion_results.is_empty() {
                        all_tests.push(TestResult {
                            name: "Assertions".into(),
                            passed: false,
                            error: Some(
                                "Nonempty assertion block produced no supported assertions".into(),
                            ),
                        });
                    }
                }
                Err(e) => {
                    script_failed = true;
                    all_tests.push(TestResult {
                        name: "Script execution".into(),
                        passed: false,
                        error: Some(e.to_string()),
                    });
                    all_console.push(ConsoleEntry {
                        level: "error".to_string(),
                        message: format!("Failed to parse assertions YAML: {e}"),
                    });
                }
            }
        }
    }

    // 5. Execute test script (if any)
    if let Some(ref script) = test_script {
        if !script.trim().is_empty() {
            let snapshot = request_snapshot_from_params(&interpolated);
            let ctx = ScriptContext {
                request: snapshot,
                response: Some(resp_snapshot.clone()),
                env: vars.clone(),
                globals: HashMap::new(),
                variables: HashMap::new(),
            };

            match execute_script(script, ctx, ScriptPhase::PostResponse) {
                Ok(result) => {
                    visualizations.extend(result.visualizations);
                    apply_env_mutations(&mut vars, &result.env_mutations);
                    env_mutations.extend(result.env_mutations);
                    all_console.extend(result.console_output);
                    all_tests.extend(result.test_results);
                }
                Err(e) => {
                    script_failed = true;
                    all_tests.push(TestResult {
                        name: "Script execution".into(),
                        passed: false,
                        error: Some(e.to_string()),
                    });
                    all_console.push(ConsoleEntry {
                        level: "error".to_string(),
                        message: format!("Test script error: {e}"),
                    });
                }
            }
        }
    }

    if !script_failed {
        match crate::scripting::visualization::validate(Some(&serde_json::to_value(&visualizations).map_err(|e|e.to_string())?)) {
            Ok(validated) => visualizations = validated,
            Err(e) => { script_failed = true; all_tests.push(TestResult {name:"Visualizations".into(),passed:false,error:Some(e)}); }
        }
    }
    if script_failed { visualizations.clear(); }
    *variables = vars;
    Ok(ScriptedResponseData {
        visualizations,
        status: response.status,
        status_text: response.status_text,
        headers: response.headers,
        cookies: response.cookies,
        body: response.body,
        time_ms: response.time_ms,
        size_bytes: response.size_bytes,
        truncated: response.truncated,
        full_size: response.full_size,
        temp_path: response.temp_path,
        test_results: all_tests,
        assertion_results,
        console_output: all_console,
        env_mutations,
    })
}

pub fn interpolate_params(
    params: &SendRequestParams,
    vars: &HashMap<String, String>,
) -> SendRequestParams {
    let mut interpolated = params.clone();
    interpolated.url = interpolation::interpolate(&params.url, vars);

    interpolated.headers = params
        .headers
        .iter()
        .map(|h| {
            KeyValuePair::new(
                interpolation::interpolate(&h.key, vars),
                interpolation::interpolate(&h.value, vars),
                h.enabled,
            )
        })
        .collect();

    interpolated.params = params
        .params
        .iter()
        .map(|p| {
            KeyValuePair::new(
                interpolation::interpolate(&p.key, vars),
                interpolation::interpolate(&p.value, vars),
                p.enabled,
            )
        })
        .collect();

    if let Some(ref body) = params.body {
        let mut new_body = body.clone();
        new_body.content = interpolation::interpolate(&body.content, vars);
        new_body.form_data = body
            .form_data
            .iter()
            .map(|fd| {
                let mut kv = KeyValuePair::new(
                    interpolation::interpolate(&fd.key, vars),
                    interpolation::interpolate(&fd.value, vars),
                    fd.enabled,
                );
                kv.value_type = fd.value_type.clone();
                kv
            })
            .collect();
        interpolated.body = Some(new_body);
    }

    if let Some(ref auth) = params.auth {
        interpolated.auth = Some(interpolate_auth(auth, vars));
    }

    interpolated
}

fn request_snapshot_from_params(params: &SendRequestParams) -> RequestSnapshot {
    RequestSnapshot {
        method: format!("{:?}", params.method),
        url: params.url.clone(),
        headers: params
            .headers
            .iter()
            .filter(|h| h.enabled)
            .map(|h| (h.key.clone(), h.value.clone()))
            .collect(),
        body: params.body.as_ref().map(|b| b.content.clone()),
    }
}

fn response_snapshot_from_data(response: &ResponseData) -> ResponseSnapshot {
    ResponseSnapshot {
        status: response.status,
        status_text: response.status_text.clone(),
        headers: response
            .headers
            .iter()
            .map(|h| (h.key.clone(), h.value.clone()))
            .collect(),
        body: response.body.clone(),
        time_ms: response.time_ms,
        size_bytes: response.size_bytes,
    }
}

fn apply_env_mutations(
    vars: &mut HashMap<String, String>,
    mutations: &HashMap<String, Option<String>>,
) {
    for (key, value) in mutations {
        match value {
            Some(v) => {
                vars.insert(key.clone(), v.clone());
            }
            None => {
                vars.remove(key);
            }
        }
    }
}

fn interpolate_auth(auth: &AuthConfig, vars: &HashMap<String, String>) -> AuthConfig {
    match auth {
        AuthConfig::None => AuthConfig::None,
        AuthConfig::Bearer { token } => AuthConfig::Bearer {
            token: interpolation::interpolate(token, vars),
        },
        AuthConfig::Basic { username, password } => AuthConfig::Basic {
            username: interpolation::interpolate(username, vars),
            password: interpolation::interpolate(password, vars),
        },
        AuthConfig::ApiKey { key, value, add_to } => AuthConfig::ApiKey {
            key: interpolation::interpolate(key, vars),
            value: interpolation::interpolate(value, vars),
            add_to: add_to.clone(),
        },
        AuthConfig::Oauth2 {
            grant_type,
            auth_url,
            token_url,
            client_id,
            client_secret,
            scope,
            callback_url,
            username,
            password,
            use_pkce,
        } => AuthConfig::Oauth2 {
            grant_type: grant_type.clone(),
            auth_url: interpolation::interpolate(auth_url, vars),
            token_url: interpolation::interpolate(token_url, vars),
            client_id: interpolation::interpolate(client_id, vars),
            client_secret: interpolation::interpolate(client_secret, vars),
            scope: interpolation::interpolate(scope, vars),
            callback_url: interpolation::interpolate(callback_url, vars),
            username: interpolation::interpolate(username, vars),
            password: interpolation::interpolate(password, vars),
            use_pkce: *use_pkce,
        },
        AuthConfig::Digest { username, password } => AuthConfig::Digest {
            username: interpolation::interpolate(username, vars),
            password: interpolation::interpolate(password, vars),
        },
        AuthConfig::AwsV4 {
            access_key,
            secret_key,
            region,
            service,
            session_token,
        } => AuthConfig::AwsV4 {
            access_key: interpolation::interpolate(access_key, vars),
            secret_key: interpolation::interpolate(secret_key, vars),
            region: interpolation::interpolate(region, vars),
            service: interpolation::interpolate(service, vars),
            session_token: interpolation::interpolate(session_token, vars),
        },
        AuthConfig::JwtBearer {
            secret,
            algorithm,
            payload,
            header_prefix,
        } => AuthConfig::JwtBearer {
            secret: interpolation::interpolate(secret, vars),
            algorithm: algorithm.clone(),
            payload: interpolation::interpolate(payload, vars),
            header_prefix: header_prefix.clone(),
        },
        AuthConfig::Ntlm {
            username,
            password,
            domain,
            workstation,
        } => AuthConfig::Ntlm {
            username: interpolation::interpolate(username, vars),
            password: interpolation::interpolate(password, vars),
            domain: interpolation::interpolate(domain, vars),
            workstation: interpolation::interpolate(workstation, vars),
        },
        AuthConfig::Saml {
            idp_url,
            entity_id,
            assertion_consumer_url,
            certificate,
            name_id_format,
            saml_token,
        } => AuthConfig::Saml {
            idp_url: interpolation::interpolate(idp_url, vars),
            entity_id: interpolation::interpolate(entity_id, vars),
            assertion_consumer_url: interpolation::interpolate(assertion_consumer_url, vars),
            certificate: interpolation::interpolate(certificate, vars),
            name_id_format: name_id_format.clone(),
            saml_token: interpolation::interpolate(saml_token, vars),
        },
    }
}

/// Convert a RequestFile from disk to SendRequestParams for the HTTP engine.
pub fn request_file_to_params(file: &RequestFile) -> SendRequestParams {
    let headers: Vec<KeyValuePair> = file
        .headers
        .iter()
        .map(|(k, v)| KeyValuePair::new(k.clone(), v.clone(), true))
        .collect();

    let params: Vec<KeyValuePair> = file
        .params
        .as_ref()
        .map(|p| {
            p.iter()
                .map(|(k, v)| KeyValuePair::new(k.clone(), v.clone(), true))
                .collect()
        })
        .unwrap_or_default();

    let body = file.body.as_ref().map(|b| {
        let body_type = match b.body_type.as_str() {
            "json" => BodyType::Json,
            "xml" => BodyType::Xml,
            "raw" => BodyType::Raw,
            "urlencoded" => BodyType::Urlencoded,
            "form-data" => BodyType::FormData,
            "binary" => BodyType::Binary,
            _ => BodyType::None,
        };
        RequestBody {
            body_type,
            content: b.content.clone(),
            form_data: Vec::new(),
        }
    });

    SendRequestParams {
        method: file.method.clone(),
        url: file.url.clone(),
        headers,
        params,
        body,
        auth: file.auth.clone(),
        proxy: None,
        timeout_ms: None,
        follow_redirects: true,
        verify_ssl: true,
        cookies: file.cookies.clone(),
        ca_cert_path: None,
        client_cert_path: None,
        client_key_path: None,
        client_cert_passphrase: None,
    }
}

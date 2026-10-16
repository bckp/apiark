use apiark_core::{
    execution::{execute, Scripts},
    http::{
        client::HttpEngine,
        cookies::{CookieEntry, CookieJarManager},
        response_parser::parse_set_cookie_header,
    },
    models::{collection::CollectionDefaults, request::SendRequestParams},
    scripting::{engine::execute_script, RequestSnapshot, ScriptContext, ScriptPhase},
};
use axum::{
    response::IntoResponse,
    routing::{get, post},
    Router,
};
use std::collections::HashMap;

fn params(url: &str) -> SendRequestParams {
    serde_json::from_value(serde_json::json!({"method":"GET", "url":url})).unwrap()
}

async fn server(app: Router) -> (String, tokio::task::JoinHandle<()>) {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let addr = listener.local_addr().unwrap();
    let task = tokio::spawn(async move { axum::serve(listener, app).await.unwrap() });
    (format!("http://127.0.0.1:{}", addr.port()), task)
}

#[test]
fn cookie_boundaries() {
    let url = url::Url::parse("https://api.example.test/Api/items").unwrap();
    let c = parse_set_cookie_header("sid=synthetic; Secure; HttpOnly", &url).unwrap();
    assert!(c.host_only);
    assert_eq!(c.domain.as_deref(), Some("api.example.test"));
    assert_eq!(c.path.as_deref(), Some("/Api"));
    let e = CookieEntry::from(&c);
    for blocked in [
        "https://other.test/Api",
        "https://sub.api.example.test/Api",
        "http://api.example.test/Api",
        "https://api.example.test/api",
        "https://api.example.test/Apix",
    ] {
        assert!(!e.matches(&url::Url::parse(blocked).unwrap()), "{blocked}");
    }
    assert!(e.matches(&url));
    let c = parse_set_cookie_header("sid=x; Domain=example.test; Path=/", &url).unwrap();
    assert!(CookieEntry::from(&c).matches(&url));
    assert!(!CookieEntry::from(&c).matches(&url::Url::parse("https://badexample.test/").unwrap()));
    assert!(parse_set_cookie_header("sid=x; Domain=com", &url).is_none());
    assert!(parse_set_cookie_header("sid=x; Domain=other.test", &url).is_none());
    let jar = CookieJarManager::new();
    jar.store_cookies("test", vec![CookieEntry::from(&c)])
        .unwrap();
    let deletion =
        parse_set_cookie_header("sid=gone; Domain=example.test; Path=/; Max-Age=0", &url).unwrap();
    jar.store_cookies("test", vec![CookieEntry::from(&deletion)])
        .unwrap();
    assert!(jar.cookie_header("test", &url).unwrap().is_empty());
    let legacy: CookieEntry = serde_json::from_value(serde_json::json!({"name":"sid","value":"x","domain":"","path":"/","expires":null,"httpOnly":false,"secure":false,"sameSite":null})).unwrap();
    assert!(legacy.host_only);
    assert!(!legacy.matches(&url));
}

#[tokio::test]
async fn redirects_keep_methods_bodies_and_scope_cookies() {
    let app = Router::new()
        .route(
            "/redirect307",
            post(|| async {
                (
                    axum::http::StatusCode::TEMPORARY_REDIRECT,
                    [("location", "/echo")],
                )
            }),
        )
        .route(
            "/redirect308",
            post(|| async {
                (
                    axum::http::StatusCode::PERMANENT_REDIRECT,
                    [("location", "/echo")],
                )
            }),
        )
        .route("/echo", post(|body: String| async move { body }))
        .route(
            "/cookie",
            get(|headers: axum::http::HeaderMap| async move {
                headers
                    .get("cookie")
                    .and_then(|h| h.to_str().ok())
                    .unwrap_or("")
                    .to_string()
            }),
        );
    let (base, task) = server(app).await;
    for status in [307, 308] {
        let p: SendRequestParams = serde_json::from_value(serde_json::json!({"method":"POST","url":format!("{base}/redirect{status}"),"body":{"type":"json","content":"{\"synthetic\":true}"}})).unwrap();
        let response = HttpEngine::send(p).await.unwrap();
        assert_eq!(response.body, "{\"synthetic\":true}");
        let p: SendRequestParams = serde_json::from_value(serde_json::json!({"method":"POST","url":format!("{base}/redirect{status}"),"body":{"type":"form-data","formData":[{"key":"field","value":"synthetic"}]}})).unwrap();
        assert!(HttpEngine::send(p)
            .await
            .unwrap()
            .body
            .contains("synthetic"));
    }
    let destination = format!("{base}/cookie");
    let (origin, source_task) = server(Router::new().route(
        "/",
        get(move || {
            let dest = destination.clone();
            async move {
                (
                    axum::http::StatusCode::FOUND,
                    [
                        ("location", dest),
                        ("set-cookie", "sid=synthetic; Path=/".into()),
                    ],
                )
            }
        }),
    ))
    .await;
    let source = origin.replace("127.0.0.1", "localhost");
    let mut p = params(&format!("{source}/"));
    p.headers
        .push(apiark_core::models::request::KeyValuePair::new(
            "Authorization".into(),
            "Bearer synthetic".into(),
            true,
        ));
    p.cookies = Some(HashMap::from([("explicit".into(), "secret".into())]));
    let jar = CookieJarManager::new();
    let r = HttpEngine::send_with_jar(p, &jar, "test", &CollectionDefaults::default())
        .await
        .unwrap();
    assert_eq!(r.body, "");
    assert_eq!(jar.get_cookies("test").unwrap()[0].domain, "localhost");
    task.abort();
    source_task.abort();
}

#[tokio::test]
async fn scripts_share_mutations_and_fail_closed() {
    let (base, task) = server(Router::new().route(
        "/",
        get(|headers: axum::http::HeaderMap| async move {
            if headers.get("x-pre").is_some() {
                "ok"
            } else {
                "missing"
            }
        }),
    ))
    .await;
    let jar = CookieJarManager::new();
    let mut vars = HashMap::new();
    let result = execute(params(&format!("{base}/")), &mut vars, None, &jar, Scripts {
        pre_request: Some("ark.env.set('v','set'); ark.request.setHeader('x-pre','yes');".into()),
        tests: Some("ark.test('pre ran',()=>{ark.expect(ark.response.body).to.equal('ok');ark.expect(ark.env.get('v')).to.equal('set')}); ark.test('failure',()=>{throw new Error('expected')});".into()),
        ..Scripts::default()
    }, |_| Ok(())).await.unwrap();
    assert_eq!(result.test_results.len(), 2);
    assert!(result.test_results[0].passed);
    assert!(!result.test_results[1].passed);
    let failed = execute(
        params(&base),
        &mut vars,
        None,
        &jar,
        Scripts {
            tests: Some("pm.test('unsupported',()=>{});".into()),
            ..Scripts::default()
        },
        |_| Ok(()),
    )
    .await
    .unwrap();
    assert!(failed.test_results.iter().any(|t| !t.passed));
    task.abort();
}

#[tokio::test]
async fn pre_request_url_change_cannot_leak_stored_cookie() {
    let (base, task) = server(Router::new().route(
        "/",
        get(|headers: axum::http::HeaderMap| async move {
            headers
                .get("cookie")
                .and_then(|h| h.to_str().ok())
                .unwrap_or("")
                .to_string()
        }),
    ))
    .await;
    let source = base.replace("127.0.0.1", "localhost");
    let jar = CookieJarManager::new();
    let c = parse_set_cookie_header("sid=synthetic", &url::Url::parse(&source).unwrap()).unwrap();
    jar.store_cookies("", vec![CookieEntry::from(&c)]).unwrap();
    let response = execute(
        params(&source),
        &mut HashMap::new(),
        None,
        &jar,
        Scripts {
            pre_request: Some(format!(
                "ark.request.setUrl({});",
                serde_json::to_string(&base).unwrap()
            )),
            ..Scripts::default()
        },
        |_| Ok(()),
    )
    .await
    .unwrap();
    assert!(response.body.is_empty());
    task.abort();
}

#[tokio::test]
async fn unknown_length_response_is_bounded() {
    let (base, task) = server(Router::new().route(
        "/",
        get(|| async {
            // Deliberately omit Content-Length and stream more than the limit.
            let stream = futures_util::stream::iter(
                (0..12).map(|_| Ok::<_, std::io::Error>(vec![b'x'; 1024 * 1024])),
            );
            axum::body::Body::from_stream(stream).into_response()
        }),
    ))
    .await;
    assert!(HttpEngine::send(params(&base))
        .await
        .unwrap_err()
        .to_string()
        .contains("Response too large"));
    task.abort();
}

#[test]
fn infinite_scripts_and_async_tests_cannot_pass() {
    let context = || ScriptContext {
        request: RequestSnapshot {
            method: "GET".into(),
            url: "http://example.test".into(),
            headers: HashMap::new(),
            body: None,
        },
        response: None,
        env: HashMap::new(),
        globals: HashMap::new(),
        variables: HashMap::new(),
    };
    let start = std::time::Instant::now();
    assert!(execute_script("while(true){}", context(), ScriptPhase::PreRequest).is_err());
    assert!(start.elapsed().as_secs() < 5);
    let result = execute_script(
        "ark.test('async',async ()=>{throw new Error('failure')})",
        context(),
        ScriptPhase::PreRequest,
    );
    assert!(result.is_err());
    assert!(execute_script(
        "Promise.resolve().then(()=>ark.test('hidden',()=>{throw Error('failure')}))",
        context(),
        ScriptPhase::PreRequest
    )
    .is_err());
    assert!(execute_script(
        "ark.test('hidden',()=>{Promise.resolve().then(()=>{throw Error('failure')})})",
        context(),
        ScriptPhase::PreRequest
    )
    .is_err());
}

#[tokio::test]
async fn explicit_cookies_override_names_without_removing_other_jar_cookies() {
    let (base, task) = server(Router::new().route(
        "/",
        get(|headers: axum::http::HeaderMap| async move {
            headers.get("cookie").unwrap().to_str().unwrap().to_string()
        }),
    ))
    .await;
    let jar = CookieJarManager::new();
    for h in ["sid=old; Path=/", "other=retained; Path=/"] {
        let c = parse_set_cookie_header(h, &url::Url::parse(&base).unwrap()).unwrap();
        jar.store_cookies("", vec![CookieEntry::from(&c)]).unwrap();
    }
    let mut p = params(&base);
    p.cookies = Some(HashMap::from([("sid".into(), "override".into())]));
    let response = HttpEngine::send_with_jar(p, &jar, "", &CollectionDefaults::default())
        .await
        .unwrap();
    assert!(response.body.contains("sid=override"));
    assert!(response.body.contains("other=retained"));
    assert!(!response.body.contains("sid=old"));
    task.abort();
}

#[tokio::test]
async fn pre_request_variables_are_interpolated_after_mutation_in_all_request_fields() {
    let (base, task) = server(Router::new().route(
        "/new",
        post(|headers: axum::http::HeaderMap, body: String| async move {
            format!(
                "{}|{}|{body}",
                headers["x-value"].to_str().unwrap(),
                headers["authorization"].to_str().unwrap()
            )
        }),
    ))
    .await;
    let p: SendRequestParams = serde_json::from_value(serde_json::json!({"method":"POST","url":format!("{base}/{{{{value}}}}"),"headers":[{"key":"x-value","value":"{{value}}"}],"body":{"type":"raw","content":"{{value}}"},"auth":{"type":"bearer","token":"{{value}}"}})).unwrap();
    let response = execute(
        p,
        &mut HashMap::from([("value".into(), "old".into())]),
        None,
        &CookieJarManager::new(),
        Scripts {
            pre_request: Some("ark.env.set('value','new')".into()),
            ..Scripts::default()
        },
        |_| Ok(()),
    )
    .await
    .unwrap();
    assert_eq!(response.body, "new|Bearer new|new");
    task.abort();
}

#[tokio::test]
async fn visualization_outputs_are_bounded_across_phases_and_discarded_on_script_error() {
    let (base, task) = server(Router::new().route("/", get(|| async { "ok" }))).await;
    for (post, tests, pass, count) in [
        (
            "ark.visualize('text',{data:'post'})",
            "ark.visualize('json',{data:{ok:true}})",
            true,
            2,
        ),
        (
            "ark.visualize('text',{data:'post'})",
            "throw new Error('synthetic')",
            false,
            0,
        ),
        (
            "for(let i=0;i<3;i++)ark.visualize('text',{data:'post'})",
            "for(let i=0;i<2;i++)ark.visualize('text',{data:'test'})",
            false,
            0,
        ),
    ] {
        let response = execute(
            params(&base),
            &mut HashMap::new(),
            None,
            &CookieJarManager::new(),
            Scripts {
                post_response: Some(post.into()),
                tests: Some(tests.into()),
                ..Scripts::default()
            },
            |_| Ok(()),
        )
        .await
        .unwrap();
        assert_eq!(response.body, "ok");
        assert_eq!(response.visualizations.len(), count);
        assert_eq!(response.test_results.iter().all(|t| t.passed), pass);
    }
    task.abort();
}

#[test]
fn invalid_authentication_cannot_silently_send_an_unauthenticated_request() {
    for auth in [
        serde_json::json!({"type":"jwt-bearer","secret":"synthetic","algorithm":"HS256","payload":"bad-json","headerPrefix":"Bearer"}),
        serde_json::json!({"type":"digest","username":"synthetic","password":"synthetic"}),
        serde_json::json!({"type":"ntlm","username":"synthetic","password":"synthetic","domain":"test","workstation":"test"}),
    ] {
        let p: SendRequestParams = serde_json::from_value(
            serde_json::json!({"method":"GET","url":"https://example.test/","auth":auth}),
        )
        .unwrap();
        assert!(
            apiark_core::http::request_builder::build_request(&reqwest::Client::new(), &p).is_err()
        );
    }
}

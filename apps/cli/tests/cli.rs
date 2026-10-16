#[tokio::test]
async fn exit_code_and_machine_reports_follow_shared_script_results() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    let server = tokio::spawn(async move {
        axum::serve(
            listener,
            axum::Router::new().route("/", axum::routing::get(|| async { "ok" })),
        )
        .await
        .unwrap()
    });
    let tmp = tempfile::tempdir().unwrap();
    std::fs::create_dir(tmp.path().join(".apiark")).unwrap();
    std::fs::write(
        tmp.path().join(".apiark/apiark.yaml"),
        "name: Synthetic\nversion: 1\n",
    )
    .unwrap();
    for (script, success) in [
        (
            "ark.test('ok',()=>ark.expect(ark.response.body).to.equal('ok'))",
            true,
        ),
        ("ark.test('failure',()=>ark.expect(1).to.equal(2))", false),
        ("pm.test('unsupported',()=>{})", false),
        (
            "Promise.resolve().then(()=>ark.test('hidden',()=>{throw Error('fail')}))",
            false,
        ),
    ] {
        std::fs::write(
            tmp.path().join("request.yaml"),
            format!("name: Test\nmethod: GET\nurl: {url}\ntests: {script:?}\n"),
        )
        .unwrap();
        let output = tokio::process::Command::new(env!("CARGO_BIN_EXE_apiark"))
            .args(["run", tmp.path().to_str().unwrap(), "--reporter", "json"])
            .output()
            .await
            .unwrap();
        assert_eq!(
            output.status.success(),
            success,
            "{}",
            String::from_utf8_lossy(&output.stderr)
        );
        let report: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
        assert_eq!(report["totalFailed"], if success { 0 } else { 1 });
    }
    server.abort();
}

#[tokio::test]
async fn folder_selection_and_invalid_arguments_cannot_run_the_wrong_requests() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}", listener.local_addr().unwrap());
    let server = tokio::spawn(async move {
        axum::serve(
            listener,
            axum::Router::new().route("/", axum::routing::get(|| async { "ok" })),
        )
        .await
        .unwrap()
    });
    let tmp = tempfile::tempdir().unwrap();
    std::fs::create_dir(tmp.path().join(".apiark")).unwrap();
    std::fs::write(
        tmp.path().join(".apiark/apiark.yaml"),
        "name: Synthetic\nversion: 1\n",
    )
    .unwrap();
    std::fs::write(
        tmp.path().join("failing.yaml"),
        format!(
            "name: Not selected\nmethod: GET\nurl: {url}\ntests: throw Error('must not run')\n"
        ),
    )
    .unwrap();
    std::fs::create_dir(tmp.path().join("selected")).unwrap();
    std::fs::write(
        tmp.path().join("selected/pass.yaml"),
        format!("name: Selected\nmethod: GET\nurl: {url}\n"),
    )
    .unwrap();
    let run = |args: Vec<String>| async move {
        tokio::process::Command::new(env!("CARGO_BIN_EXE_apiark"))
            .args(args)
            .output()
            .await
            .unwrap()
    };
    let root = tmp.path().to_str().unwrap();
    let output = run(vec![
        "run".into(),
        root.into(),
        "--folder".into(),
        "selected".into(),
        "--reporter".into(),
        "json".into(),
    ])
    .await;
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let report: serde_json::Value = serde_json::from_slice(&output.stdout).unwrap();
    assert_eq!(report["totalRequests"], 1);
    let output = run(vec![
        "run".into(),
        root.into(),
        "--folder".into(),
        "..".into(),
    ])
    .await;
    assert!(!output.status.success());
    let output = run(vec![
        "run".into(),
        root.into(),
        "--reporter".into(),
        "typo".into(),
    ])
    .await;
    assert_eq!(output.status.code(), Some(2));
    server.abort();
}

use apiark_core::{
    execution::{interpolate_params, request_file_to_params},
    exporter::postman::export_to_postman,
    http::request_builder::build_url_with_params,
    importer::{postman::parse_postman, writer::write_import},
    models::collection::CollectionNode,
    storage::collection::{load_collection_tree, read_request},
};
use serde_json::json;

fn fixture() -> serde_json::Value {
    json!({
    "info":{"name":"Round Trip","description":"Synthetic metadata","schema":"https://schema.getpostman.com/json/collection/v2.1.0/collection.json"},
    "variable":[{"key":"host","value":"http://example.test"},{"key":"number","value":12}],
    "auth":{"type":"bearer","bearer":[{"key":"token","value":"synthetic"}]},
    "event":[{"listen":"prerequest","script":{"exec":["ark.env.set('trace','C');"]}}],
    "item":[{"name":"Z first","request":{"method":"GET","url":"{{host}}/health"}},
    {"name":"Folder With Spaces","description":"folder metadata","event":[{"listen":"test","script":{"exec":["ark.test('scope',()=>ark.expect(ark.env.get('trace')).to.equal('C')); "]}}],"item":[
    {"name":"A","event":[{"listen":"test","script":{"exec":["pm.test('preserved',()=>{});"]}}],"response":[{"name":"Example","body":"synthetic"}],"request":{"method":"POST",
    "url":{"raw":"{{host}}/items/:id?tag=a&tag=b&off=x","variable":[{"key":"id","value":"a b/č"}],"query":[{"key":"tag","value":"a"},{"key":"tag","value":"b"},{"key":"off","value":"x","disabled":true}]},
    "header":[{"key":"X-Duplicate","value":"first"},{"key":"X-Duplicate","value":"last"},{"key":"X-Off","value":"retained","disabled":true}],
    "auth":{"type":"noauth"},"body":{"mode":"urlencoded","urlencoded":[{"key":"a&=+","value":"č & = +"}]}}},
    {"name":"A-1","request":{"method":"POST","url":"{{host}}/form","body":{"mode":"formdata","formdata":[{"key":"a","value":"text"},{"key":"off","value":"keep","disabled":true}]}}},
    {"name":"A","request":{"method":"GET","url":"{{host}}/second"}}
    ]}]})
}

#[test]
fn import_export_import_retains_requests_folders_urls_bodies_auth_variables_scripts_and_metadata() {
    let root = tempfile::tempdir().unwrap();
    let original = fixture();
    let data = parse_postman(&original.to_string()).unwrap();
    assert!(data.warnings.iter().any(|w| w.message.contains("pm.*")));
    assert!(data
        .warnings
        .iter()
        .any(|w| w.message.contains("Duplicate")));
    let path = write_import(&data, root.path()).unwrap();
    let export: serde_json::Value =
        serde_json::from_str(&export_to_postman(std::path::Path::new(&path)).unwrap()).unwrap();
    assert_eq!(export["info"], original["info"]);
    assert_eq!(export["variable"], original["variable"]);
    assert_eq!(export["event"], original["event"]);
    assert_eq!(export["item"][0]["name"], "Z first");
    assert_eq!(export["item"][1]["name"], "Folder With Spaces");
    for index in 0..3 {
        for key in ["header", "body", "auth"] {
            assert_eq!(
                export["item"][1]["item"][index]["request"][key],
                original["item"][1]["item"][index]["request"][key]
            );
        }
    }
    assert_eq!(
        export["item"][1]["item"][0]["event"],
        original["item"][1]["item"][0]["event"]
    );
    assert_eq!(
        export["item"][1]["item"][0]["response"],
        original["item"][1]["item"][0]["response"]
    );
    let reimport = parse_postman(&export.to_string()).unwrap();
    assert_eq!(reimport.preview().request_count, 4);
    assert_eq!(reimport.preview().folder_count, 1);
    let second = tempfile::tempdir().unwrap();
    let second_path = write_import(&reimport, second.path()).unwrap();
    let export_again: serde_json::Value =
        serde_json::from_str(&export_to_postman(std::path::Path::new(&second_path)).unwrap())
            .unwrap();
    assert_eq!(export_again, export);
    let tree = load_collection_tree(std::path::Path::new(&path)).unwrap();
    let CollectionNode::Collection { children, .. } = tree else {
        panic!()
    };
    let CollectionNode::Folder { children, .. } = &children[1] else {
        panic!()
    };
    let CollectionNode::Request { path, .. } = &children[0] else {
        panic!()
    };
    let file = read_request(std::path::Path::new(path)).unwrap();
    let params = interpolate_params(
        &request_file_to_params(&file),
        &std::collections::HashMap::from([("host".into(), "http://example.test".into())]),
    );
    let url = build_url_with_params(&params.url, &params.params, &params.auth).unwrap();
    assert_eq!(url.path(), "/items/a%20b%2F%C4%8D");
    assert_eq!(url.query(), Some("tag=a&tag=b"));
    assert_eq!(params.body.unwrap().form_data[0].value, "č & = +");
}

#[test]
fn unsupported_auth_body_and_methods_abort_instead_of_dropping_data() {
    for request in [
        json!({"method":"TRACE","url":"http://example.test"}),
        json!({"method":"GET","url":"http://example.test","auth":{"type":"oauth1"}}),
        json!({"method":"POST","url":"http://example.test","body":{"mode":"file","file":{"src":"synthetic.bin"}}}),
    ] {
        assert!(parse_postman(
            &json!({"info":{"name":"Rejected"},"item":[{"name":"Rejected","request":request}]})
                .to_string()
        )
        .is_err());
    }
}

#[tokio::test]
async fn scoped_scripts_variables_and_explicit_noauth_run_identically_in_core() {
    use axum::{http::HeaderMap, routing::get, Router};
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let address = format!("http://{}", listener.local_addr().unwrap());
    let server = tokio::spawn(async move {
        axum::serve(
            listener,
            Router::new().route(
                "/",
                get(|headers: HeaderMap| async move {
                    assert!(!headers.contains_key("authorization"));
                    headers["x-trace"].to_str().unwrap().to_string()
                }),
            ),
        )
        .await
        .unwrap()
    });
    let value = json!({"info":{"name":"Scope"},"variable":[{"key":"base","value":address}],
        "auth":{"type":"bearer","bearer":[{"key":"token","value":"must-not-send"}]},
        "event":[{"listen":"prerequest","script":{"exec":["ark.env.set('trace','C');"]}}],
        "item":[{"name":"Folder","event":[{"listen":"prerequest","script":{"exec":["ark.env.set('trace',ark.env.get('trace')+'F');"]}},{"listen":"test","script":{"exec":["ark.test('scope ran',()=>ark.expect(ark.response.body).to.equal('CFR'));"]}}],
            "item":[{"name":"Request","event":[{"listen":"prerequest","script":{"exec":["ark.request.setHeader('x-trace',ark.env.get('trace')+'R');"]}}],"request":{"method":"GET","url":"{{base}}/","auth":{"type":"noauth"}}}]}]});
    let tmp = tempfile::tempdir().unwrap();
    let path = write_import(&parse_postman(&value.to_string()).unwrap(), tmp.path()).unwrap();
    let result = apiark_core::runner::run_collection(
        apiark_core::runner::RunConfig {
            collection_path: path,
            folder_path: None,
            environment_name: None,
            delay_ms: 0,
            iterations: 1,
            data_file: None,
            stop_on_error: false,
        },
        &apiark_core::http::cookies::CookieJarManager::new(),
        |_| Ok(()),
        |_| {},
    )
    .await
    .unwrap();
    assert_eq!(result.total_failed, 0);
    assert_eq!(result.iterations[0].results[0].test_count, 1);
    server.abort();
}

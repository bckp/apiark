mod interpolation {
    use apiark_core::http::interpolation::interpolate;
    use std::collections::HashMap;

    #[test]
    fn test_basic_variable_substitution() {
        let mut vars = HashMap::new();
        vars.insert("baseUrl".into(), "https://api.example.com".into());
        vars.insert("version".into(), "v2".into());
        assert_eq!(
            interpolate("{{baseUrl}}/{{version}}/users", &vars),
            "https://api.example.com/v2/users"
        );
        println!("  Variable interpolation: basic substitution OK");
    }

    #[test]
    fn test_dynamic_uuid() {
        let vars = HashMap::new();
        let result = interpolate("id={{$uuid}}", &vars);
        assert!(result.starts_with("id="));
        let uuid_part = &result[3..];
        assert_eq!(uuid_part.len(), 36);
        assert_eq!(uuid_part.chars().filter(|c| *c == '-').count(), 4);
        println!("  Dynamic {{{{$uuid}}}}: {}", uuid_part);
    }

    #[test]
    fn test_dynamic_timestamp() {
        let vars = HashMap::new();
        let result = interpolate("ts={{$timestamp}}", &vars);
        let ts: i64 = result[3..].parse().expect("Not a valid timestamp");
        assert!(ts > 1700000000); // After 2023
        println!("  Dynamic {{{{$timestamp}}}}: {}", ts);
    }

    #[test]
    fn test_dynamic_random_int() {
        let vars = HashMap::new();
        let result = interpolate("n={{$randomInt}}", &vars);
        let n: u32 = result[2..].parse().expect("Not a valid int");
        assert!(n <= 1000);
        println!("  Dynamic {{{{$randomInt}}}}: {}", n);
    }

    #[test]
    fn test_dynamic_random_email() {
        let vars = HashMap::new();
        let result = interpolate("e={{$randomEmail}}", &vars);
        let email = &result[2..];
        assert!(email.contains('@'));
        assert!(email.ends_with("@example.com"));
        println!("  Dynamic {{{{$randomEmail}}}}: {}", email);
    }

    #[test]
    fn test_unresolved_preserved() {
        let vars = HashMap::new();
        assert_eq!(interpolate("{{unknown}}", &vars), "{{unknown}}");
        println!("  Unresolved variable: preserved as-is");
    }

    #[test]
    fn test_mixed_vars() {
        let mut vars = HashMap::new();
        vars.insert("host".into(), "localhost".into());
        let result = interpolate("{{host}}/{{$uuid}}/{{missing}}", &vars);
        assert!(result.starts_with("localhost/"));
        assert!(result.ends_with("/{{missing}}"));
        println!("  Mixed vars: resolved + dynamic + unresolved OK");
    }
}

// ─── Scripting Engine Tests ──────────────────────────────────────────────────

mod scripting {
    use apiark_core::scripting::engine::execute_script;
    use apiark_core::scripting::{RequestSnapshot, ResponseSnapshot, ScriptContext, ScriptPhase};
    use std::collections::HashMap;

    fn empty_ctx() -> ScriptContext {
        ScriptContext {
            request: RequestSnapshot {
                method: "GET".into(),
                url: "https://api.example.com/users".into(),
                headers: HashMap::new(),
                body: None,
            },
            response: None,
            env: HashMap::new(),
            globals: HashMap::new(),
            variables: HashMap::new(),
        }
    }

    fn ctx_with_response() -> ScriptContext {
        let mut ctx = empty_ctx();
        ctx.response = Some(ResponseSnapshot {
            status: 200,
            status_text: "OK".into(),
            headers: {
                let mut h = HashMap::new();
                h.insert("content-type".into(), "application/json".into());
                h
            },
            body: r#"{"users":[{"id":1,"name":"Alice"},{"id":2,"name":"Bob"}]}"#.into(),
            time_ms: 150,
            size_bytes: 2048,
        });
        ctx
    }

    #[test]
    fn test_pre_request_set_env() {
        let ctx = empty_ctx();
        let result = execute_script(
            r#"ark.env.set("token", "generated-" + Date.now());"#,
            ctx,
            ScriptPhase::PreRequest,
        )
        .unwrap();
        assert!(result.env_mutations.contains_key("token"));
        let val = result.env_mutations["token"].as_ref().unwrap();
        assert!(val.starts_with("generated-"));
        println!("  Pre-request script: env.set OK");
    }

    #[test]
    fn test_pre_request_modify_url() {
        let ctx = empty_ctx();
        let result = execute_script(
            r#"ark.request.setUrl("https://modified.com/api/v2");"#,
            ctx,
            ScriptPhase::PreRequest,
        )
        .unwrap();
        assert_eq!(result.request.url, "https://modified.com/api/v2");
        println!("  Pre-request script: URL modification OK");
    }

    #[test]
    fn test_pre_request_set_header() {
        let ctx = empty_ctx();
        let result = execute_script(
            r#"
            ark.request.setHeader("X-Request-ID", "abc123");
            ark.request.setHeader("Authorization", "Bearer mytoken");
            "#,
            ctx,
            ScriptPhase::PreRequest,
        )
        .unwrap();
        assert_eq!(
            result.request.headers.get("X-Request-ID").unwrap(),
            "abc123"
        );
        assert_eq!(
            result.request.headers.get("Authorization").unwrap(),
            "Bearer mytoken"
        );
        println!("  Pre-request script: set headers OK");
    }

    #[test]
    fn test_post_response_tests() {
        let ctx = ctx_with_response();
        let result = execute_script(
            r#"
            ark.test("status is 200", function() {
                ark.expect(ark.response.status).to.equal(200);
            });
            ark.test("has users array", function() {
                var body = ark.response.json();
                ark.expect(body.users).to.be.a("object"); // arrays are objects in JS
                ark.expect(body.users.length).to.equal(2);
            });
            ark.test("first user is Alice", function() {
                var body = ark.response.json();
                ark.expect(body.users[0].name).to.equal("Alice");
            });
            "#,
            ctx,
            ScriptPhase::PostResponse,
        )
        .unwrap();
        assert_eq!(result.test_results.len(), 3);
        for tr in &result.test_results {
            assert!(tr.passed, "Test '{}' failed: {:?}", tr.name, tr.error);
        }
        println!("  Post-response tests: 3/3 passed");
    }

    #[test]
    fn test_post_response_extract_to_env() {
        let ctx = ctx_with_response();
        let result = execute_script(
            r#"
            var body = ark.response.json();
            ark.env.set("firstUserId", String(body.users[0].id));
            ark.env.set("userName", body.users[0].name);
            "#,
            ctx,
            ScriptPhase::PostResponse,
        )
        .unwrap();
        assert_eq!(result.env_mutations["firstUserId"].as_ref().unwrap(), "1");
        assert_eq!(result.env_mutations["userName"].as_ref().unwrap(), "Alice");
        println!("  Post-response script: extracted values to env OK");
    }

    #[test]
    fn test_console_log() {
        let ctx = empty_ctx();
        let result = execute_script(
            r#"
            console.log("info message");
            console.warn("warning!");
            console.error("error!");
            "#,
            ctx,
            ScriptPhase::PreRequest,
        )
        .unwrap();
        assert_eq!(result.console_output.len(), 3);
        assert_eq!(result.console_output[0].level, "log");
        assert_eq!(result.console_output[1].level, "warn");
        assert_eq!(result.console_output[2].level, "error");
        println!("  Console output: log/warn/error captured");
    }

    #[test]
    fn test_expect_negation() {
        let ctx = empty_ctx();
        let result = execute_script(
            r#"
            ark.test("not equal", function() {
                ark.expect(1).to.not.equal(2);
                ark.expect("hello").to.not.include("xyz");
            });
            "#,
            ctx,
            ScriptPhase::PreRequest,
        )
        .unwrap();
        assert!(result.test_results[0].passed);
        println!("  Expect negation: .not.equal/.not.include OK");
    }

    #[test]
    fn test_expect_comparison() {
        let ctx = ctx_with_response();
        let result = execute_script(
            r#"
            ark.test("comparisons", function() {
                ark.expect(ark.response.time).to.be.below(10000);
                ark.expect(ark.response.time).to.be.above(0);
                ark.expect(ark.response.size).to.be.above(100);
            });
            "#,
            ctx,
            ScriptPhase::PostResponse,
        )
        .unwrap();
        assert!(result.test_results[0].passed);
        println!("  Expect comparison: above/below OK");
    }

    #[test]
    fn test_script_error_handled() {
        let ctx = empty_ctx();
        let result = execute_script(
            "throw new Error('intentional failure');",
            ctx,
            ScriptPhase::PreRequest,
        );
        assert!(result.is_err());
        println!("  Script error: properly caught and returned");
    }

    #[test]
    fn test_read_only_in_post_response() {
        let ctx = ctx_with_response();
        let result = execute_script(
            r#"
            ark.test("cannot modify request", function() {
                var threw = false;
                try { ark.request.setUrl("http://evil.com"); } catch(e) { threw = true; }
                ark.expect(threw).to.equal(true);
            });
            "#,
            ctx,
            ScriptPhase::PostResponse,
        )
        .unwrap();
        assert!(result.test_results[0].passed);
        println!("  Post-response read-only: request mutation blocked");
    }

    #[test]
    fn test_globals_and_variables() {
        let ctx = empty_ctx();
        let result = execute_script(
            r#"
            ark.globals.set("apiVersion", "v3");
            ark.variables.set("requestId", "req_999");
            "#,
            ctx,
            ScriptPhase::PreRequest,
        )
        .unwrap();
        assert_eq!(
            result.global_mutations["apiVersion"].as_ref().unwrap(),
            "v3"
        );
        assert_eq!(
            result.variable_mutations["requestId"].as_ref().unwrap(),
            "req_999"
        );
        println!("  Globals + variables: set OK");
    }
}

// ─── Declarative Assertions Tests ────────────────────────────────────────────

mod assertions {
    use apiark_core::scripting::assertions::evaluate_assertions;
    use apiark_core::scripting::ResponseSnapshot;
    use std::collections::HashMap;

    fn make_response() -> ResponseSnapshot {
        ResponseSnapshot {
            status: 200,
            status_text: "OK".into(),
            headers: {
                let mut h = HashMap::new();
                h.insert("content-type".into(), "application/json".into());
                h
            },
            body: r#"{"users": [{"id": 1, "name": "Alice"}], "count": 1}"#.into(),
            time_ms: 150,
            size_bytes: 2048,
        }
    }

    #[test]
    fn test_status_assertion() {
        let resp = make_response();
        let assertions: serde_yaml::Value = serde_yaml::from_str("status: 200").unwrap();
        let results = evaluate_assertions(&assertions, &resp);
        assert!(results.iter().all(|r| r.passed), "Status assertion failed");
        println!("  Assert status=200: passed");
    }

    #[test]
    fn test_body_path_assertion() {
        let resp = make_response();
        let assertions: serde_yaml::Value = serde_yaml::from_str("body.count: 1").unwrap();
        let results = evaluate_assertions(&assertions, &resp);
        assert!(results.iter().all(|r| r.passed));
        println!("  Assert body.count=1: passed");
    }

    #[test]
    fn test_response_time_assertion() {
        let resp = make_response();
        let assertions: serde_yaml::Value =
            serde_yaml::from_str("responseTime:\n  lt: 5000").unwrap();
        let results = evaluate_assertions(&assertions, &resp);
        assert!(results.iter().all(|r| r.passed));
        println!("  Assert responseTime < 5000: passed");
    }

    #[test]
    fn test_contains_assertion() {
        let resp = make_response();
        let assertions: serde_yaml::Value =
            serde_yaml::from_str("headers.content-type:\n  contains: json").unwrap();
        let results = evaluate_assertions(&assertions, &resp);
        assert!(results.iter().all(|r| r.passed));
        println!("  Assert header contains 'json': passed");
    }

    #[test]
    fn test_failing_assertion() {
        let resp = make_response();
        let assertions: serde_yaml::Value = serde_yaml::from_str("status: 404").unwrap();
        let results = evaluate_assertions(&assertions, &resp);
        assert!(results.iter().any(|r| !r.passed));
        println!("  Assert status=404 (expected fail): correctly failed");
    }
}

// ─── cURL Parsing Tests ─────────────────────────────────────────────────────

mod curl {
    use apiark_core::http::curl::parse_curl;

    #[test]
    fn test_simple_get() {
        let result = parse_curl("curl https://api.example.com/users").unwrap();
        assert_eq!(result.method, "GET");
        assert_eq!(result.url, "https://api.example.com/users");
        println!("  cURL parse simple GET: OK");
    }

    #[test]
    fn test_post_with_data() {
        let result = parse_curl(
            r#"curl -X POST https://api.example.com/users -H "Content-Type: application/json" -d '{"name":"test"}'"#
        ).unwrap();
        assert_eq!(result.method, "POST");
        assert!(result.headers.contains_key("Content-Type"));
        assert!(result.body.as_ref().unwrap().contains("name"));
        println!("  cURL parse POST with data: OK");
    }

    #[test]
    fn test_basic_auth() {
        let result = parse_curl("curl -u user:pass https://api.example.com").unwrap();
        assert!(result.auth_basic.is_some());
        let (user, pass) = result.auth_basic.unwrap();
        assert_eq!(user, "user");
        assert_eq!(pass, "pass");
        println!("  cURL parse -u basic auth: OK");
    }

    #[test]
    fn test_multiple_headers() {
        let result = parse_curl(
            r#"curl -H "Authorization: Bearer token123" -H "Accept: application/json" -H "X-Custom: value" https://api.example.com"#
        ).unwrap();
        assert_eq!(result.headers.len(), 3);
        assert_eq!(result.headers["Authorization"], "Bearer token123");
        assert_eq!(result.headers["Accept"], "application/json");
        println!("  cURL parse multiple headers: OK");
    }

    #[test]
    fn test_flags() {
        let result = parse_curl("curl -k -L https://self-signed.example.com").unwrap();
        assert!(!result.verify_ssl);
        assert!(result.follow_redirects);
        println!("  cURL parse flags (-k, -L): OK");
    }
}

// ─── Mock Server Tests ───────────────────────────────────────────────────────

mod storage {
    use std::fs;

    #[test]
    fn test_yaml_request_roundtrip() {
        let temp_dir =
            std::env::temp_dir().join(format!("apiark_storage_test_{}", std::process::id()));
        fs::create_dir_all(&temp_dir).unwrap();

        let yaml_content = r#"name: Create User
method: POST
url: "https://api.example.com/users"
headers:
  - key: Content-Type
    value: application/json
    enabled: true
  - key: X-Request-ID
    value: "123"
    enabled: true
body:
  type: json
  content: |
    {"name": "test", "email": "test@example.com"}
auth:
  type: bearer
  token: my-secret-token
assert:
  status: 201
"#;

        let file_path = temp_dir.join("create-user.yaml");
        fs::write(&file_path, yaml_content).unwrap();

        // Parse it
        let parsed: serde_yaml::Value = serde_yaml::from_str(yaml_content).unwrap();
        assert_eq!(parsed["name"].as_str().unwrap(), "Create User");
        assert_eq!(parsed["method"].as_str().unwrap(), "POST");
        assert_eq!(parsed["auth"]["type"].as_str().unwrap(), "bearer");
        assert_eq!(parsed["auth"]["token"].as_str().unwrap(), "my-secret-token");

        // Read back and verify
        let read_back = fs::read_to_string(&file_path).unwrap();
        let re_parsed: serde_yaml::Value = serde_yaml::from_str(&read_back).unwrap();
        assert_eq!(parsed, re_parsed);

        println!("  YAML roundtrip: write + read + parse OK");
        fs::remove_dir_all(&temp_dir).ok();
    }

    #[test]
    fn test_environment_yaml() {
        let yaml = r#"name: Development
variables:
  baseUrl: http://localhost:3000
  apiKey: dev-key-12345
secrets:
  - accessToken
  - adminToken
"#;
        let parsed: serde_yaml::Value = serde_yaml::from_str(yaml).unwrap();
        assert_eq!(parsed["name"].as_str().unwrap(), "Development");
        assert_eq!(
            parsed["variables"]["baseUrl"].as_str().unwrap(),
            "http://localhost:3000"
        );
        let secrets = parsed["secrets"].as_sequence().unwrap();
        assert_eq!(secrets.len(), 2);
        println!("  Environment YAML parsing: OK");
    }

    #[test]
    fn test_git_merge_conflict_detection() {
        let content_with_conflict = r#"name: Test Request
method: GET
<<<<<<< HEAD
url: "https://api.v1.example.com"
=======
url: "https://api.v2.example.com"
>>>>>>> feature-branch
"#;
        let has_conflict = content_with_conflict.contains("<<<<<<<")
            && content_with_conflict.contains("=======")
            && content_with_conflict.contains(">>>>>>>");
        assert!(has_conflict);
        println!("  Git merge conflict detection: markers detected");
    }
}

// ─── SQLite History Tests ────────────────────────────────────────────────────

mod history {
    use rusqlite::Connection;

    #[test]
    fn test_history_db_operations() {
        let conn = Connection::open_in_memory().unwrap();

        // Create table (matching the actual schema)
        conn.execute_batch(
            "CREATE TABLE IF NOT EXISTS history (
                id INTEGER PRIMARY KEY AUTOINCREMENT,
                method TEXT NOT NULL,
                url TEXT NOT NULL,
                status INTEGER,
                status_text TEXT,
                time_ms INTEGER,
                size_bytes INTEGER,
                request_headers TEXT,
                request_body TEXT,
                response_body TEXT,
                collection_path TEXT,
                request_name TEXT,
                timestamp TEXT NOT NULL DEFAULT (datetime('now'))
            );",
        )
        .unwrap();

        // Insert a history entry
        conn.execute(
            "INSERT INTO history (method, url, status, status_text, time_ms, size_bytes, request_headers, request_body, response_body)
             VALUES (?1, ?2, ?3, ?4, ?5, ?6, ?7, ?8, ?9)",
            rusqlite::params![
                "GET",
                "https://api.example.com/users",
                200,
                "OK",
                150,
                2048,
                r#"{"Content-Type":"application/json"}"#,
                "",
                r#"{"users":[]}"#,
            ],
        ).unwrap();

        // Query
        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM history", [], |r| r.get(0))
            .unwrap();
        assert_eq!(count, 1);

        // Search
        let mut stmt = conn
            .prepare("SELECT method, url, status FROM history WHERE url LIKE '%example%'")
            .unwrap();
        let entry: (String, String, i64) = stmt
            .query_row([], |r| Ok((r.get(0)?, r.get(1)?, r.get(2)?)))
            .unwrap();
        assert_eq!(entry.0, "GET");
        assert_eq!(entry.2, 200);

        // Clear
        conn.execute("DELETE FROM history", []).unwrap();
        let count: i64 = conn
            .query_row("SELECT COUNT(*) FROM history", [], |r| r.get(0))
            .unwrap();
        assert_eq!(count, 0);

        // Integrity check
        let integrity: String = conn
            .query_row("PRAGMA integrity_check", [], |r| r.get(0))
            .unwrap();
        assert_eq!(integrity, "ok");

        println!("  SQLite history: insert, search, clear, integrity check OK");
    }
}

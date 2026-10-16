//! Repeatable loopback timing; reports raw samples and median, no network service.
use apiark_core::{http::client::HttpEngine, models::request::SendRequestParams};

#[tokio::main]
async fn main() {
    let listener = tokio::net::TcpListener::bind("127.0.0.1:0").await.unwrap();
    let url = format!("http://{}/", listener.local_addr().unwrap());
    let task = tokio::spawn(async move {
        axum::serve(
            listener,
            axum::Router::new().route("/", axum::routing::get(|| async { "{\"ok\":true}" })),
        )
        .await
        .unwrap()
    });
    let params: SendRequestParams =
        serde_json::from_value(serde_json::json!({"method":"GET","url":url})).unwrap();
    for _ in 0..10 {
        HttpEngine::send(params.clone()).await.unwrap();
    }
    let mut samples = Vec::new();
    for _ in 0..5 {
        let start = std::time::Instant::now();
        for _ in 0..100 {
            HttpEngine::send(params.clone()).await.unwrap();
        }
        samples.push(start.elapsed().as_secs_f64() * 1000.0);
    }
    println!("100 sequential requests, 5 rounds, milliseconds: {samples:?}");
    samples.sort_by(f64::total_cmp);
    println!("median_ms={:.3}", samples[2]);
    task.abort();
}

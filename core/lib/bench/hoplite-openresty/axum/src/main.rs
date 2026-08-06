use std::time::Duration;

use axum::{response::IntoResponse, routing::get, Json, Router};
use serde_json::json;

#[tokio::main(flavor = "current_thread")]
async fn main() {
    let app = Router::new()
        .route("/hello", get(|| async { "Hello from Axum\n" }))
        .route("/json", get(|| async { Json(json!({"message": "Hello from Axum"})) }))
        .route("/delay", get(|| async {
            tokio::time::sleep(Duration::from_millis(25)).await;
            "delayed 25ms\n".into_response()
        }));
    let listener = tokio::net::TcpListener::bind("127.0.0.1:18085").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

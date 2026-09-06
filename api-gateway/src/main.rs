mod routes;
use axum::{Json, Router, routing::get};

use crate::routes::account::account_router;

async fn health() -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "status": "running"
    }))
}

#[tokio::main]
async fn main() {
    let app = Router::<()>::new()
        .route("/health", get(health))
        .nest("/account", account_router());

    let listener = tokio::net::TcpListener::bind("127.0.0.1:8000").await;
    if !listener.is_ok() {
        return;
    }

    axum::serve(listener.unwrap(), app).await.unwrap();
}

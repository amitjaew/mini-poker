mod config;
mod routes;
mod schema;
mod state;

use axum::{Json, Router, routing::get};

use crate::{config::init_config, routes::account::account_router, state::init_state};

async fn health() -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "status": "running"
    }))
}

#[tokio::main]
async fn main() {
    println!("STARTING API");
    let config = init_config();
    let state = init_state(&config).await;

    let app = Router::new()
        .route("/health", get(health))
        .nest("/account", account_router())
        .with_state(state);

    let listener = tokio::net::TcpListener::bind(config.server_url).await;
    if !listener.is_ok() {
        return;
    }

    axum::serve(listener.unwrap(), app).await.unwrap();
}

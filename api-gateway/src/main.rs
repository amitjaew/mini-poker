mod auth;
mod config;
mod helpers;
mod routes;
mod schema;
mod state;

use axum::{Json, Router, middleware, routing::get};

use crate::{
    auth::authorization_middleware, config::init_config, routes::account::account_router,
    state::init_state,
};

async fn health() -> Json<serde_json::Value> {
    Json(serde_json::json!({
        "status": "running"
    }))
}

#[tokio::main]
async fn main() {
    println!("STARTING API");
    dotenvy::dotenv().expect("Failed to load environment variables");
    let config = init_config();
    let state = init_state(&config).await;

    let app = Router::new()
        .route("/health", get(health))
        .nest(
            "/account",
            account_router().layer(middleware::from_fn_with_state(
                state.clone(),
                authorization_middleware,
            )),
        )
        .with_state(state);

    let listener = tokio::net::TcpListener::bind(config.server_url).await;
    if !listener.is_ok() {
        return;
    }

    axum::serve(listener.unwrap(), app).await.unwrap();
}

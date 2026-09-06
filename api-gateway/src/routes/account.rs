use axum::{Json, Router, extract::Path, routing};
use serde_json::{Value, json};
use uuid::Uuid;

async fn get_account(Path(id): Path<Uuid>) -> Json<Value> {
    println!("id: {}", id.to_string());
    Json(json!({
        "id": &id.to_string()
    }))
}

pub fn account_router() -> Router<()> {
    Router::<()>::new().route("/{id}", routing::get(get_account))
}

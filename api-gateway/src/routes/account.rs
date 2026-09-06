use std::sync::Arc;

use axum::{
    Json, Router,
    extract::{Path, State},
    routing,
};
use serde_json::{Value, json};
use uuid::Uuid;

use crate::{schemas::UserDTO, state::AppState};

async fn get_account(Path(id): Path<Uuid>, State(state): State<Arc<AppState>>) -> Json<Value> {
    println!("id: {}", id.to_string());
    let result = sqlx::query_as!(UserDTO, r"SELECT id FROM users WHERE id=$1", id)
        .fetch_one(&state.db_pool)
        .await;

    match result {
        Ok(user) => Json(json!({
           "id": &user.id
        })),
        Err(err) => Json(json!({
            "status": &err.to_string()
        })),
    }
}

pub fn account_router() -> Router<Arc<AppState>> {
    Router::new().route("/{id}", routing::get(get_account))
}

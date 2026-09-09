use std::sync::Arc;

use axum::{
    Json, Router,
    extract::{Path, State},
    routing,
};
use serde_json::{Value, json};
use uuid::Uuid;

use crate::{
    schema::database::{
        BalanceMovement, BalanceMovementStatus, BalanceMovementType, Currency, GameType,
        UserBalanceDTO, UserDTO,
    },
    state::AppState,
};

async fn get_account(Path(id): Path<Uuid>, State(state): State<Arc<AppState>>) -> Json<Value> {
    println!("id: {}", id.to_string());
    let user_query = sqlx::query_as!(UserDTO, r"SELECT * FROM users WHERE id=$1", id)
        .fetch_one(&state.db_pool)
        .await;

    let balances_query = sqlx::query_as!(
        UserBalanceDTO,
        r#"SELECT id, user_id, currency as "currency: Currency", amount FROM users_balance"#
    )
    .fetch_all(&state.db_pool)
    .await;
    let movements_query = sqlx::query_as!(
        BalanceMovement,
        r#"SELECT id, balance_id, wallet_address, amount, movement_type as "movement_type: BalanceMovementType", game_type as "game_type: GameType", game_name, status as "status: BalanceMovementStatus", created_at FROM balance_movements"#
    ).fetch_all(&state.db_pool).await;

    match (user_query, balances_query, movements_query) {
        (Ok(user), Ok(balances), Ok(movements)) => {
            return Json(json!({
                "user": user,
                "movements": movements,
                "balances": balances
            }));
        }
        _ => {
            return Json(json!({
                "status": "not found"
            }));
        }
    }
}

pub fn account_router() -> Router<Arc<AppState>> {
    Router::new().route("/{id}", routing::get(get_account))
}

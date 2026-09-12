use std::sync::Arc;

use argon2::{Argon2, PasswordHasher};
use axum::{
    Json, Router,
    extract::{Path, State},
    middleware, routing,
};
use serde_json::{Value, json};
use uuid::Uuid;

use crate::{
    auth::authorization_middleware,
    schema::database::{
        BalanceMovement, BalanceMovementStatus, BalanceMovementType, Currency, GameType,
        UserBalanceDTO, UserDTO,
    },
    state::AppState,
};

/*
 * TODO:
 * - Complete personal account CRUD
 * - User profile view
 */

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

struct CreateAccountPayload {
    email: String,
    username: String,
    password: String,
}

async fn create_account(
    payload: CreateAccountPayload,
    State(state): State<Arc<AppState>>,
) -> Json<Value> {
    let user_search = sqlx::query!(
        r"SELECT EXISTS(SELECT 1 FROM users as u WHERE u.username=$1 OR u.email=$2)",
        payload.username,
        payload.email
    )
    .fetch_optional(&state.db_pool)
    .await;

    if user_search.is_err() {
        return Json(json!(
            {"errpr": "some error"}
        ));
    }

    if user_search.ok().is_some() {
        return Json(json!(
            {"error": "user already exists"}
        ));
    }

    let argon2 = Argon2::default();
    let password_hash = argon2
        .hash_password(payload.password.as_bytes())
        .unwrap()
        .to_string();

    let query = sqlx::query!(
        "INSERT INTO users (username, email, password_hash) VALUES ($1, $2, $3)",
        payload.username,
        payload.email,
        password_hash
    )
    .execute(&state.db_pool)
    .await;

    match query {
        Ok(res) => {
            return Json(json!(
                {"whatever": "whatever"}
            ));
        }
        Err(err) => {
            return Json(json!(
                {"whatever": "whatever"}
            ));
        }
    }
}

pub fn account_router(state: Arc<AppState>) -> Router<Arc<AppState>> {
    Router::new()
        .route("/{id}", routing::get(get_account))
        .route_layer(middleware::from_fn_with_state(
            state,
            authorization_middleware,
        ))
}

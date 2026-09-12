use std::sync::Arc;

use argon2::{Argon2, PasswordHasher};
use axum::{
    Extension, Json, Router,
    extract::{Path, State},
    http::StatusCode,
    middleware, routing,
};
use serde::Deserialize;
use serde_json::{Value, json};
use uuid::Uuid;

use crate::{
    auth::authorization_middleware,
    schema::{
        AppError,
        database::{ShallowUserDTO, UserDTO},
    },
    state::AppState,
};

/*
 * TODO:
 * - Complete personal account CRUD
 * - User profile view
 */

async fn get_account(
    Path(id): Path<Uuid>,
    State(state): State<Arc<AppState>>,
) -> Result<Json<Value>, AppError> {
    let user_query = sqlx::query_as!(UserDTO, r"SELECT * FROM users WHERE id=$1", id)
        .fetch_one(&state.db_pool)
        .await;

    // let balances_query = sqlx::query_as!(
    //     UserBalanceDTO,
    //     r#"SELECT id, user_id, currency as "currency: Currency", amount FROM users_balance"#
    // )
    // .fetch_all(&state.db_pool)
    // .await;
    // let movements_query = sqlx::query_as!(
    //     BalanceMovement,
    //     r#"SELECT id, balance_id, wallet_address, amount, movement_type as "movement_type: BalanceMovementType", game_type as "game_type: GameType", game_name, status as "status: BalanceMovementStatus", created_at FROM balance_movements"#
    // ).fetch_all(&state.db_pool).await;

    // match (user_query, balances_query, movements_query) {
    // (Ok(user), Ok(balances), Ok(movements)) => {
    match user_query {
        Ok(user) => {
            return Ok(Json(json!(ShallowUserDTO {
                id: user.id,
                username: user.username,
                at_room: user.at_room,
                at_server: user.at_server
            })));
        }
        _ => {
            return Err(AppError {
                message: "user not found".to_string(),
                status_code: StatusCode::NOT_FOUND,
            });
        }
    }
}

#[derive(Deserialize, Debug)]
struct CreateAccountPayload {
    pub email: String,
    pub username: String,
    pub password: String,
}

async fn create_account(
    State(state): State<Arc<AppState>>,
    Json(payload): Json<CreateAccountPayload>,
) -> Result<Json<Value>, AppError> {
    let user_search = sqlx::query!(
        r"SELECT EXISTS(SELECT 1 FROM users as u WHERE u.username=$1 OR u.email=$2)",
        payload.username,
        payload.email
    )
    .fetch_optional(&state.db_pool)
    .await;

    if user_search.is_err() {
        return Err(AppError {
            message: String::new(),
            status_code: StatusCode::INTERNAL_SERVER_ERROR,
        });
    }

    if user_search.ok().is_some() {
        return Err(AppError {
            message: "account already exists".to_string(),
            status_code: StatusCode::CONFLICT,
        });
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
        Ok(_) => {
            return Ok(Json(json!(
                {"status": "success"}
            )));
        }
        Err(_) => {
            return Err(AppError {
                message: String::new(),
                status_code: StatusCode::INTERNAL_SERVER_ERROR,
            });
        }
    }
}

async fn get_my_account(Extension(user): Extension<UserDTO>) -> Json<Value> {
    Json(json!(user))
}

pub fn account_router(state: Arc<AppState>) -> Router<Arc<AppState>> {
    Router::new()
        .route(
            "/{id}",
            routing::get(get_account).route_layer(middleware::from_fn_with_state(
                state,
                authorization_middleware,
            )),
        )
        .route("/", routing::post(create_account).get(get_my_account))
}

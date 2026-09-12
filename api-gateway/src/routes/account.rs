use std::sync::Arc;

use argon2::{Argon2, PasswordHash, PasswordHasher, PasswordVerifier};
use axum::{
    Extension, Json, Router,
    extract::{Path, State},
    http::StatusCode,
    middleware, routing,
};
use serde::Deserialize;
use serde_json::{Value, json};
use sqlx::{query, query_scalar};
use uuid::Uuid;

use crate::{
    auth::authorization_middleware,
    schema::{
        AppError,
        database::{ShallowUserDTO, UserDTO},
    },
    state::AppState,
};

async fn get_account(
    Path(id): Path<Uuid>,
    State(state): State<Arc<AppState>>,
) -> Result<Json<Value>, AppError> {
    let user_query = sqlx::query_as!(UserDTO, r"SELECT * FROM users WHERE id=$1", id)
        .fetch_one(&state.db_pool)
        .await;

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

async fn get_personal_account(Extension(user): Extension<UserDTO>) -> Json<Value> {
    Json(json!(user))
}

#[derive(Deserialize, Debug)]
struct UpdatePersonalAccountPasswordDTO {
    old_password: String,
    new_password: String,
}
async fn update_personal_account_password(
    State(state): State<Arc<AppState>>,
    Extension(user): Extension<UserDTO>,
    Json(payload): Json<UpdatePersonalAccountPasswordDTO>,
) -> Result<Json<Value>, AppError> {
    let argon2 = Argon2::default();

    let old_password_hash = match PasswordHash::new(&user.password_hash) {
        Ok(hash) => hash,
        Err(_) => {
            return Err(AppError {
                message: String::new(),
                status_code: StatusCode::INTERNAL_SERVER_ERROR,
            });
        }
    };

    match argon2.verify_password(&payload.old_password.into_bytes(), &old_password_hash) {
        Ok(()) => {}
        Err(password_hash::Error::PasswordInvalid) => {
            return Err(AppError {
                message: "invalid password".to_string(),
                status_code: StatusCode::UNAUTHORIZED,
            });
        }
        Err(_) => {
            return Err(AppError {
                message: String::new(),
                status_code: StatusCode::INTERNAL_SERVER_ERROR,
            });
        }
    }

    let new_password_hash = match argon2.hash_password(&payload.new_password.into_bytes()) {
        Ok(value) => value,
        Err(_) => {
            return Err(AppError {
                message: String::new(),
                status_code: StatusCode::INTERNAL_SERVER_ERROR,
            });
        }
    }
    .to_string();

    match query!(
        "UPDATE users SET password_hash=$1 WHERE id=$2",
        new_password_hash,
        user.id
    )
    .execute(&state.db_pool)
    .await
    {
        Ok(value) => {
            if value.rows_affected() == 0 {
                return Err(AppError {
                    message: "user not found".to_string(),
                    status_code: StatusCode::NOT_FOUND,
                });
            }
            return Ok(Json(json!({"status": "success"})));
        }
        Err(_) => {
            return Err(AppError {
                message: String::new(),
                status_code: StatusCode::INTERNAL_SERVER_ERROR,
            });
        }
    }
}

#[derive(Deserialize, Debug)]
struct UpdateUsernameDTO {
    username: String,
}
async fn update_personal_account_username(
    Extension(user): Extension<UserDTO>,
    State(state): State<Arc<AppState>>,
    Json(payload): Json<UpdateUsernameDTO>,
) -> Result<Json<Value>, AppError> {
    match query_scalar!(
        r"SELECT EXISTS(SELECT 1 FROM users WHERE username=$1)",
        payload.username,
    )
    .fetch_optional(&state.db_pool)
    .await
    {
        Ok(Some(Some(exists))) => {
            if exists {
                return Err(AppError {
                    message: "username already exists".to_string(),
                    status_code: StatusCode::CONFLICT,
                });
            }
        }
        _ => {
            return Err(AppError {
                message: String::new(),
                status_code: StatusCode::INTERNAL_SERVER_ERROR,
            });
        }
    }

    match query!(
        "UPDATE users SET username=$1 WHERE id=$2",
        payload.username,
        user.id
    )
    .execute(&state.db_pool)
    .await
    {
        Ok(value) => {
            if value.rows_affected() == 0 {
                return Err(AppError {
                    message: "user not found".to_string(),
                    status_code: StatusCode::NOT_FOUND,
                });
            }
            return Ok(Json(json!({"status": "success"})));
        }
        Err(_) => {
            return Err(AppError {
                message: String::new(),
                status_code: StatusCode::INTERNAL_SERVER_ERROR,
            });
        }
    }
}

async fn delete_personal_account(
    Extension(user): Extension<UserDTO>,
    State(state): State<Arc<AppState>>,
) -> Result<Json<Value>, AppError> {
    match query!("DELETE FROM users WHERE id=$1", user.id)
        .execute(&state.db_pool)
        .await
    {
        Ok(value) => {
            if value.rows_affected() == 0 {
                return Err(AppError {
                    message: "user not found".to_string(),
                    status_code: StatusCode::NOT_FOUND,
                });
            }
            return Ok(Json(json!({"status": "success"})));
        }
        Err(_) => {
            return Err(AppError {
                message: String::new(),
                status_code: StatusCode::INTERNAL_SERVER_ERROR,
            });
        }
    }
}

pub fn account_router(state: Arc<AppState>) -> Router<Arc<AppState>> {
    let auth_routes = Router::new()
        .route(
            "/",
            routing::get(get_personal_account).delete(delete_personal_account),
        )
        .route(
            "/username",
            routing::patch(update_personal_account_username),
        )
        .route(
            "/password",
            routing::patch(update_personal_account_password),
        )
        .route_layer(middleware::from_fn_with_state(
            state,
            authorization_middleware,
        ));

    Router::new()
        .route("/shallow/{id}", routing::get(get_account))
        .route("/", routing::post(create_account))
        .merge(auth_routes)
}

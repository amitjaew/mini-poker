use std::sync::Arc;

use axum::{
    body::Body,
    extract::{Request, State},
    http::{Response, StatusCode, header},
    middleware::Next,
};
use serde::{Deserialize, Serialize};
use sqlx::query_as;
use uuid::Uuid;

use crate::{
    helpers::jwt::decode_jwt,
    schema::{AppError, database::UserDTO},
    state::AppState,
};

#[derive(Serialize, Deserialize)]
pub struct SessionClaim {
    pub user_id: Uuid,
    pub expires_at: usize,
    pub created_at: usize,
}

pub async fn authorization_middleware(
    State(state): State<Arc<AppState>>,
    mut req: Request,
    next: Next,
) -> Result<Response<Body>, AppError> {
    let auth_header = req.headers_mut().get(header::AUTHORIZATION);
    let auth_header = match auth_header {
        Some(value) => value.to_str().map_err(|_| AppError {
            message: "empty header is not allowed".to_string(),
            status_code: StatusCode::FORBIDDEN,
        })?,
        None => {
            return Err(AppError {
                message: "missing token".to_string(),
                status_code: StatusCode::FORBIDDEN,
            });
        }
    };

    let mut header_it = auth_header.split_whitespace();
    let (_, token) = (header_it.next(), header_it.next());

    let payload = match decode_jwt(token.unwrap().to_string()) {
        Ok(data) => data,
        Err(_) => {
            return Err(AppError {
                message: "invalid jwt".to_string(),
                status_code: StatusCode::UNAUTHORIZED,
            });
        }
    };

    if payload.claims.expires_at < time::UtcDateTime::now().unix_timestamp() as usize {
        return Err(AppError {
            message: "token expired".to_string(),
            status_code: StatusCode::UNAUTHORIZED,
        });
    }
    let user_query = query_as!(
        UserDTO,
        r"SELECT * FROM users WHERE id=$1",
        payload.claims.user_id
    )
    .fetch_optional(&state.db_pool)
    .await;

    match user_query {
        Ok(Some(user)) => {
            req.extensions_mut().insert(user);
        }
        _ => {
            return Err(AppError {
                message: "invalid user".to_string(),
                status_code: StatusCode::UNAUTHORIZED,
            });
        }
    }

    Ok(next.run(req).await)
}

use axum::http::StatusCode;
use jsonwebtoken::{DecodingKey, EncodingKey, Header, TokenData, Validation};
use uuid::Uuid;

use crate::auth::SessionClaim;

pub fn encode_jwt(user_id: Uuid) -> Result<String, StatusCode> {
    let secret = env!("JWT_SECRET").as_bytes();
    let now = time::UtcDateTime::now();
    let expire = time::Duration::hours(24);
    let claim = SessionClaim {
        user_id,
        created_at: now.unix_timestamp() as usize,
        expires_at: (now + expire).unix_timestamp() as usize,
    };

    return jsonwebtoken::encode(
        &Header::default(),
        &claim,
        &EncodingKey::from_secret(secret),
    )
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR);
}

pub fn decode_jwt(token: String) -> Result<TokenData<SessionClaim>, StatusCode> {
    let secret = env!("JWT_SECRET").as_bytes();

    return jsonwebtoken::decode(
        &token,
        &DecodingKey::from_secret(secret),
        &Validation::default(),
    )
    .map_err(|_| StatusCode::INTERNAL_SERVER_ERROR);
}

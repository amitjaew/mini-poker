use axum::{
    http::StatusCode,
    response::{IntoResponse, Response},
};

pub mod database;

pub struct AppError {
    pub message: String,
    pub status_code: StatusCode,
}

impl IntoResponse for AppError {
    fn into_response(self) -> Response {
        (self.status_code, self.message).into_response()
    }
}

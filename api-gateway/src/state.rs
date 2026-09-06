use std::sync::Arc;

use sqlx::{PgPool, postgres::PgPoolOptions};

pub struct AppState {
    pub db_pool: PgPool,
}

pub async fn init_state() -> Arc<AppState> {
    let db_pool = PgPoolOptions::new()
        .max_connections(5)
        .connect("postgres://postgres:changeme@0.0.0.0:5432/main_db")
        .await
        .expect("Failed to connect to database");

    Arc::new(AppState { db_pool })
}

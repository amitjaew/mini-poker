use std::sync::Arc;

use sqlx::{PgPool, postgres::PgPoolOptions};

use crate::config::AppCofig;

pub struct AppState {
    pub db_pool: PgPool,
}

pub async fn init_state(config: &AppCofig) -> Arc<AppState> {
    let db_pool = PgPoolOptions::new()
        .max_connections(5)
        .connect(&config.database_url)
        .await
        .expect("Failed to connect to database");

    Arc::new(AppState { db_pool })
}

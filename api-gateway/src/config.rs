use std::env;

const DEFAULT_DATABASE_URL: &str = "postgres://postgres:changeme@0.0.0.0:5432/main_db";
const DEFAULT_SERVER_URL: &str = "127.0.0.1:8000";

pub struct AppCofig {
    pub database_url: String,
    pub server_url: String,
}

pub fn init_config() -> AppCofig {
    let database_url = env::var("DATABASE_URL").unwrap_or(DEFAULT_DATABASE_URL.to_string());
    let server_url = env::var("SERVER_URL").unwrap_or(DEFAULT_SERVER_URL.to_string());

    AppCofig {
        database_url,
        server_url,
    }
}

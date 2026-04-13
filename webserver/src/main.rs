mod core;
mod server;
mod demo;

#[tokio::main]
async fn main() {
    server::http::start().await;
}

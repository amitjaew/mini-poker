mod core;
mod demo;

use demo::gameserver::gameserver_demo;

#[tokio::main]
async fn main() 
{
    gameserver_demo().await;
}

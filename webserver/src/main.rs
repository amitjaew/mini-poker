mod core;
mod demo;

use demo::{hand_evaluation::hand_evaluation_demo, gameserver::gameserver_demo};

use crate::demo::hand_evaluation::omaha_evaluation_demo;

#[tokio::main]
async fn main()
{
    // gameserver_demo().await;
    hand_evaluation_demo();
    omaha_evaluation_demo();
}

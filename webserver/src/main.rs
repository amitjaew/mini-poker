mod core;
mod demo;

use demo::{gameserver::gameserver_demo};

use crate::demo::hand_evaluation::{hand_evaluation_demo, omaha_evaluation_demo, compare_hands_holdem_demo, compare_hands_omaha_demo};

#[tokio::main]
async fn main()
{
    // gameserver_demo().await;
    hand_evaluation_demo();
    omaha_evaluation_demo();
    compare_hands_holdem_demo();
    compare_hands_omaha_demo();
}

use std::env;

use crate::core::game::GameType;

mod core;
mod demo;
mod server;

const ARGS_MESSAGE: &str = "\
Required params:

\tserver <mode>
\t\ttexas-holdem
\t\tomaha

\tdemo <mode>
\t\thand_eval
\t\thand_eval_omaha
\t\tcompare_holdem
\t\tcompare_omaha
";

#[tokio::main]
async fn main() {
    let args: Vec<String> = env::args().collect();
    assert!(args.len() == 3, "{ARGS_MESSAGE}");

    if args[1] == "server" {
        handle_server(args[2].clone()).await;
    } else if args[1] == "demo" {
        handle_demo(args[2].clone());
    } else {
        println!("{ARGS_MESSAGE}");
    }
}

async fn handle_server(mode: String) {
    if mode == "texas-holdem" {
        server::http::start(vec![GameType::TexasHoldemPoker]).await;
    } else if mode == "omaha" {
        server::http::start(vec![GameType::OmahaPoker]).await;
    } else {
        eprintln!("Invalid mode: {mode}");
        eprintln!("Valid modes:\n\ttexas-holdem\n\tomaha")
    }
}

fn handle_demo(mode: String) {
    if mode == "hand_eval" {
        demo::hand_evaluation::hand_evaluation_demo();
    } else if mode == "hand_eval_omaha" {
        demo::hand_evaluation::omaha_evaluation_demo();
    } else if mode == "compare_holdem" {
        demo::hand_evaluation::compare_hands_holdem_demo();
    } else if mode == "compare_omaha" {
        demo::hand_evaluation::compare_hands_omaha_demo();
    } else {
        eprintln!("Invalid mode: {mode}");
        eprintln!(
            "Valid modes:\n\thand_eval\n\thand_eval_omaha\n\tcompare_holdem\n\tcompare_omaha"
        );
    }
}

use tokio::sync::{ mpsc, Mutex };
use tokio::time::{timeout, Duration};
use tokio;
use axum::extract::ws::{ CloseFrame, Message, Utf8Bytes, WebSocket };
use futures_util::{
   sink::SinkExt,
   stream::{ StreamExt, SplitSink, SplitStream }
};
use uuid::Uuid;
use std::sync::Arc;
use serde::{Serialize, Deserialize};

use crate::server::game::gameroom::{GameRoomMessage, PlayerPayload};

pub struct PlayerSession {
    pub id: uuid::Uuid,
}

#[derive(Serialize, Deserialize, Clone)]
pub struct WinnerPayload {
    winner_id: String,
    prize: u32
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum PlayerWarningType {
    Debug,
    InvalidAction
}
#[derive(Serialize, Deserialize, Clone)]
pub struct PingData {
    pub timer: f32
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum PlayerMessage {
    Debug { content: String },
    Turn { player_id: Uuid },
    Timer { time: f32 },
    Result {
        winners: Vec<WinnerPayload>
    },
    Warning {
        warning_type: PlayerWarningType,
        message: String
    },
    Ping {
        data: PingData,
        server_ts: u64,
    },
    PongAck {
        server_ts: u64,
        client_ts: u64,
        server_ack_ts: u64
    },
    TerminateSession
}

impl PlayerSession {
    pub fn new(
        id: Uuid,
        sender: mpsc::Sender<GameRoomMessage>,
        receiver: mpsc::Receiver<PlayerMessage>,
        socket: WebSocket,
    ) -> Self {
        let (socket_sender, socket_receiver) = socket.split();
        let active = Arc::new(Mutex::new(true));
        tokio::spawn(
            player_message_recv_loop(
                id.clone(),
                receiver,
                socket_sender,
                active.clone()
            )
        );
        tokio::spawn(
            player_socket_recv_loop(
                id.clone(),
                socket_receiver,
                sender,
                active.clone()
            )
        );

        Self { id: id.clone() }
    }
}

async fn player_message_recv_loop(
    player_id: uuid::Uuid,
    mut receiver: mpsc::Receiver<PlayerMessage>,
    mut socket_sender: SplitSink<WebSocket, Message>,
    active: Arc<Mutex<bool>>
) {
    loop {
        if !*active.lock().await { break; }
        const REFRESH_TIMEOUT: u64 = 1;
        let result = timeout(Duration::from_secs(REFRESH_TIMEOUT), receiver.recv()).await;

        match result {
            Ok(Some(message)) => {
                match message {
                    PlayerMessage::TerminateSession => {
                        println!("Player {}: Sending Terminate Session Message", player_id);

                        let close_payload: CloseFrame = CloseFrame { code: 1000, reason: Utf8Bytes::from("closing") };
                        let _ = socket_sender.send(Message::Close(Some(close_payload))).await;
                    },
                    _ => {
                        let content = serde_json::to_string(&message).unwrap_or(String::new());
                        println!("Player {}: Sending \n{}\n------------------------", player_id, content);
                        let _ = socket_sender.send(Message::text(content)).await;
                    }
                }
            },
            Ok(None) => {
                eprintln!("Player {} connection closed", player_id);
                *active.lock().await = false;
            },
            Err(_) => {
                println!("Player {}: No message received within {} seconds", player_id, REFRESH_TIMEOUT);
                continue;
            }
        }
    }

    println!("Closing recv loop for player: {}", player_id);
}

async fn player_socket_recv_loop(
    player_id: uuid::Uuid,
    mut socket_receiver: SplitStream<WebSocket>,
    sender: mpsc::Sender<GameRoomMessage>,
    active: Arc<Mutex<bool>>
) {
    loop {
        if !*active.lock().await { break; }

        const REFRESH_TIMEOUT: u64 = 1;
        let result = timeout(Duration::from_secs(REFRESH_TIMEOUT), socket_receiver.next()).await;

        match result {
            Ok(Some(Ok(message_unparsed))) => {
                match message_unparsed.to_text() {
                    Ok(message) => {
                        println!("Player {} sent message: {}", player_id, message);
                        match serde_json::from_str::<PlayerPayload>(message) {
                            Ok(payload) => {
                                let _ = sender.send(
                                    GameRoomMessage::PlayerPayload {
                                        payload,
                                        from: player_id
                                    }
                                ).await;
                            },
                            Err(err) => {
                                eprintln!("Player {} sent invalid action: {}, {}", player_id, message, err);
                            }
                        }
                    },
                    Err(err) => {
                        eprintln!("Invalid text message: {}", err);
                    }
                }
            },
            Ok(Some(Err(err))) => {
                eprintln!("Player inbound socket error: {}", err);
            },
            Ok(None) => {},
            Err(_err) => {}
        }
    }

    println!("Closing socket inbound loop for player {}", player_id);
}

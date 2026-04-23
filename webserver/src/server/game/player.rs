use std::time::{Duration, SystemTime, UNIX_EPOCH};

use tokio::sync::{ mpsc };
use tokio;
use axum::extract::ws::{ CloseFrame, Message, Utf8Bytes, WebSocket };
use futures_util::{
   sink::SinkExt,
   stream::{ StreamExt, SplitSink, SplitStream }
};
use uuid::Uuid;
use serde::{Serialize, Deserialize};

use crate::server::game::gameroom::{GameRoomMessage, PlayerGameAction, PlayerPayload};

pub struct PlayerSession {
    pub id: uuid::Uuid,
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum PlayerWarningType {
    Debug,
    InvalidAction
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum PlayerMessage {
    Debug { content: String },
    Turn { player_id: Uuid, timeout: u64 },
    Timer { time: f32 },
    PlayerAction {
        player_id: Uuid,
        action: PlayerGameAction,
        bet_base: u32
    },
    StateUpdate {
        active_players: Option<Vec<Uuid>>,
        current_player_turn: Option<Uuid>,
        bet_base: Option<u32>
    },
    Result {
        winners: Vec<Uuid>,
        prizes: Vec<u32>
    },
    Warning {
        warning_type: PlayerWarningType,
        message: String
    },
    Ping {
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
        gameroom_sender: mpsc::Sender<GameRoomMessage>,
        player_sender: mpsc::Sender<PlayerMessage>,
        player_receiver: mpsc::Receiver<PlayerMessage>,
        socket: WebSocket,
    ) -> Self {
        let (socket_sender, socket_receiver) = socket.split();
        let (shutdown_tx, shutdown_rx) = tokio::sync::watch::channel(false);
        // let active = Arc::new(Mutex::new(true));

        tokio::spawn(
            player_message_recv_loop(
                id.clone(),
                player_receiver,
                socket_sender,
                shutdown_tx,
                shutdown_rx.clone()
            )
        );
        tokio::spawn(
            player_socket_recv_loop(
                id.clone(),
                socket_receiver,
                gameroom_sender,
                shutdown_rx.clone()
            )
        );
        tokio::spawn(
            player_ping_loop(
                player_sender,
                shutdown_rx
            )
        );

        Self { id: id.clone() }
    }
}

async fn player_message_recv_loop(
    player_id: uuid::Uuid,
    mut receiver: mpsc::Receiver<PlayerMessage>,
    mut socket_sender: SplitSink<WebSocket, Message>,
    shutdown_tx: tokio::sync::watch::Sender<bool>,
    mut shutdown_rx: tokio::sync::watch::Receiver<bool>
) {
    loop {
        tokio::select! {
            received = receiver.recv() => {
                match received {
                    Some(PlayerMessage::TerminateSession) => {
                        println!("Player {}: Sending Terminate Session Message", player_id);
                        let close_payload: CloseFrame = CloseFrame { code: 1000, reason: Utf8Bytes::from("closing") };
                        let _ = socket_sender.send(Message::Close(Some(close_payload))).await;
                    },
                    Some(message) => {
                        let content = serde_json::to_string(&message).unwrap_or(String::new());
                        println!("Player {}: Sending \n{}\n------------------------", player_id, content);
                        let _ = socket_sender.send(Message::text(content)).await;
                    },
                    None => {
                        eprintln!("Player {} connection closed", player_id);
                        _ = shutdown_tx.send(true);
                    }
                }
            }

            _ = shutdown_rx.changed() => {
                break;
            }
        }
    }
    println!("Closing recv loop for player: {}", player_id);
}

async fn handle_player_inbound_message(
    unparsed_message: Message,
    sender: &mpsc::Sender<GameRoomMessage>,
    player_id: Uuid
){
    match unparsed_message.to_text() {
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
}

async fn player_socket_recv_loop(
    player_id: uuid::Uuid,
    mut socket_receiver: SplitStream<WebSocket>,
    sender: mpsc::Sender<GameRoomMessage>,
    mut shutdown_rx: tokio::sync::watch::Receiver<bool>
) {

    loop {
        tokio::select! {
            received = socket_receiver.next() => {
                match received {
                    Some(Ok(unparsed_message)) => {
                        handle_player_inbound_message(unparsed_message, &sender, player_id).await;
                    },
                    Some(Err(err)) => {
                        eprintln!("Player inbound socket error: {}", err);
                    }
                    None => {}
                }
            }

            _ = shutdown_rx.changed() => {
                break;
            }
        }
    }
    println!("Closing socket inbound loop for player {}", player_id);
}

async fn player_ping_loop(
    sender: mpsc::Sender<PlayerMessage>,
    mut shutdown_rx: tokio::sync::watch::Receiver<bool>
) {
    loop {
        tokio::select! {
            _ = tokio::time::sleep(Duration::from_secs(5)) => {
                let payload = PlayerMessage::Ping {
                    server_ts: SystemTime::now().duration_since(UNIX_EPOCH).unwrap().as_millis() as u64
                };
                _ = sender.send(payload).await;
            }

            _ = shutdown_rx.changed() => {
                break;
            }
        }
    }
}

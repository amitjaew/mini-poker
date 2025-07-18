use tokio::sync::{ mpsc, Mutex };
use tokio::time::{timeout, Duration};
use tokio;
use uuid;
use axum::extract::ws::{ WebSocket, Message };
use futures_util::{
   sink::SinkExt,
   stream::{ StreamExt, SplitSink, SplitStream }
};
use std::sync::Arc;

use crate::core::gameroom::GameRoomMessage;


pub struct Player {
    pub id: uuid::Uuid,
    active: Arc<Mutex<bool>>
}
pub enum PlayerMessage {
    GameRoomPayload { content: u32 },
    TerminateSession
}

impl Player {
    pub fn new(
        sender: mpsc::Sender<GameRoomMessage>,
        receiver: mpsc::Receiver<PlayerMessage>,
        socket: WebSocket
    ) -> Self {
        let (socket_sender, socket_receiver) = socket.split();
        let id = uuid::Uuid::new_v4();
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

        Self {
            id: uuid::Uuid::new_v4(),
            active
        }
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
                    PlayerMessage::GameRoomPayload { content } => {
                        println!("Player {} receives {}", player_id, content);
                        let message = Message::text(format!("message: {}", content));
                        let _ = socket_sender.send(message).await;
                    },
                    PlayerMessage::TerminateSession => {
                        println!("Player {} eceived Terminate Session Message", player_id);
                        *active.lock().await = false
                    }
                }
            },
            Ok(None) => {
                eprintln!("Player {} connection closed", player_id);
                *active.lock().await = false;
            },
            Err(_) => {
                println!("Player {}: No message received within {} seconds", player_id, REFRESH_TIMEOUT);
                continue; // continue the loop
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
                        let payload = GameRoomMessage::PlayerPayload { 
                            content: message.to_string(),
                            from: player_id
                        };
                        let _ = sender.send(payload).await;
                    },
                    Err(err) => {
                        eprintln!("Invalid text message: {}", err);
                    }
                }
            },
            Ok(Some(Err(err))) => {
                eprintln!("Player inbound socket error: {}", err);
            },
            Ok(None) => {
                
            },
            Err(err) => {
            
            }
        }
    }

    println!("Closing socket inbound loop for player {}", player_id);
}

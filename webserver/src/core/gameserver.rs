use crate::core::gameroom::GameRoomHandle;
use tokio::sync::{mpsc, oneshot};
use tokio;
use uuid;
use axum::extract::ws::WebSocket;


struct GameServer {
    gameroom_handlers: Vec<GameRoomHandle>,
    receiver: mpsc::Receiver<GameServerMessage>
}
impl GameServer {
    fn new(receiver: mpsc::Receiver<GameServerMessage>) -> Self {
        Self {
            gameroom_handlers: Vec::new(),
            receiver
        }
    }

    async fn handle_start_gameroom(&mut self) {
        self.gameroom_handlers.push(GameRoomHandle::new().await);
    }

    async fn handle_join_player(&mut self, websocket: WebSocket, room_id: uuid::Uuid) {
        let gameroom_handler_attempt = self.gameroom_handlers.iter().find(|&v| v.id == room_id);
        match gameroom_handler_attempt {
            Some(gameroom_handler) => gameroom_handler.handle_player_connection(websocket).await,
            None => {}
        }
    }

    fn handle_list_gamerooms(&self, respond_to: oneshot::Sender<Vec<GameRoomDTO>>) {
        let gameroom_dtos  = self.gameroom_handlers.iter().map(
            |game_room_handle| GameRoomDTO { id: game_room_handle.id.clone() }
        ).collect();
        let _ = respond_to.send(gameroom_dtos);
    }
}


pub struct GameRoomDTO {
    pub id: uuid::Uuid
}
pub enum GameServerMessage {
    GameRoomStart,
    PlayerJoin { websocket: WebSocket, room_id: uuid::Uuid },
    ListGameRooms { respond_to: oneshot::Sender<Vec<GameRoomDTO>> }
}

#[derive(Clone)]
pub struct GameServerHandle {
    pub sender: mpsc::Sender<GameServerMessage>
}

impl GameServerHandle {
    pub fn new() -> Self {
        let (sender, receiver) = mpsc::channel(100);
        let gameserver = GameServer::new(receiver);
        tokio::spawn(gameserver_message_recv_loop(gameserver));
        Self { sender }
    }

    pub async fn list_gamerooms(&self) -> Vec<GameRoomDTO> {
        let (oneshot_sender, oneshot_receiver) = oneshot::channel();
        let _ = self.sender.send(GameServerMessage::ListGameRooms { respond_to: oneshot_sender }).await;
        oneshot_receiver.await.expect("Gameserver Channel Closed")
    }

    pub async fn player_join(&self, websocket: WebSocket, room_id: uuid::Uuid) {
        let _ = self.sender.send(GameServerMessage::PlayerJoin { websocket, room_id }).await;
    }

    pub async fn gameroom_start(&self) {
        let _ = self.sender.send(GameServerMessage::GameRoomStart).await;
    }
}

async fn gameserver_message_recv_loop(mut gameserver: GameServer) {
    while let Some(message) = gameserver.receiver.recv().await {
        match message {
            GameServerMessage::GameRoomStart => gameserver.handle_start_gameroom().await,
            GameServerMessage::PlayerJoin { websocket, room_id } => gameserver.handle_join_player(websocket, room_id).await,
            GameServerMessage::ListGameRooms { respond_to } => gameserver.handle_list_gamerooms(respond_to)
        }
    }
}

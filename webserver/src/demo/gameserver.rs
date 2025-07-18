use axum::{
    extract::ws::{WebSocketUpgrade, WebSocket},
    routing,
    response::Response,
    Router,
};

use crate::core::gameserver::GameServerHandle;


pub async fn gameserver_demo() {
    let gameserver_handle = GameServerHandle::new();
    gameserver_handle.gameroom_start().await;

    let rooms = gameserver_handle.list_gamerooms().await;
    for room in rooms { println!("room: {}", room.id); };

    async fn player_conn_handler(socket: WebSocketUpgrade, gameserver_handle: GameServerHandle) -> Response {
        socket.on_upgrade(move |socket| handle_socket(socket, gameserver_handle))
    }
    async fn handle_socket(websocket: WebSocket, gameserver_handle: GameServerHandle) {
        let rooms = gameserver_handle.list_gamerooms().await;
        if rooms.len() > 0 {
            gameserver_handle.player_join(websocket, rooms[0].id).await;
        }
    }

    let app = Router::new().route(
        "/ws",
        routing::any(
        move |ws| player_conn_handler(ws, gameserver_handle.clone())
    ));
    let listener = tokio::net::TcpListener::bind("0.0.0.0:3000").await.unwrap();
    axum::serve(listener, app).await.unwrap();
}

use tokio::sync::{ mpsc, Mutex };
use tokio;
use uuid;
use axum::extract::ws::WebSocket;
use std::sync::Arc;
use crate::core::player::{ PlayerMessage, Player };
use crate::core::card::{ Card, Rank, Suit, DECK };
use rand::Rng;
use rand::rngs::ThreadRng;

#[derive(Clone)]
struct GameRoomPlayer {
    id: uuid::Uuid,
    sender: mpsc::Sender<PlayerMessage>,
    state: GameRoomPlayerState
}
struct GameRoom {
    //id: uuid::Uuid, // (unneeded for now)
    players: Vec<GameRoomPlayer>,
    state: GameRoomState,
}
struct GameRoomState {
    step: u32,
    deck: [Card; 52]
}
#[derive(Clone)]
struct GameRoomPlayerState {
    is_active: bool
}
pub enum GameRoomMessage {
    PlayerPayload { content: String, from: uuid::Uuid },
    PlayerJoin { id: uuid::Uuid, sender: mpsc::Sender<PlayerMessage> }
}

impl GameRoom {
    fn new() -> Self {
        let players = Vec::new();
        let state = GameRoomState {
            step: 0,
            deck: DECK,
        };

        Self {
            // id: uuid::Uuid::new_v4(), //(unneeded for now)
            players,
            state,
            //rng: rand::rng()
        }
    }
    async fn handle_gameroom_message(&mut self, message: GameRoomMessage) {
        match message {
            GameRoomMessage::PlayerJoin { id, sender } => {
                self.players.push(
                    GameRoomPlayer{
                        id,
                        sender,
                        state: GameRoomPlayerState { 
                            is_active: false
                        }
                    }
                );
            },
            GameRoomMessage::PlayerPayload { from, content } => {
                //println!("Gameroom {} received {} from Player {}", self.id, content, from);
                println!("Gameroom received {} from Player {}", content, from);
                self.state.step = 0; // JUST FOR DEBUGGING
            }
        }
    }
}

enum PokerStep {
    Blind,
    PreFlop,
    Flop,
    Turn,
    River,
    Showdown,
    BettingRound
}

fn handle_poker_step(
    step: PokerStep,
    gameroom_state:GameRoomState,
    players:Vec<GameRoomPlayer>
) {
    match step {
        PokerStep::Blind => {
            
        },
        PokerStep::PreFlop => {
            
        },
        PokerStep::Flop => {
            
        },
        PokerStep::Turn => {
            
        },
        PokerStep::River => {
            
        },
        PokerStep::Showdown => {
            
        },
        PokerStep::BettingRound => {
            
        }
    }
}

const STANDARD_POKER_STEPS: [PokerStep; 10] = [
    PokerStep::Blind,
    PokerStep::PreFlop,
    PokerStep::BettingRound,
    PokerStep::Flop,
    PokerStep::BettingRound,
    PokerStep::Turn,
    PokerStep::BettingRound,
    PokerStep::River,
    PokerStep::BettingRound,
    PokerStep::Showdown
];

async fn gameroom_message_loop(
    gameroom: Arc<Mutex<GameRoom>>,
    mut receiver: mpsc::Receiver<GameRoomMessage>
) {
    while let Some(message) = receiver.recv().await {
        gameroom.lock().await.handle_gameroom_message(message).await;
    }
}
async fn gameroom_state_loop(gameroom: Arc<Mutex<GameRoom>>) {
    loop {
        {
            let mut gameroom_guard = gameroom.lock().await;
            gameroom_guard.state.step += 1; // JUST FOR DEBUGGING
            let mut removed_players = Vec::new();

            for player in gameroom_guard.players.iter() {
                match player.sender.clone().send(PlayerMessage::GameRoomPayload { content: 123 }).await {
                    Ok(_) => println!("Message sent to player {}", player.id),
                    Err(err) => eprintln!("FAILED to send to {}: {}", player.id, err),
                }
                let _ = player.sender.send(PlayerMessage::TerminateSession).await;
                removed_players.push(player.clone());
            }
            //println!("state: {}", gameroom_guard.state.step);

            for player in removed_players {
                gameroom_guard.players.retain(|val| val.id != player.id);
            }
        }

        tokio::time::sleep(tokio::time::Duration::new(1, 0)).await;
    }
}

pub struct GameRoomHandle {
    pub id: uuid::Uuid,
    sender: mpsc::Sender<GameRoomMessage>
}

impl GameRoomHandle {
    pub async fn new() -> Self {
        let (sender, receiver) = mpsc::channel(100);
        let gameroom_mutex = Arc::new(Mutex::new(
            GameRoom::new()
        ));

        tokio::spawn(gameroom_message_loop(gameroom_mutex.clone(), receiver));
        tokio::spawn(gameroom_state_loop(gameroom_mutex));


        Self { 
            id: uuid::Uuid::new_v4(),
            sender
        }
    }

    pub async fn handle_player_connection (&self, websocket: WebSocket) {
        let (player_sender, player_receiver) = mpsc::channel(10);
        let gameroom_sender = self.sender.clone();
        let player = Player::new(
            gameroom_sender,
            player_receiver,
            websocket
        );
        let _ = self.sender.send(
            GameRoomMessage::PlayerJoin{
                id: player.id.clone(),
                sender: player_sender
            }
        ).await;
    }
}

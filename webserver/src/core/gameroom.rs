use tokio::sync::{ mpsc, Mutex };
use tokio;
use uuid;
use axum::extract::ws::WebSocket;
use std::sync::Arc;
use crate::core::player::{ PlayerMessage, Player };
use crate::core::card::{ Card, Rank, Suit, DECK };
use rand;
use rand::seq::SliceRandom;

#[derive(Clone)]
struct GameRoomPlayer {
    id: uuid::Uuid,
    sender: mpsc::Sender<PlayerMessage>,
    state: GameRoomPlayerState,
}
struct GameRoom {
    players: Vec<GameRoomPlayer>,
    state: GameRoomState,
}
struct GameRoomState {
    step: u32,
    deck: [Card; 52],
    turn_timer: u16,
    turn_duration: u16,
    community_cards: Vec<Card>,
    current_blind_idx: u8,
    dealt_card_offset: usize,
    current_bet_base: u32
}
#[derive(Clone)]
struct GameRoomPlayerState {
    is_active: bool,
    dealt_cards: Vec<Card>,
    current_bet: u32,
    action: PlayerGameAction
}
#[derive(Clone)]
enum PlayerGameAction {
    None,
    Fold,
    Check,
    Call,
    Raise(u32)
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
            community_cards: Vec::new(),
            current_blind_idx: 0,
            turn_timer: 0,
            turn_duration: 60,
            dealt_card_offset: 0,
            current_bet_base: 0
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
                            is_active: false,
                            dealt_cards: Vec::new(),
                            current_bet: 0,
                            action: PlayerGameAction::None
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

async fn handle_poker_step(
    step: PokerStep,
    mut gameroom_state:GameRoomState,
    mut players:Vec<GameRoomPlayer>
) {
    for player in players.iter_mut() {
        player.state.action = PlayerGameAction::None;
    }
    match step {
        PokerStep::Blind => {
            let mut rng = rand::rng();
            gameroom_state.deck.shuffle(&mut rng);
            gameroom_state.community_cards.clear();

            for player in players.iter_mut() {
                player.state.is_active = true;
                player.state.dealt_cards.clear();
            }

            let n_players = players.iter().len() as u8;
            let small_blind = gameroom_state.current_blind_idx % n_players as u8;
            let big_blind = (small_blind + 1) % n_players as u8;

            gameroom_state.current_blind_idx = big_blind;
        },
        PokerStep::PreFlop => {
            gameroom_state.dealt_card_offset = 0;

            for player in players.iter_mut() {
                if !player.state.is_active { continue; }

                player.state.dealt_cards.push(
                    gameroom_state.deck[gameroom_state.dealt_card_offset]
                );
                player.state.dealt_cards.push(
                    gameroom_state.deck[gameroom_state.dealt_card_offset + 1]
                );
                gameroom_state.dealt_card_offset += 2;
            }
        },
        PokerStep::Flop => {
            gameroom_state.community_cards.push(
                    gameroom_state.deck[gameroom_state.dealt_card_offset]
            );
            gameroom_state.community_cards.push(
                    gameroom_state.deck[gameroom_state.dealt_card_offset + 1]
            );
            gameroom_state.community_cards.push(
                    gameroom_state.deck[gameroom_state.dealt_card_offset + 2]
            );
            gameroom_state.dealt_card_offset += 3;
        },
        PokerStep::Turn => {
            gameroom_state.community_cards.push(
                    gameroom_state.deck[gameroom_state.dealt_card_offset]
            );
            gameroom_state.dealt_card_offset += 1;
        },
        PokerStep::River => {
            gameroom_state.community_cards.push(
                    gameroom_state.deck[gameroom_state.dealt_card_offset]
            );
            gameroom_state.dealt_card_offset += 1;
            
        },
        PokerStep::Showdown => {
            
        },
        PokerStep::BettingRound => {
            loop {
                for player in players.iter_mut() {
                    if !player.state.is_active { continue; }
                    
                    let mut turn_timer = gameroom_state.turn_duration;
                    while turn_timer > 0 {
                        turn_timer -= 1;

                        match player.state.action {
                            PlayerGameAction::None => { },
                            PlayerGameAction::Fold => { 
                                player.state.is_active = false;
                                break;
                            },
                            PlayerGameAction::Call => {
                                player.state.current_bet = gameroom_state.current_bet_base;
                                break;
                            },
                            PlayerGameAction::Check => {
                                if player.state.current_bet == gameroom_state.current_bet_base {
                                    break;
                                }
                                else if player.state.current_bet > gameroom_state.current_bet_base {
                                    gameroom_state.current_bet_base = player.state.current_bet;
                                }
                            },
                            PlayerGameAction::Raise(raise) => {
                                gameroom_state.current_bet_base += raise;
                                player.state.current_bet += raise;
                                break;
                            }
                        }
                        tokio::time::sleep(tokio::time::Duration::from_secs(1)).await;
                    }

                    if player.state.current_bet < gameroom_state.current_bet_base {
                        player.state.is_active = false;
                    }
                }

                let mut active_players = players.iter().filter(|player| player.state.is_active);
                if
                    active_players.clone().count() <= 1 ||
                    active_players.all(|player| player.state.current_bet == gameroom_state.current_bet_base)
                { break; }
            }
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

        //tokio::time::sleep(tokio::time::Duration::new(1, 0)).await;
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

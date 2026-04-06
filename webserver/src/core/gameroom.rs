use tokio::sync::{ Mutex, MutexGuard, mpsc, oneshot };
use tokio;
use uuid;
use axum::extract::ws::WebSocket;
use std::sync::Arc;
use std::time::Duration;
use crate::core::game::{self, GameType};
use crate::core::hand::{ compare_hands, HandCompare };
use crate::core::player::{ PlayerMessage, Player };
use crate::core::card::{ Card, DECK };
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
    game_type: GameType,
    min_bet: u32
}
struct GameRoomState {
    step: u32,
    deck: [Card; 52],
    turn_timer: u16,
    turn_duration: u16,
    community_cards: Vec<Card>,
    big_blind_idx: u8,
    dealt_card_offset: usize,
    bet_base: u32
}
#[derive(Clone)]
struct GameRoomPlayerState {
    is_playing: bool,
    is_betting: bool,
    dealt_cards: Vec<Card>,
    bet: u32,
    action: PlayerGameAction,
    funds: u32,
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

struct GameRoomStateNotification {
    content: String
}

impl GameRoom {
    fn new(game_type: GameType) -> Self {
        let players = Vec::new();
        let state = GameRoomState {
            step: 0,
            deck: DECK,
            community_cards: Vec::new(),
            big_blind_idx: 0,
            turn_timer: 0,
            turn_duration: 60,
            dealt_card_offset: 0,
            bet_base: 0
        };

        Self {
            // id: uuid::Uuid::new_v4(), //(unneeded for now)
            players,
            state,
            game_type,
            min_bet: 1
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
                            is_playing: false,
                            is_betting: false,
                            dealt_cards: Vec::new(),
                            bet: 0,
                            action: PlayerGameAction::None,
                            funds: 1_000
                        }
                    }
                );
            },
            GameRoomMessage::PlayerPayload { from, content } => {
                //println!("Gameroom {} received {} from Player {}", self.id, content, from);
                println!("Gameroom received {} from Player {}", content, from);
                self.state.step = 0; // JUST FOR DEBUGGING

                // TODO: Manage and validate player actions (check, raise, fold, call, etc)
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

async fn handle_step_blind(
    gameroom: &mut GameRoom
) {
    let mut rng = rand::rng();
    gameroom.state.deck.shuffle(&mut rng);
    gameroom.state.community_cards.clear();

    for player in gameroom.players.iter_mut() {
        player.state.is_betting = true;
        player.state.dealt_cards.clear();
    }

    let n_players = gameroom.players.iter().len() as u8;
    let small_blind_idx = gameroom.state.big_blind_idx % n_players as u8;
    gameroom.state.big_blind_idx = (small_blind_idx + 1) % n_players as u8;

    match gameroom.players.get_mut(small_blind_idx as usize) {
        Some(player) => {
            player.state.bet = gameroom.min_bet;
        },
        None => {}
    }
    match gameroom.players.get_mut(gameroom.state.big_blind_idx as usize) {
        Some(player) => {
            player.state.bet = 2 * gameroom.min_bet;
        },
        None => {}
    }
}

async fn handle_step_preflop(
    gameroom: &mut GameRoom
) {
    gameroom.state.dealt_card_offset = 0;

    for player in gameroom.players.iter_mut() {
        if !player.state.is_betting { continue; }

        for i in 0..2 {
            player.state.dealt_cards.push(
                gameroom.state.deck[gameroom.state.dealt_card_offset + i]
            );
        }
        gameroom.state.dealt_card_offset += 2;
    }
}

async fn handle_step_flop(
    gameroom: &mut GameRoom
) {
    for i in 0..3 {
        gameroom.state.community_cards.push(
            gameroom.state.deck[gameroom.state.dealt_card_offset + i]
        );
    }
    gameroom.state.dealt_card_offset += 3;
}

async fn handle_step_deal_player_card(
    gameroom: &mut GameRoom
) {
    gameroom.state.community_cards.push(
            gameroom.state.deck[gameroom.state.dealt_card_offset]
    );
    gameroom.state.dealt_card_offset += 1;
}

async fn handle_step_betting_round(
    gameroom_mutex: Arc<Mutex<GameRoom>>,
    mut notification_receiver: &mut oneshot::Receiver<GameRoomStateNotification>
) {
    loop {
        let senders: Vec<mpsc::Sender<PlayerMessage>>;
        let n_players: usize;
        {
            let gameroom = gameroom_mutex.lock().await;
            senders = gameroom.players.iter().map(|player| player.sender.clone()).collect();
            n_players = gameroom.players.len();
        }

        for player_idx in 0..n_players {
        // for player in gameroom.players.iter_mut() {
            let player_is_betting: bool;
            let mut turn_timer: f32;
            {
                let gameroom = gameroom_mutex.lock().await;
                player_is_betting = gameroom.players[player_idx].state.is_betting;
                turn_timer = gameroom.state.turn_duration as f32;
            }

            if !player_is_betting { continue; }

            while turn_timer > 0.0 {
                let tick_start = tokio::time::Instant::now();
                for sender in senders.iter() {
                    let timer_payload = PlayerMessage::GameRoomPayload {
                        content: format!("timer: {}", turn_timer)
                    };
                    let _ = sender.send(timer_payload).await;
                }
                let tick_end = tick_start + Duration::from_secs(1);
                loop {
                    let remaining = tick_end.saturating_duration_since(tokio::time::Instant::now());
                    if remaining.is_zero() { break; }
                    match tokio::time::timeout(remaining, &mut notification_receiver).await {
                        Ok(_) => {},
                        Err(_) => break,
                    }
                }


                {
                    let mut gameroom = gameroom_mutex.lock().await;
                    let bet_base = gameroom.state.bet_base;
                    let mut bet_base_update = gameroom.state.bet_base;
                    let mut is_action = true;

                    match gameroom.players.get_mut(player_idx) {
                        Some(player) => {
                            match player.state.action {
                                PlayerGameAction::None => {
                                    is_action = false;
                                },
                                PlayerGameAction::Fold => {
                                    player.state.is_betting = false;
                                },
                                PlayerGameAction::Call => {
                                    player.state.bet = bet_base;
                                },
                                PlayerGameAction::Check => {
                                    if player.state.bet == bet_base {
                                    }
                                    else {
                                        // Cannot Check if bet base changed
                                        is_action = false;
                                        let warning = PlayerMessage::GameRoomPayload { content: "Error".to_string() };
                                        let _ = player.sender.send(warning).await;
                                    }
                                },
                                PlayerGameAction::Raise(raise) => {
                                    bet_base_update += raise;
                                    player.state.bet += bet_base_update;
                                }
                            }
                        },
                        None => { is_action = false; }
                    }
                    if is_action {
                        gameroom.state.bet_base = bet_base_update;
                        break;
                    }
                }

                turn_timer -= tick_start.elapsed().as_secs_f32();
            }

            {
                let mut gameroom = gameroom_mutex.lock().await;
                let bet_base = gameroom.state.bet_base;

                match gameroom.players.get_mut(player_idx) {
                    Some(player) => {
                        if player.state.bet < bet_base {
                            player.state.is_betting = false;
                        }
                    },
                    None => {}
                }
            }
        }

        {
            let gameroom = gameroom_mutex.lock().await;
            let mut active_players = gameroom.players.iter().filter(|player| player.state.is_betting);
            if
                active_players.clone().count() <= 1 ||
                active_players.all(|player| player.state.bet == gameroom.state.bet_base)
            { break; }
        }
    }
}

async fn handle_step_showdown(
    gameroom: &mut GameRoom
) {
    let hands: Vec<Vec<Card>> = gameroom.players.iter().map(
        |player|  player.state.dealt_cards.iter()
            .chain(gameroom.state.community_cards.iter())
            .map(|card| card.clone()).collect()
    ).collect();

    let bet_sum: u32 = gameroom.players.iter().map(|player| player.state.bet).sum();

    match compare_hands(hands, gameroom.game_type) {
        Ok(result) => {
            match result {
                HandCompare::Tie(tied_indexes) => {
                    // handle tie between player indexes
                    let divided_amount = bet_sum / tied_indexes.len() as u32;

                    for player_idx in tied_indexes {
                        match gameroom.players.get_mut(player_idx) {
                            Some(player) => {
                                player.state.funds += divided_amount;
                            },
                            None => {}
                        }
                    }
                },
                HandCompare::Winner(winner_index) => {
                    match gameroom.players.get_mut(winner_index) {
                        Some(player) => {
                            player.state.funds += bet_sum;
                        },
                        None => {}
                    }
                }
            }

            gameroom.state.bet_base = 0;
        },
        Err(err) => {}
    }
}

async fn handle_poker_step(
    step: PokerStep,
    // gameroom: &mut GameRoom,
    gameroom_mutex: Arc<Mutex<GameRoom>>,
    mut notification_receiver: &mut oneshot::Receiver<GameRoomStateNotification>
) {
    for player in gameroom_mutex.lock().await.players.iter_mut() {
        player.state.action = PlayerGameAction::None;
    }
    match step {
        PokerStep::Blind => {
            let mut gameroom = gameroom_mutex.lock().await;
            handle_step_blind(&mut gameroom).await;
        },
        PokerStep::PreFlop => {
            let mut gameroom = gameroom_mutex.lock().await;
            handle_step_preflop(&mut gameroom).await;
        },
        PokerStep::Flop => {
            let mut gameroom = gameroom_mutex.lock().await;
            handle_step_flop(&mut gameroom).await;
        },
        PokerStep::Turn => {
            let mut gameroom = gameroom_mutex.lock().await;
            handle_step_deal_player_card(&mut gameroom).await;
        },
        PokerStep::River => {
            let mut gameroom = gameroom_mutex.lock().await;
            handle_step_deal_player_card(&mut gameroom).await;
        },
        PokerStep::Showdown => {
            let mut gameroom = gameroom_mutex.lock().await;
            handle_step_showdown(&mut gameroom).await;
        },
        PokerStep::BettingRound => {
            handle_step_betting_round(gameroom_mutex, &mut notification_receiver).await;
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
    mut receiver: mpsc::Receiver<GameRoomMessage>,
    mut notification_sender: oneshot::Sender<GameRoomStateNotification>
) {
    while let Some(message) = receiver.recv().await {
        gameroom.lock().await.handle_gameroom_message(message).await;
    }
}
async fn gameroom_state_loop(
    gameroom: Arc<Mutex<GameRoom>>,
    mut notification_receiver: oneshot::Receiver<GameRoomStateNotification>
) {
    loop {
        {
            if gameroom.lock().await.players.len() == 0 { continue;}
        }


        for _ in 0..2 {
            for step in STANDARD_POKER_STEPS {
                handle_poker_step(
                    step,
                    gameroom.clone(),
                    &mut notification_receiver
                ).await;
            }
        }

        {
            // {
            //     let mut gameroom_guard = gameroom.lock().await;
            //     let mut removed_players = Vec::new();
            //
            //     for player in gameroom_guard.players.iter() {
            //         match player.sender.clone().send(PlayerMessage::GameRoomPayload { content: "test message".to_string() }).await {
            //             Ok(_) => println!("Message sent to player {}", player.id),
            //             Err(err) => eprintln!("FAILED to send to {}: {}", player.id, err),
            //         }
            //         tokio::time::sleep(tokio::time::Duration::new(1, 0)).await;
            //         let _ = player.sender.send(PlayerMessage::TerminateSession).await;
            //         removed_players.push(player.clone());
            //     }
            //
            //     for player in removed_players {
            //         gameroom_guard.players.retain(|val| val.id != player.id);
            //     }
            // }
            //
            // Wait till enough players ready
            // _ = tokio::time::timeout(
            //     Duration::from_secs(2),
            //     &mut notification_receiver
            // ).await;


            //println!("state: {}", gameroom_guard.state.step);
        }
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
            GameRoom::new(GameType::TexasHoldemPoker)
        ));

        let (notif_sender, notif_receiver) = oneshot::channel();

        tokio::spawn(gameroom_message_loop(gameroom_mutex.clone(), receiver, notif_sender));
        tokio::spawn(gameroom_state_loop(gameroom_mutex, notif_receiver));


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

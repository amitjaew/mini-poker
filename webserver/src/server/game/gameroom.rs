use serde::{Deserialize, Serialize};
use tokio::sync::{ Mutex, mpsc };
use tokio;
use axum::extract::ws::WebSocket;
use tokio::time::{Instant};
use uuid::Uuid;
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use crate::core::game::GameType;
use crate::core::hand::{ compare_hands, HandCompare };
use crate::server::game::player::{ PlayerMessage, PlayerSession, PlayerWarningType };
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

impl GameRoom {
    async fn broadcast(self: &Self, message: PlayerMessage) {
        for player in self.players.iter() {
            _ = player.sender.send(message.clone()).await;
        }
    }
}

struct GameRoomState {
    deck: [Card; 52],
    turn_duration: u16,
    community_cards: Vec<Card>,
    big_blind_idx: u8,
    dealt_card_offset: usize,
    bet_base: u32,
    current_player_turn: Option<Uuid>,
    current_player_timeout: Option<SystemTime>
}

#[derive(Clone)]
struct GameRoomPlayerState {
    is_betting: bool,
    dealt_cards: Vec<Card>,
    bet: u32,
    action: PlayerGameAction,
    funds: u32,
}

#[derive(Clone, Serialize, Deserialize)]
pub enum PlayerGameAction {
    None,
    Fold,
    Check,
    Call,
    Raise(u32)
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum PlayerPayload {
    Fold,
    Check,
    Call,
    Raise { amount: u32 },
    Pong {
        client_ts: u64,
        server_ts: u64
    }
}

pub enum GameRoomMessage {
    PlayerPayload { payload: PlayerPayload, from: uuid::Uuid },
    PlayerJoin { id: uuid::Uuid, sender: mpsc::Sender<PlayerMessage> }
}

struct GameRoomStateNotification {
    content: String
}

impl GameRoom {
    fn new(game_type: GameType) -> Self {
        let players = Vec::new();
        let state = GameRoomState {
            deck: DECK,
            community_cards: Vec::new(),
            big_blind_idx: 0,
            turn_duration: 10,
            dealt_card_offset: 0,
            bet_base: 0,
            current_player_turn: None,
            current_player_timeout: None
        };

        Self {
            players,
            state,
            game_type,
            min_bet: 1
        }
    }

    async fn handle_gameroom_message(
        &mut self, message: GameRoomMessage,
        notification_sender: &mut mpsc::Sender<GameRoomStateNotification>
    ) {
        match message {
            GameRoomMessage::PlayerJoin { id, sender } => {
                match self.players.iter_mut().find(|player| player.id == id) {
                    Some(player) => {
                        player.sender = sender;
                    },
                    None => {
                        self.players.push(
                            GameRoomPlayer{
                                id,
                                sender,
                                state: GameRoomPlayerState {
                                    is_betting: false,
                                    dealt_cards: Vec::new(),
                                    bet: 0,
                                    action: PlayerGameAction::None,
                                    funds: 1_000
                                }
                            }
                        );
                    }
                }
            },
            GameRoomMessage::PlayerPayload { from, payload } => {
                println!("Gameroom received {:?} from Player {}", payload, from);

                let mut _player = self.players.iter_mut().find(|player| player.id == from);
                if _player.is_none() {
                    return;
                }
                let player = _player.unwrap();

                match payload {
                    PlayerPayload::Fold => {
                        player.state.action = PlayerGameAction::Fold;
                        _ = notification_sender.send(
                            GameRoomStateNotification { content: "player updated".to_string() }
                        ).await;
                    },
                    PlayerPayload::Call => {
                        player.state.action = PlayerGameAction::Call;
                        _ = notification_sender.send(
                            GameRoomStateNotification { content: "player updated".to_string() }
                        ).await;
                    },
                    PlayerPayload::Check => {
                        player.state.action = PlayerGameAction::Check;
                        _ = notification_sender.send(
                            GameRoomStateNotification { content: "player updated".to_string() }
                        ).await;
                    },
                    PlayerPayload::Raise { amount } => {
                        player.state.action = PlayerGameAction::Raise(amount);
                        _ = notification_sender.send(
                            GameRoomStateNotification { content: "player updated".to_string() }
                        ).await;
                    },
                    PlayerPayload::Pong { client_ts, server_ts } => {
                        let timer = SystemTime::now().duration_since(UNIX_EPOCH);
                        match timer {
                            Ok(duration) => {
                                let server_payload = PlayerMessage::PongAck {
                                    server_ts,
                                    client_ts,
                                    server_ack_ts: duration.as_millis() as u64
                                };
                                let _ = player.sender.send(server_payload).await;
                            },
                            Err(_) => {}
                        }

                    }
                }
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

async fn handle_step_blind(gameroom: &mut GameRoom) {
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
        Some(player) => { player.state.bet = gameroom.min_bet; },
        None => {}
    }
    match gameroom.players.get_mut(gameroom.state.big_blind_idx as usize) {
        Some(player) => { player.state.bet = 2 * gameroom.min_bet; },
        None => {}
    }
}

async fn handle_step_preflop(gameroom: &mut GameRoom) {
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

async fn handle_step_flop(gameroom: &mut GameRoom) {
    for i in 0..3 {
        gameroom.state.community_cards.push(
            gameroom.state.deck[gameroom.state.dealt_card_offset + i]
        );
    }
    gameroom.state.dealt_card_offset += 3;
}

async fn handle_step_deal_player_card(gameroom: &mut GameRoom) {
    gameroom.state.community_cards.push(
        gameroom.state.deck[gameroom.state.dealt_card_offset]
    );
    gameroom.state.dealt_card_offset += 1;
}

async fn handle_step_betting_round(
    gameroom_mutex: Arc<Mutex<GameRoom>>,
    notification_receiver: &mut mpsc::Receiver<GameRoomStateNotification>
) {
    loop {
        let n_players: usize;
        {
            let gameroom = gameroom_mutex.lock().await;
            n_players = gameroom.players.len();
        }

        for player_idx in 0..n_players {
            let timeout_instant: Instant;
            let timeout_time: SystemTime;

            {
                let mut gameroom = gameroom_mutex.lock().await;
                let player_is_betting = gameroom.players[player_idx].state.is_betting;
                if !player_is_betting { continue; }

                gameroom.state.current_player_turn = Some(gameroom.players[player_idx].id);
                timeout_instant = Instant::now() + Duration::from_secs(gameroom.state.turn_duration as u64);
                timeout_time = SystemTime::now() + Duration::from_secs(gameroom.state.turn_duration as u64);
                gameroom.state.current_player_timeout = Some(timeout_time);

                gameroom.broadcast(
                    PlayerMessage::Turn {
                        player_id: gameroom.players[player_idx].id,
                        timeout: timeout_time.duration_since(UNIX_EPOCH).unwrap().as_millis() as u64
                    }
                ).await;
            }

            while SystemTime::now() < timeout_time {
                match tokio::time::timeout_at(timeout_instant, notification_receiver.recv()).await.unwrap_or(None) {
                    Some(notif) => {
                        print!("State loop received notification: {}", notif.content);
                    },
                    None => {},
                }

                let mut gameroom = gameroom_mutex.lock().await;
                let bet_base = gameroom.state.bet_base;
                let mut bet_base_update = gameroom.state.bet_base;
                let mut is_action = true;
                let mut pending_broadcast: Option<PlayerMessage> = None;

                match gameroom.players.get_mut(player_idx) {
                    Some(player) => {
                        pending_broadcast = Some(PlayerMessage::PlayerAction {
                            player_id: player.id.clone(),
                            action: player.state.action.clone(),
                            bet_base: bet_base_update
                        });
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
                                if player.state.bet != bet_base {
                                    is_action = false;
                                    let warning = PlayerMessage::Warning {
                                        warning_type: PlayerWarningType::InvalidAction,
                                        message: "Cannot check".to_owned()
                                    };
                                    let _ = player.sender.send(warning).await;
                                    player.state.action = PlayerGameAction::None;
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
                    if pending_broadcast.is_some() {
                        gameroom.broadcast(pending_broadcast.unwrap()).await;
                    }
                    break;
                }
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

async fn handle_step_showdown(gameroom: &mut GameRoom) {
    let end_players: Vec<usize> = gameroom.players.iter()
        .enumerate()
        .filter_map(|(idx, player)| player.state.is_betting.then_some(idx))
        .collect();

    let hands: Vec<Vec<Card>> = end_players.iter().map(
        |&idx| gameroom.players[idx].state.dealt_cards.iter()
            .chain(gameroom.state.community_cards.iter())
            .map(|card| card.clone()).collect()
    ).collect();

    let bet_sum: u32 = gameroom.players.iter_mut().map(|player| player.state.bet).sum();
    let winners: Vec<Uuid>;
    let prizes: Vec<u32>;

    match compare_hands(hands, gameroom.game_type) {
        Ok(result) => {
            match result {
                HandCompare::Tie(tied_indexes) => {
                    let divided_amount = if tied_indexes.len() == 0 { 0 } else { bet_sum / tied_indexes.len() as u32 };
                    winners = end_players.iter()
                        .filter(|&idx| tied_indexes.contains(idx))
                        .map(|&idx| gameroom.players[idx].id)
                        .collect();
                    prizes = winners.iter().map(|_| divided_amount).collect(); // PENDING FIX: prizes proportional to bet

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
                    gameroom.players[end_players[winner_index]].state.funds += bet_sum;
                    winners = vec![gameroom.players[end_players[winner_index]].id.clone()];
                    prizes = vec![ bet_sum ];
                }
            }

            gameroom.broadcast(PlayerMessage::Result { winners, prizes }).await;
            gameroom.state.bet_base = 0;
        },
        Err(err) => {
            println!("Error comparing hands: {:?}", err);
        },
    }
}

async fn handle_poker_step(
    step: PokerStep,
    gameroom_mutex: Arc<Mutex<GameRoom>>,
    notification_receiver: &mut mpsc::Receiver<GameRoomStateNotification>
) {
    for player in gameroom_mutex.lock().await.players.iter_mut() {
        player.state.action = PlayerGameAction::None;
    }
    match step {
        PokerStep::Blind    => { handle_step_blind(&mut *gameroom_mutex.lock().await).await; },
        PokerStep::PreFlop  => { handle_step_preflop(&mut *gameroom_mutex.lock().await).await; },
        PokerStep::Flop     => { handle_step_flop(&mut *gameroom_mutex.lock().await).await; },
        PokerStep::Turn     => { handle_step_deal_player_card(&mut *gameroom_mutex.lock().await).await; },
        PokerStep::River    => { handle_step_deal_player_card(&mut *gameroom_mutex.lock().await).await; },
        PokerStep::Showdown => { handle_step_showdown(&mut *gameroom_mutex.lock().await).await; },
        PokerStep::BettingRound => {
            handle_step_betting_round(gameroom_mutex, notification_receiver).await;
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
    mut notification_sender: mpsc::Sender<GameRoomStateNotification>
) {
    while let Some(message) = receiver.recv().await {
        gameroom.lock().await.handle_gameroom_message(
            message,
            &mut notification_sender
        ).await;
    }
}

async fn gameroom_state_loop(
    gameroom: Arc<Mutex<GameRoom>>,
    mut notification_receiver: mpsc::Receiver<GameRoomStateNotification>
) {
    loop {
        if gameroom.lock().await.players.len() == 0 { continue; }
        tokio::time::sleep(Duration::from_secs(5)).await;

        for _ in 0..2 {
            for step in STANDARD_POKER_STEPS {
                handle_poker_step(
                    step,
                    gameroom.clone(),
                    &mut notification_receiver
                ).await;
            }
        }
    }
}

pub struct GameRoomHandle {
    pub id: uuid::Uuid,
    sender: mpsc::Sender<GameRoomMessage>
}

impl GameRoomHandle {
    pub async fn new(game_type: GameType) -> Self {
        let (sender, receiver) = mpsc::channel(100);
        let gameroom_mutex = Arc::new(Mutex::new(
            GameRoom::new(game_type)
        ));

        let (notif_sender, notif_receiver) = mpsc::channel(10);
        tokio::spawn(gameroom_message_loop(gameroom_mutex.clone(), receiver, notif_sender));
        tokio::spawn(gameroom_state_loop(gameroom_mutex, notif_receiver));

        Self {
            id: uuid::Uuid::new_v4(),
            sender
        }
    }

    pub async fn handle_player_connection(&self, websocket: WebSocket, player_id: Uuid) {
        let (player_sender, player_receiver) = mpsc::channel(10);
        let gameroom_sender = self.sender.clone();

        let player = PlayerSession::new(
            player_id,
            gameroom_sender,
            player_sender.clone(),
            player_receiver,
            websocket
        );
        let _ = self.sender.send(
            GameRoomMessage::PlayerJoin {
                id: player.id.clone(),
                sender: player_sender
            }
        ).await;
    }
}

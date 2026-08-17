use crate::core::card::{Card, Owner, DECK};
use crate::core::game::GameType;
use crate::core::hand::compare_hands;
use crate::server::game::player::{
    CardDealDTO, CardOwnerDTO, CardReveallDTO, HandRevealDTO, PlayerMessage, PlayerSession,
    PlayerWarningType,
};
use axum::extract::ws::WebSocket;
use rand;
use rand::seq::SliceRandom;
use serde::{Deserialize, Serialize};
use std::sync::Arc;
use std::time::{Duration, SystemTime, UNIX_EPOCH};
use tokio;
use tokio::sync::{mpsc, Mutex};
use tokio::time::Instant;
use uuid::Uuid;

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
    turn_duration: u16,
    min_bet: u32,
    min_funds: u32,
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
    community_cards: Vec<Card>,
    big_blind_idx: u8,
    dealt_card_offset: usize,
    bet_base: u32,
    current_player_turn: Option<Uuid>,
    current_player_timeout: Option<SystemTime>,
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

#[derive(Clone, Serialize, Deserialize)]
pub enum PlayerGameAction {
    None,
    Fold,
    Check,
    Call,
    Raise(u32),
}

#[derive(Serialize, Deserialize, Debug)]
#[serde(tag = "type", rename_all = "snake_case")]
pub enum PlayerAction {
    Fold,
    Check,
    Call,
    Raise { amount: u32 },
    Pong { client_ts: u64, server_ts: u64 },
    Update { is_playing: bool },
}

pub enum GameRoomMessage {
    PlayerAction {
        payload: PlayerAction,
        from: uuid::Uuid,
    },
    PlayerJoin {
        id: uuid::Uuid,
        sender: mpsc::Sender<PlayerMessage>,
    },
}

struct GameRoomStateNotification {
    content: String,
}

struct GameRoomConfig {
    min_bet: u32,
    min_funds: u32,
    turn_duration: u16,
    game_type: GameType,
}

impl GameRoom {
    fn new(config: GameRoomConfig) -> Self {
        let players = Vec::new();
        let state = GameRoomState {
            deck: DECK,
            community_cards: Vec::new(),
            big_blind_idx: 0,
            dealt_card_offset: 0,
            bet_base: 0,
            current_player_turn: None,
            current_player_timeout: None,
        };

        assert!(
            config.min_funds >= 2 * config.min_bet,
            "Min funds should be more or equal than big blind"
        );

        Self {
            players,
            state,
            game_type: config.game_type,
            min_bet: config.min_bet,
            min_funds: config.min_funds,
            turn_duration: config.turn_duration,
        }
    }

    async fn handle_gameroom_message(
        &mut self,
        message: GameRoomMessage,
        notification_sender: &mut mpsc::Sender<GameRoomStateNotification>,
    ) {
        match message {
            GameRoomMessage::PlayerJoin { id, sender } => {
                match self.players.iter_mut().find(|player| player.id == id) {
                    Some(player) => {
                        player.sender = sender;
                    }
                    None => {
                        self.players.push(GameRoomPlayer {
                            id,
                            sender,
                            state: GameRoomPlayerState {
                                is_playing: false,
                                is_betting: false,
                                dealt_cards: Vec::new(),
                                bet: 0,
                                action: PlayerGameAction::None,
                                funds: 1_000,
                            },
                        });
                    }
                }
            }
            GameRoomMessage::PlayerAction { from, payload } => {
                println!("Gameroom received {:?} from Player {}", payload, from);

                let mut _player = self.players.iter_mut().find(|player| player.id == from);
                if _player.is_none() {
                    return;
                }
                let player = _player.unwrap();

                match payload {
                    PlayerAction::Update { is_playing } => {
                        if is_playing && player.state.funds >= self.min_funds {
                            player.state.is_playing = is_playing;
                        }
                    }
                    PlayerAction::Fold => {
                        player.state.action = PlayerGameAction::Fold;
                        _ = notification_sender
                            .send(GameRoomStateNotification {
                                content: "player updated".to_string(),
                            })
                            .await;
                    }
                    PlayerAction::Call => {
                        player.state.action = PlayerGameAction::Call;
                        _ = notification_sender
                            .send(GameRoomStateNotification {
                                content: "player updated".to_string(),
                            })
                            .await;
                    }
                    PlayerAction::Check => {
                        player.state.action = PlayerGameAction::Check;
                        _ = notification_sender
                            .send(GameRoomStateNotification {
                                content: "player updated".to_string(),
                            })
                            .await;
                    }
                    PlayerAction::Raise { amount } => {
                        player.state.action = PlayerGameAction::Raise(amount);
                        _ = notification_sender
                            .send(GameRoomStateNotification {
                                content: "player updated".to_string(),
                            })
                            .await;
                    }
                    PlayerAction::Pong {
                        client_ts,
                        server_ts,
                    } => {
                        let timer = SystemTime::now().duration_since(UNIX_EPOCH);
                        match timer {
                            Ok(duration) => {
                                let server_payload = PlayerMessage::PongAck {
                                    server_ts,
                                    client_ts,
                                    server_ack_ts: duration.as_millis() as u64,
                                };
                                let _ = player.sender.send(server_payload).await;
                            }
                            Err(_) => {}
                        }
                    }
                }
            }
        }
    }
}

#[derive(Serialize, Deserialize, Clone)]
#[serde(rename_all = "snake_case")]
pub enum PokerStep {
    Blind,
    PreFlop,
    Flop,
    Turn,
    River,
    Showdown,
    BettingRound,
}

async fn handle_step_blind(gameroom: &mut GameRoom) {
    let mut rng = rand::rng();
    gameroom.state.deck.shuffle(&mut rng);
    gameroom.state.community_cards.clear();

    for player in gameroom.players.iter_mut() {
        if player.state.is_playing && player.state.funds < gameroom.min_funds {
            player.state.is_playing = false;
        }
        player.state.is_betting = player.state.is_playing;
        player.state.dealt_cards.clear();
        player.state.bet = 0;
    }

    let n_players = gameroom.players.iter().len() as u8;
    let small_blind_idx = gameroom.state.big_blind_idx % n_players as u8;
    gameroom.state.big_blind_idx = (small_blind_idx + 1) % n_players as u8;

    gameroom.state.bet_base = gameroom.min_bet * 2;

    match gameroom.players.get_mut(small_blind_idx as usize) {
        Some(player) => {
            player.state.bet = gameroom.min_bet;
            player.state.funds -= gameroom.min_bet;
        }
        None => {}
    }
    match gameroom
        .players
        .get_mut(gameroom.state.big_blind_idx as usize)
    {
        Some(player) => {
            player.state.bet = 2 * gameroom.min_bet;
            player.state.funds -= 2 * gameroom.min_bet;
        }
        None => {}
    }
}

async fn handle_step_preflop(gameroom: &mut GameRoom) {
    gameroom.state.dealt_card_offset = 0;
    let hole_count = match gameroom.game_type {
        GameType::TexasHoldemPoker => 2,
        GameType::OmahaPoker => 4,
    };

    for player in gameroom.players.iter_mut() {
        if !player.state.is_betting {
            continue;
        }

        for i in 0..hole_count {
            let mut card = gameroom.state.deck[gameroom.state.dealt_card_offset + i];
            card.owner = Owner::Player;
            player.state.dealt_cards.push(card);
        }

        _ = player
            .sender
            .send(PlayerMessage::CardDeal {
                cards: player
                    .state
                    .dealt_cards
                    .iter()
                    .map(|card| CardDealDTO {
                        rank: card.rank as u8,
                        suit: card.suit.into(),
                    })
                    .collect(),
                owner: CardOwnerDTO::Player,
            })
            .await;
        gameroom.state.dealt_card_offset += hole_count;
    }
}

async fn handle_step_flop(gameroom: &mut GameRoom) {
    for i in 0..3 {
        gameroom
            .state
            .community_cards
            .push(gameroom.state.deck[gameroom.state.dealt_card_offset + i]);
    }

    gameroom
        .broadcast(PlayerMessage::CardDeal {
            cards: gameroom
                .state
                .community_cards
                .iter()
                .map(|card| CardDealDTO {
                    rank: card.rank as u8,
                    suit: card.suit.into(),
                })
                .collect(),
            owner: CardOwnerDTO::Community,
        })
        .await;
    gameroom.state.dealt_card_offset += 3;
}

async fn handle_step_deal_community_cards(gameroom: &mut GameRoom, n_cards: usize) {
    for _ in 0..n_cards {
        gameroom
            .state
            .community_cards
            .push(gameroom.state.deck[gameroom.state.dealt_card_offset]);
        gameroom.state.dealt_card_offset += 1;
    }

    gameroom
        .broadcast(PlayerMessage::CardDeal {
            cards: gameroom
                .state
                .community_cards
                .iter()
                .rev()
                .take(n_cards)
                .map(|card| CardDealDTO {
                    rank: card.rank as u8,
                    suit: card.suit.into(),
                })
                .collect(),
            owner: CardOwnerDTO::Community,
        })
        .await;
}

async fn handle_step_betting_round(
    gameroom_mutex: Arc<Mutex<GameRoom>>,
    notification_receiver: &mut mpsc::Receiver<GameRoomStateNotification>,
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
                if !player_is_betting {
                    continue;
                }

                gameroom.state.current_player_turn = Some(gameroom.players[player_idx].id);
                timeout_instant =
                    Instant::now() + Duration::from_secs(gameroom.turn_duration as u64);
                timeout_time =
                    SystemTime::now() + Duration::from_secs(gameroom.turn_duration as u64);
                gameroom.state.current_player_timeout = Some(timeout_time);

                gameroom
                    .broadcast(PlayerMessage::Turn {
                        player_id: gameroom.players[player_idx].id,
                        timeout: timeout_time.duration_since(UNIX_EPOCH).unwrap().as_millis()
                            as u64,
                    })
                    .await;
            }

            while SystemTime::now() < timeout_time {
                match tokio::time::timeout_at(timeout_instant, notification_receiver.recv())
                    .await
                    .unwrap_or(None)
                {
                    Some(notif) => {
                        print!("State loop received notification: {}", notif.content);
                    }
                    None => {}
                }

                let mut gameroom = gameroom_mutex.lock().await;
                let bet_base = gameroom.state.bet_base;
                let mut bet_base_update = gameroom.state.bet_base;
                let mut is_action = true;
                let mut pending_broadcast: Option<PlayerMessage> = None;

                match gameroom.players.get_mut(player_idx) {
                    Some(player) => {
                        match player.state.action.clone() {
                            PlayerGameAction::None => {
                                is_action = false;
                            }
                            PlayerGameAction::Fold => {
                                player.state.is_betting = false;
                            }
                            PlayerGameAction::Call => {
                                let delta = bet_base - player.state.bet;
                                if player.state.funds < delta {
                                    is_action = false;
                                    _ = player
                                        .sender
                                        .send(PlayerMessage::Warning {
                                            warning_type: PlayerWarningType::InvalidAction,
                                            message: "Not enough funds".to_string(),
                                        })
                                        .await;
                                } else {
                                    player.state.funds -= bet_base - player.state.bet;
                                    player.state.bet = bet_base;
                                }
                            }
                            PlayerGameAction::Check => {
                                if player.state.bet != bet_base {
                                    is_action = false;
                                    let warning = PlayerMessage::Warning {
                                        warning_type: PlayerWarningType::InvalidAction,
                                        message: "Cannot check".to_owned(),
                                    };
                                    let _ = player.sender.send(warning).await;
                                    player.state.action = PlayerGameAction::None;
                                }
                            }
                            PlayerGameAction::Raise(raise) => {
                                let delta = bet_base_update + raise - player.state.bet;
                                if delta > player.state.funds {
                                    is_action = false;
                                    _ = player
                                        .sender
                                        .send(PlayerMessage::Warning {
                                            warning_type: PlayerWarningType::InvalidAction,
                                            message: "Not enough funds".to_string(),
                                        })
                                        .await;
                                } else {
                                    bet_base_update += raise;
                                    player.state.funds -= delta;
                                    player.state.bet = bet_base_update;
                                }
                            }
                        }
                        pending_broadcast = Some(PlayerMessage::PlayerAction {
                            player_id: player.id.clone(),
                            action: player.state.action.clone(),
                            bet_base: bet_base_update,
                        });
                    }
                    None => {
                        is_action = false;
                    }
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
                    }
                    None => {}
                }
            }
        }

        {
            let gameroom = gameroom_mutex.lock().await;
            let mut active_players = gameroom
                .players
                .iter()
                .filter(|player| player.state.is_betting);
            if active_players.clone().count() <= 1
                || active_players.all(|player| player.state.bet == gameroom.state.bet_base)
            {
                break;
            }
        }
    }
}

async fn handle_step_showdown(gameroom: &mut GameRoom) {
    let end_players: Vec<usize> = gameroom
        .players
        .iter()
        .enumerate()
        .filter_map(|(idx, player)| player.state.is_betting.then_some(idx))
        .collect();

    let hands: Vec<Vec<Card>> = end_players
        .iter()
        .map(|&idx| {
            gameroom.players[idx]
                .state
                .dealt_cards
                .iter()
                .chain(gameroom.state.community_cards.iter())
                .map(|card| card.clone())
                .collect()
        })
        .collect();

    let result = compare_hands(hands, gameroom.game_type);
    if !result.is_ok() {
        return;
    }

    let winners: Vec<(usize, Uuid, u32)> = result
        .unwrap_or(Vec::new())
        .iter()
        .map(|&idx| {
            (
                idx,
                gameroom.players[end_players[idx]].id,
                gameroom.players[end_players[idx]].state.bet,
            )
        })
        .collect();

    let bet_cap: u32 = winners
        .iter()
        .max_by_key(|(_, _, bet)| bet)
        .map_or(0, |(_, _, bet)| *bet);

    let bet_pool: u32 = gameroom
        .players
        .iter_mut()
        .map(|player| {
            if player.state.bet > bet_cap {
                player.state.funds += player.state.bet - bet_cap;
                return bet_cap;
            }
            player.state.bet
        })
        .sum();

    let winner_bet_sum: u32 = winners.iter().map(|(_, _, bet)| bet).sum();
    let mut prizes: Vec<u32> = winners
        .iter()
        .map(|(_, _, bet)| *bet * bet_pool / winner_bet_sum)
        .collect();

    let rounding_error: u32 = bet_pool.saturating_sub(prizes.iter().sum());

    if rounding_error > 0 {
        let mut shuffled_prizes_indexes: Vec<usize> = (0..prizes.len()).collect();
        shuffled_prizes_indexes.shuffle(&mut rand::rng());

        for i in 0..rounding_error as usize {
            let price_idx = shuffled_prizes_indexes[i % shuffled_prizes_indexes.len()];
            prizes[price_idx] += 1;
        }
    }

    for (i, (idx, _, _)) in winners.iter().enumerate() {
        let player_idx = end_players[*idx];
        gameroom.players[player_idx].state.funds += prizes[i];
    }

    let player_hands: Vec<HandRevealDTO> = gameroom
        .players
        .iter()
        .map(|player| HandRevealDTO {
            player_id: player.id.clone(),
            cards: player
                .state
                .dealt_cards
                .iter()
                .map(|card| CardReveallDTO {
                    suit: card.suit.into(),
                    rank: card.rank as u8,
                    owner: match card.owner {
                        Owner::Player => CardOwnerDTO::Player,
                        Owner::Community => CardOwnerDTO::Community,
                    },
                })
                .collect(),
        })
        .collect();

    gameroom
        .broadcast(PlayerMessage::Result {
            winners: winners.iter().map(|(_, id, _)| id.to_owned()).collect(),
            prizes,
            player_hands,
        })
        .await;
    gameroom.state.bet_base = 0;
}

async fn handle_poker_step(
    step: PokerStep,
    gameroom_mutex: Arc<Mutex<GameRoom>>,
    notification_receiver: &mut mpsc::Receiver<GameRoomStateNotification>,
) {
    for player in gameroom_mutex.lock().await.players.iter_mut() {
        player.state.action = PlayerGameAction::None;
    }
    match step {
        PokerStep::Blind => {
            handle_step_blind(&mut *gameroom_mutex.lock().await).await;
        }
        PokerStep::PreFlop => {
            handle_step_preflop(&mut *gameroom_mutex.lock().await).await;
        }
        PokerStep::Flop => {
            handle_step_flop(&mut *gameroom_mutex.lock().await).await;
        }
        PokerStep::Turn => {
            handle_step_deal_community_cards(&mut *gameroom_mutex.lock().await, 1).await;
        }
        PokerStep::River => {
            handle_step_deal_community_cards(&mut *gameroom_mutex.lock().await, 1).await;
        }
        PokerStep::Showdown => {
            handle_step_showdown(&mut *gameroom_mutex.lock().await).await;
        }
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
    PokerStep::Showdown,
];

async fn gameroom_message_loop(
    gameroom: Arc<Mutex<GameRoom>>,
    mut receiver: mpsc::Receiver<GameRoomMessage>,
    mut notification_sender: mpsc::Sender<GameRoomStateNotification>,
) {
    while let Some(message) = receiver.recv().await {
        gameroom
            .lock()
            .await
            .handle_gameroom_message(message, &mut notification_sender)
            .await;
    }
}

async fn gameroom_state_loop(
    gameroom: Arc<Mutex<GameRoom>>,
    mut notification_receiver: mpsc::Receiver<GameRoomStateNotification>,
) {
    loop {
        if gameroom.lock().await.players.len() == 0 {
            continue;
        }
        tokio::time::sleep(Duration::from_secs(5)).await;

        for step in STANDARD_POKER_STEPS {
            gameroom
                .lock()
                .await
                .broadcast(PlayerMessage::Step { step: step.clone() })
                .await;

            handle_poker_step(step, gameroom.clone(), &mut notification_receiver).await;
        }
    }
}

pub struct GameRoomHandle {
    pub id: uuid::Uuid,
    sender: mpsc::Sender<GameRoomMessage>,
}

impl GameRoomHandle {
    pub async fn new(game_type: GameType) -> Self {
        let (sender, receiver) = mpsc::channel(100);
        let gameroom_mutex = Arc::new(Mutex::new(GameRoom::new(GameRoomConfig {
            min_bet: 10,
            min_funds: 100,
            turn_duration: 10,
            game_type,
        })));

        let (notif_sender, notif_receiver) = mpsc::channel(10);
        tokio::spawn(gameroom_message_loop(
            gameroom_mutex.clone(),
            receiver,
            notif_sender,
        ));
        tokio::spawn(gameroom_state_loop(gameroom_mutex, notif_receiver));

        Self {
            id: uuid::Uuid::new_v4(),
            sender,
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
            websocket,
        );
        let _ = self
            .sender
            .send(GameRoomMessage::PlayerJoin {
                id: player.id.clone(),
                sender: player_sender.clone(),
            })
            .await;
        _ = player_sender
            .send(PlayerMessage::Session { player_id })
            .await;
    }
}

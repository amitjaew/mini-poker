use sqlx::types::time::PrimitiveDateTime;
use uuid::Uuid;

#[derive(serde::Serialize)]
pub struct UserDTO {
    pub id: Uuid,
    pub email: String,
    pub username: String,
    pub password_hash: String,
    pub at_room: Option<String>,
    pub at_server: Option<String>,
    pub created_at: PrimitiveDateTime,
}

#[derive(serde::Serialize)]
pub struct UserBalanceDTO {
    pub id: Uuid,
    pub user_id: Uuid,
    pub currency: Currency,
    pub amount: i64,
}

#[derive(serde::Serialize)]
pub struct BalanceMovement {
    pub id: Uuid,
    pub balance_id: Uuid,
    pub wallet_address: Option<String>,
    pub amount: i64,
    pub movement_type: BalanceMovementType,
    pub game_type: Option<GameType>,
    pub game_name: Option<String>,
    pub status: BalanceMovementStatus,
    pub created_at: PrimitiveDateTime,
}

#[derive(sqlx::Type, serde::Serialize)]
#[sqlx(type_name = "casino_game_type_enum", rename_all = "snake_case")]
pub enum GameType {
    Slots,
    Poker,
}

#[derive(sqlx::Type, serde::Serialize)]
#[sqlx(type_name = "balance_movement_type_enum", rename_all = "snake_case")]
pub enum BalanceMovementType {
    Deposit,
    Widrawal,
    RoomDeposit,
    RoomWidrawal,
    Prize,
    Bet,
}

#[derive(sqlx::Type, serde::Serialize)]
#[sqlx(type_name = "currency_enum", rename_all = "snake_case")]
pub enum Currency {
    BTC,
    ETH,
    USDC,
    XMR,
    Virtual,
}

#[derive(sqlx::Type, serde::Serialize)]
#[sqlx(type_name = "balance_movement_status_enum", rename_all = "snake_case")]
pub enum BalanceMovementStatus {
    Pending,
    Approved,
    Canceled,
}

DROP TABLE IF EXISTS balance_movements;
DROP TABLE IF EXISTS users_balance;
DROP TABLE IF EXISTS users;

DROP TYPE IF EXISTS balance_movement_status_enum;
DROP TYPE IF EXISTS casino_game_type_enum;
DROP TYPE IF EXISTS balance_movement_type_enum;
DROP TYPE IF EXISTS currency_enum;

CREATE TYPE currency_enum AS ENUM ('BTC', 'ETH', 'USDC', 'XMR', 'VIRTUAL');
CREATE TYPE balance_movement_type_enum AS ENUM ('DEPOSIT', 'WIDRAWAL', 'PRICE', 'BET');
CREATE TYPE casino_game_type_enum AS ENUM ('SLOTS', 'POKER');
CREATE TYPE balance_movement_status_enum AS ENUM ('PENDING', 'APPROVED', 'CANCELED');

CREATE TABLE users (
    id            UUID PRIMARY KEY,
    email         VARCHAR(255) NOT NULL UNIQUE,
    username      VARCHAR(255) NOT NULL UNIQUE,
    password_hash VARCHAR(255) NOT NULL,
    at_room       VARCHAR(255),
    at_server     VARCHAR(255),
    created_at     TIMESTAMP NOT NULL DEFAULT now()
);

CREATE TABLE users_balance (
    id       UUID PRIMARY KEY,
    user_id  UUID NOT NULL REFERENCES users(id),
    currency currency_enum NOT NULL DEFAULT 'BTC',
    amount   NUMERIC(18, 8) NOT NULL
);

CREATE INDEX idx_users_balance_user_id ON users_balance (user_id);

CREATE TABLE balance_movements (
    id             UUID PRIMARY KEY,
    balance_id     UUID NOT NULL REFERENCES users_balance(id),
    wallet_address VARCHAR(255),
    amount         NUMERIC(16, 8) NOT NULL,
    actually_moved NUMERIC(16, 8) NOT NULL DEFAULT 0,
    type           balance_movement_type_enum NOT NULL,
    game_type      casino_game_type_enum,
    game_name      VARCHAR(255),
    status         balance_movement_status_enum NOT NULL DEFAULT 'PENDING',
    created_at     TIMESTAMP NOT NULL DEFAULT now()
);

CREATE INDEX idx_balance_movements_balance_id ON balance_movements (balance_id);
CREATE INDEX idx_balance_movements_created_at ON balance_movements (created_at);

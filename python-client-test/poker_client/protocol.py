from __future__ import annotations

# ── Client → Server action types (serde tag = "type", rename_all = "snake_case")
ACTION_FOLD = "fold"
ACTION_CHECK = "check"
ACTION_CALL = "call"
ACTION_RAISE = "raise"
ACTION_PONG = "pong"
ACTION_UPDATE = "update"

# ── Server → Client message types
MSG_SESSION = "session"
MSG_STEP = "step"
MSG_BET_BASE = "bet_base"
MSG_ACTIVE_PLAYERS = "active_players"
MSG_TURN = "turn"
MSG_GAME_STATE = "game_state"
MSG_BLIND = "blind"
MSG_CARD_DEAL = "card_deal"
MSG_PLAYER_ACTION = "player_action"
MSG_RESULT = "result"
MSG_WARNING = "warning"
MSG_PING = "ping"
MSG_PONG_ACK = "pong_ack"
MSG_TERMINATE_SESSION = "terminate_session"

# Steps that reset the per-hand bet state (serde rename_all = "snake_case").
STEPS_RESET_BET = ("blind", "betting_round")


def encode_action(action_type: str, amount: int | None = None) -> dict:
    if amount is not None:
        return {"type": action_type, "amount": amount}
    return {"type": action_type}


def encode_pong(client_ts: int, server_ts: int) -> dict:
    return {"type": ACTION_PONG, "client_ts": client_ts, "server_ts": server_ts}


def encode_update(is_playing: bool) -> dict:
    return {"type": ACTION_UPDATE, "is_playing": is_playing}


def parse_player_action(action, bet_base: int) -> tuple[str, int]:
    """Parse a server `player_action` payload.

    The server serialises `PlayerGameAction` without a tag, so a raise arrives as
    ``{"Raise": N}`` while fold/check/call arrive as plain strings and ``None``
    arrives as JSON ``null``.

    The broadcast already carries the updated (post-action) ``bet_base``, so it
    is returned as-is. Returns ``(action_name, bet_base)``.
    """
    if isinstance(action, dict):
        if "Raise" in action:
            amount = action["Raise"]
            return f"RAISE+{amount}", bet_base
        name = next(iter(action), "?").upper()
        return name, bet_base

    name = str(action).upper() if action else "?"
    return name, bet_base

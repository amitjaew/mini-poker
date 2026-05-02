from __future__ import annotations
import asyncio
import datetime
import json
import os
import random
import time
from dataclasses import dataclass
from typing import Optional, TYPE_CHECKING

import websockets
from websockets.exceptions import ConnectionClosedOK, ConnectionClosedError

if TYPE_CHECKING:
    from textual.app import App

from messages import (
    CommunityCardsUpdated,
    PlayerConnected,
    PlayerStateUpdated,
    PlayerFundsChanged,
    GameEvent,
    SessionStopped,
)

# Rank::Two=0 … Rank::Ace=12
_RANK_NAMES  = ["2","3","4","5","6","7","8","9","T","J","Q","K","A"]
_SUIT_SYMBOLS = {'c':'♣','d':'♦','h':'♥','s':'♠'}


def fmt_card(suit: str, rank: int) -> str:
    r = _RANK_NAMES[rank] if 0 <= rank < 13 else "?"
    s = _SUIT_SYMBOLS.get(suit, suit)
    return f"{r}{s}"


@dataclass
class PlayerState:
    my_id: Optional[str] = None
    step: Optional[str] = None
    bet_base: int = 0
    turn_player_id: Optional[str] = None
    funds: int = 1000
    status: str = "Waiting"
    last_action: str = "None"
    current_bet: int = 0
    latency_ms: int = 0
    hole_cards_text: str = ""
    community_cards: list = None

    def __post_init__(self):
        if self.community_cards is None:
            self.community_cards = []


def now_ms() -> int:
    return int(time.time() * 1000)


def short(uuid: str) -> str:
    return uuid[:8] if uuid else "?"


def choose_smart_action(state: PlayerState, no_fold: bool = False) -> dict:
    if state.bet_base == 0:
        choice = random.choices(["check", "raise"], weights=[65, 35])[0]
    elif no_fold:
        choice = random.choices(["call", "raise"], weights=[65, 35])[0]
    else:
        choice = random.choices(["call", "raise", "fold"], weights=[60, 30, 10])[0]
    if choice == "raise":
        return {"type": "raise", "amount": random.randint(10, 200)}
    return {"type": choice}


async def run_agent(
    player_index: int,
    url: str,
    app: "App",
    no_fold: bool = False,
    session_id: str = "",
) -> None:
    state = PlayerState()
    pending_action: Optional[asyncio.Task] = None

    log_dir = os.path.join(os.path.dirname(__file__), "logs")
    os.makedirs(log_dir, exist_ok=True)
    sid = session_id or datetime.datetime.now().strftime("%Y%m%d_%H%M%S")
    log_file = open(
        os.path.join(log_dir, f"session_{sid}_P{player_index + 1}.log"),
        "w", encoding="utf-8",
    )

    def dump(direction: str, payload: dict):
        ts = datetime.datetime.now().isoformat(timespec="milliseconds")
        log_file.write(f"{ts} {direction} {json.dumps(payload)}\n")
        log_file.flush()

    def post(msg):
        app.post_message(msg)

    def log(text: str):
        post(GameEvent(player_index, text))

    def upd(field: str, value):
        post(PlayerStateUpdated(player_index, field, value))

    try:
        async with websockets.connect(url) as ws:
            log(f"[green]P{player_index + 1}[/] connected")
            async for raw in ws:
                try:
                    data = json.loads(raw)
                except json.JSONDecodeError:
                    continue

                msg_type = data.get("type")
                dump("RX", data)

                if msg_type == "session":
                    state.my_id = data.get("player_id", "")
                    post(PlayerConnected(player_index, state.my_id))
                    log(f"[green]P{player_index + 1}[/] id=[bold]{short(state.my_id)}[/]")

                elif msg_type == "ping":
                    server_ts = data.get("server_ts", 0)
                    client_ts = now_ms()
                    pong = {"type": "pong", "client_ts": client_ts, "server_ts": server_ts}
                    dump("TX", pong)
                    await ws.send(json.dumps(pong))

                elif msg_type == "pong_ack":
                    rtt = now_ms() - data.get("client_ts", now_ms())
                    state.latency_ms = rtt
                    upd("latency", rtt)

                elif msg_type == "step":
                    step = data.get("step", "?")
                    state.step = step
                    if step in ("blind", "betting_round"):
                        state.bet_base = 0
                        state.current_bet = 0
                    state.last_action = "None"
                    upd("action", "None")
                    upd("bet", 0)
                    upd("status", "Active")
                    log(f"[cyan]▸ {step.upper()}[/] [dim](P{player_index + 1})[/]")

                elif msg_type == "bet_base":
                    state.bet_base = data.get("bet_base", 0)

                elif msg_type == "blind":
                    sb_pid = data.get("small_blind_player", "")
                    bb_pid = data.get("big_blind_player", "")
                    sa     = data.get("small_blind_amount", 0)
                    ba     = data.get("big_blind_amount", 0)
                    state.community_cards = []
                    post(CommunityCardsUpdated(""))
                    log(
                        f"[cyan]Blinds:[/] SB [bold]{short(sb_pid)}[/]({sa}) "
                        f"BB [bold]{short(bb_pid)}[/]({ba})"
                    )

                elif msg_type == "card_deal":
                    cards = data.get("cards", [])
                    owner = data.get("owner", "player")
                    card_strs  = [fmt_card(c.get("suit", "?"), c.get("rank", 0)) for c in cards]
                    cards_text = " ".join(card_strs)
                    if owner == "player":
                        state.hole_cards_text = cards_text
                        upd("cards", cards_text)
                        log(f"[blue]P{player_index + 1} hole:[/] [bold]{cards_text}[/]")
                    else:
                        # Flop sends all 3 at once; turn/river send 1 new card each
                        if len(cards) > 1:
                            state.community_cards = card_strs
                        else:
                            state.community_cards.extend(card_strs)
                        post(CommunityCardsUpdated(" ".join(state.community_cards)))

                elif msg_type == "active_players":
                    players   = data.get("players", [])
                    is_active = bool(state.my_id) and state.my_id in players
                    status    = "Active" if is_active else "Folded"
                    state.status = status
                    upd("status", status)

                elif msg_type == "turn":
                    turn_pid = data.get("player_id", "")
                    state.turn_player_id = turn_pid
                    is_mine  = bool(state.my_id) and state.my_id == turn_pid

                    if is_mine:
                        if pending_action and not pending_action.done():
                            pending_action.cancel()

                        async def think_and_act(ws=ws, state=state):
                            try:
                                await asyncio.sleep(random.uniform(0.3, 1.5))
                                action_data = choose_smart_action(state, no_fold=no_fold)
                                action_type = action_data["type"].upper()
                                amount      = action_data.get("amount", 0)

                                if action_type == "CALL":
                                    delta = max(0, state.bet_base - state.current_bet)
                                    state.current_bet = state.bet_base
                                    state.funds -= delta
                                elif action_type == "RAISE":
                                    state.current_bet = state.bet_base + amount
                                    state.funds -= state.current_bet
                                    state.bet_base = state.current_bet

                                state.last_action = action_type
                                upd("action", action_type)
                                upd("bet", state.current_bet)
                                post(PlayerFundsChanged(player_index, state.funds))
                                suffix = f" {amount}" if action_type == "RAISE" else ""
                                log(f"[yellow]P{player_index + 1}[/] → [bold]{action_type}{suffix}[/]")
                                dump("TX", action_data)
                                await ws.send(json.dumps(action_data))
                            except asyncio.CancelledError:
                                pass
                            except Exception:
                                pass

                        pending_action = asyncio.create_task(think_and_act())

                elif msg_type == "player_action":
                    raw_bet_base = data.get("bet_base", 0)
                    action       = data.get("action")
                    acted_pid    = data.get("player_id", "")
                    if isinstance(action, dict):
                        a_type  = action.get("type", "?").upper()
                        a_amt   = action.get("amount", 0)
                        action_name  = f"{a_type}+{a_amt}" if a_type == "RAISE" else a_type
                        new_bet_base = raw_bet_base + (a_amt if a_type == "RAISE" else 0)
                    else:
                        action_name  = str(action).upper() if action else "?"
                        new_bet_base = raw_bet_base
                    state.bet_base = new_bet_base
                    if acted_pid != state.my_id:
                        log(f"[magenta]{short(acted_pid)}[/] → [bold]{action_name}[/]")

                elif msg_type == "result":
                    winners = data.get("winners", [])
                    prizes  = data.get("prizes", [])
                    state.bet_base    = 0
                    state.current_bet = 0
                    state.hole_cards_text = ""
                    state.community_cards = []
                    upd("cards", "")
                    post(CommunityCardsUpdated(""))
                    if state.my_id in winners:
                        idx   = winners.index(state.my_id)
                        prize = prizes[idx] if idx < len(prizes) else 0
                        state.funds += prize
                        post(PlayerFundsChanged(player_index, state.funds))
                        log(f"[green bold]P{player_index + 1} WON +{prize}![/]")
                    else:
                        log(f"[dim]P{player_index + 1} lost this hand[/]")
                    for hand in data.get("player_hands", []):
                        pid        = hand.get("player_id", "")
                        hand_cards = hand.get("cards", [])
                        card_strs  = [fmt_card(c.get("suit","?"), c.get("rank",0)) for c in hand_cards]
                        log(f"[dim]{short(pid)} showed: {' '.join(card_strs)}[/dim]")
                    upd("status", "Waiting")
                    upd("action", "None")
                    upd("bet", 0)

                elif msg_type == "warning":
                    msg = data.get("message", "")
                    log(f"[red]⚠ P{player_index + 1}:[/] {msg}")

                elif msg_type == "terminate_session":
                    log(f"[red bold]P{player_index + 1} session terminated[/]")
                    break

    except ConnectionClosedOK:
        log(f"[dim]P{player_index + 1} disconnected cleanly[/]")
    except ConnectionClosedError:
        log(f"[red]P{player_index + 1} connection error[/]")
    except asyncio.CancelledError:
        raise
    except Exception as e:
        log(f"[red]P{player_index + 1} error: {e}[/]")
    finally:
        if pending_action and not pending_action.done():
            pending_action.cancel()
        upd("status", "Disconnected")
        post(SessionStopped(player_index, "done"))
        log_file.close()

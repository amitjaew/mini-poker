from __future__ import annotations
import asyncio
import json
import random
import time
from dataclasses import dataclass
from typing import Optional, TYPE_CHECKING

import websockets
from websockets.exceptions import ConnectionClosedOK, ConnectionClosedError

if TYPE_CHECKING:
    from textual.app import App

from messages import (
    PlayerConnected,
    PlayerStateUpdated,
    PlayerFundsChanged,
    GameEvent,
    SessionStopped,
)


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


async def run_agent(player_index: int, url: str, app: "App", no_fold: bool = False) -> None:
    state = PlayerState()
    pending_action: Optional[asyncio.Task] = None

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

                if msg_type == "session":
                    state.my_id = data.get("player_id", "")
                    post(PlayerConnected(player_index, state.my_id))
                    log(f"[green]P{player_index + 1}[/] id=[bold]{short(state.my_id)}[/]")

                elif msg_type == "ping":
                    server_ts = data.get("server_ts", 0)
                    client_ts = now_ms()
                    await ws.send(json.dumps({
                        "type": "pong",
                        "client_ts": client_ts,
                        "server_ts": server_ts,
                    }))

                elif msg_type == "pong_ack":
                    rtt = now_ms() - data.get("client_ts", now_ms())
                    state.latency_ms = rtt
                    upd("latency", rtt)

                elif msg_type == "step":
                    step = data.get("step", "?")
                    state.step = step
                    if step in ("Blind", "BettingRound"):
                        state.bet_base = 0
                        state.current_bet = 0
                    state.last_action = "None"
                    upd("action", "None")
                    upd("bet", 0)
                    upd("status", "Active")
                    log(f"[cyan]▸ {step.upper()}[/] [dim](P{player_index + 1})[/]")

                elif msg_type == "bet_base":
                    state.bet_base = data.get("bet_base", 0)

                elif msg_type == "active_players":
                    players = data.get("players", [])
                    is_active = bool(state.my_id) and state.my_id in players
                    status = "Active" if is_active else "Folded"
                    state.status = status
                    upd("status", status)

                elif msg_type == "turn":
                    turn_pid = data.get("player_id", "")
                    state.turn_player_id = turn_pid
                    is_mine = bool(state.my_id) and state.my_id == turn_pid

                    if is_mine:
                        if pending_action and not pending_action.done():
                            pending_action.cancel()

                        async def think_and_act(ws=ws, state=state):
                            try:
                                await asyncio.sleep(random.uniform(0.3, 1.5))
                                action_data = choose_smart_action(state, no_fold=no_fold)
                                action_type = action_data["type"].upper()
                                amount = action_data.get("amount", 0)

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
                                await ws.send(json.dumps(action_data))
                            except asyncio.CancelledError:
                                pass
                            except Exception:
                                pass

                        pending_action = asyncio.create_task(think_and_act())

                elif msg_type == "player_action":
                    raw_bet_base = data.get("bet_base", 0)
                    action = data.get("action")
                    acted_pid = data.get("player_id", "")
                    if isinstance(action, dict) and "Raise" in action:
                        new_bet_base = raw_bet_base + action["Raise"]
                        action_name = f"RAISE+{action['Raise']}"
                    else:
                        new_bet_base = raw_bet_base
                        action_name = str(action).upper() if action else "?"
                    state.bet_base = new_bet_base
                    if acted_pid != state.my_id:
                        log(f"[magenta]{short(acted_pid)}[/] → [bold]{action_name}[/]")

                elif msg_type == "result":
                    winners = data.get("winners", [])
                    prizes = data.get("prizes", [])
                    state.bet_base = 0
                    state.current_bet = 0
                    if state.my_id in winners:
                        idx = winners.index(state.my_id)
                        prize = prizes[idx] if idx < len(prizes) else 0
                        state.funds += prize
                        post(PlayerFundsChanged(player_index, state.funds))
                        log(f"[green bold]P{player_index + 1} WON +{prize}![/]")
                    else:
                        log(f"[dim]P{player_index + 1} lost this hand[/]")
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

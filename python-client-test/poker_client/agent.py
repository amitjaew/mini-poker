from __future__ import annotations

import asyncio
import json
import random
from typing import Optional, TYPE_CHECKING

import websockets
from websockets.exceptions import ConnectionClosedError, ConnectionClosedOK

if TYPE_CHECKING:
    from textual.app import App

from poker_client.actions import choose_smart_action
from poker_client.cards import fmt_cards
from poker_client.messages import (
    CommunityCardsUpdated,
    GameEvent,
    PlayerConnected,
    PlayerFundsChanged,
    PlayerStateUpdated,
    SessionStopped,
)
from poker_client.protocol import (
    MSG_ACTIVE_PLAYERS,
    MSG_BET_BASE,
    MSG_BLIND,
    MSG_CARD_DEAL,
    MSG_PING,
    MSG_PLAYER_ACTION,
    MSG_PONG_ACK,
    MSG_RESULT,
    MSG_SESSION,
    MSG_STEP,
    MSG_TERMINATE_SESSION,
    MSG_TURN,
    MSG_WARNING,
    STEPS_RESET_BET,
    encode_pong,
    encode_update,
    parse_player_action,
)
from poker_client.session_logger import SessionLogger
from poker_client.state import PlayerState
from poker_client.util import now_ms, short


async def run_agent(
    player_index: int,
    url: str,
    app: "App",
    no_fold: bool = False,
    session_id: str = "",
) -> None:
    state = PlayerState()
    pending_action: Optional[asyncio.Task] = None

    logger = SessionLogger(player_index, session_id)

    def dump(direction: str, payload):
        logger.dump(direction, payload)

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

                if msg_type == MSG_SESSION:
                    state.my_id = data.get("player_id", "")
                    post(PlayerConnected(player_index, state.my_id))
                    log(f"[green]P{player_index + 1}[/] id=[bold]{short(state.my_id)}[/]")

                    # Formal game join.
                    join = encode_update(True)
                    dump("TX", join)
                    await ws.send(json.dumps(join))

                elif msg_type == MSG_PING:
                    pong = encode_pong(now_ms(), data.get("server_ts", 0))
                    dump("TX", pong)
                    await ws.send(json.dumps(pong))

                elif msg_type == MSG_PONG_ACK:
                    state.latency_ms = now_ms() - data.get("client_ts", now_ms())
                    upd("latency", state.latency_ms)

                elif msg_type == MSG_STEP:
                    step = data.get("step", "?")
                    state.step = step
                    if step in STEPS_RESET_BET:
                        state.bet_base = 0
                        state.current_bet = 0
                    state.last_action = "None"
                    upd("action", "None")
                    upd("bet", 0)
                    upd("status", "Active")
                    log(f"[cyan]▸ {step.upper()}[/] [dim](P{player_index + 1})[/]")

                elif msg_type == MSG_BET_BASE:
                    state.bet_base = data.get("bet_base", 0)

                elif msg_type == MSG_BLIND:
                    sb_pid = data.get("small_blind_player", "")
                    bb_pid = data.get("big_blind_player", "")
                    sa = data.get("small_blind_amount", 0)
                    ba = data.get("big_blind_amount", 0)
                    state.community_cards = []
                    post(CommunityCardsUpdated(""))
                    log(
                        f"[cyan]Blinds:[/] SB [bold]{short(sb_pid)}[/]({sa}) "
                        f"BB [bold]{short(bb_pid)}[/]({ba})"
                    )

                elif msg_type == MSG_CARD_DEAL:
                    cards = data.get("cards", [])
                    owner = data.get("owner", "player")
                    cards_text = fmt_cards(cards)
                    if owner == "player":
                        state.hole_cards_text = cards_text
                        upd("cards", cards_text)
                        log(f"[blue]P{player_index + 1} hole:[/] [bold]{cards_text}[/]")
                    else:
                        if len(cards) > 1:
                            state.community_cards = cards_text.split()
                        else:
                            state.community_cards.extend(cards_text.split())
                        post(CommunityCardsUpdated(" ".join(state.community_cards)))

                elif msg_type == MSG_ACTIVE_PLAYERS:
                    players = data.get("players", [])
                    is_active = bool(state.my_id) and state.my_id in players
                    status = "Active" if is_active else "Folded"
                    state.status = status
                    upd("status", status)

                elif msg_type == MSG_TURN:
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

                                state.apply_action(action_type, amount)
                                upd("action", action_type)
                                upd("bet", state.current_bet)
                                post(PlayerFundsChanged(player_index, state.funds))

                                suffix = f" {amount}" if action_type == "RAISE" else ""
                                log(
                                    f"[yellow]P{player_index + 1}[/] "
                                    f"→ [bold]{action_type}{suffix}[/]"
                                )
                                dump("TX", action_data)
                                await ws.send(json.dumps(action_data))
                            except asyncio.CancelledError:
                                pass
                            except Exception:
                                pass

                        pending_action = asyncio.create_task(think_and_act())

                elif msg_type == MSG_PLAYER_ACTION:
                    acted_pid = data.get("player_id", "")
                    action_name, new_bet_base = parse_player_action(
                        data.get("action"), data.get("bet_base", 0)
                    )
                    state.bet_base = new_bet_base
                    if acted_pid != state.my_id:
                        log(f"[magenta]{short(acted_pid)}[/] → [bold]{action_name}[/]")

                elif msg_type == MSG_RESULT:
                    winners = data.get("winners", [])
                    prizes = data.get("prizes", [])
                    state.reset_hand()
                    upd("cards", "")
                    post(CommunityCardsUpdated(""))
                    if state.my_id in winners:
                        idx = winners.index(state.my_id)
                        prize = prizes[idx] if idx < len(prizes) else 0
                        state.apply_prize(prize)
                        post(PlayerFundsChanged(player_index, state.funds))
                        log(f"[green bold]P{player_index + 1} WON +{prize}![/]")
                    else:
                        log(f"[dim]P{player_index + 1} lost this hand[/]")
                    for hand in data.get("player_hands", []):
                        pid = hand.get("player_id", "")
                        log(f"[dim]{short(pid)} showed: {fmt_cards(hand.get('cards', []))}[/dim]")
                    upd("status", "Waiting")
                    upd("action", "None")
                    upd("bet", 0)

                elif msg_type == MSG_WARNING:
                    log(f"[red]⚠ P{player_index + 1}:[/] {data.get('message', '')}")

                elif msg_type == MSG_TERMINATE_SESSION:
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
        logger.close()

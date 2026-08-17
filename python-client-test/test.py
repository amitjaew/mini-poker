import argparse
import asyncio
import json
import random
from typing import Optional

import websockets
from websockets.exceptions import ConnectionClosedOK

from poker_client.actions import choose_smart_action, random_action
from poker_client.protocol import (
    MSG_ACTIVE_PLAYERS,
    MSG_BET_BASE,
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
from poker_client.state import PlayerState
from poker_client.util import now_ms, short

# ── ANSI colours ──────────────────────────────────────────────────────────────
R = "\033[0m"
B = "\033[1m"
DIM = "\033[2m"
CY = "\033[36m"
YL = "\033[33m"
GR = "\033[32m"
RD = "\033[31m"
MG = "\033[35m"


# ── Logging helpers ───────────────────────────────────────────────────────────

def log_verbose(direction: str, raw: str):
    arrow = f"{CY}>>{R}" if direction == "recv" else f"{YL}<<{R}"
    print(f"{arrow} {raw}")


def log_session(player_id: str):
    print(f"  {GR}session{R}  id={B}{short(player_id)}{R}  ({player_id})")


def log_turn(player_id: str, timeout_ts: int, is_mine: bool):
    secs = max(0, round((timeout_ts - now_ms()) / 1000))
    tag = f"{GR}(mine){R}" if is_mine else f"{DIM}(other){R}"
    print(f"\n{B}{GR}TURN{R}  player={B}{short(player_id)}{R}  expires_in={B}{secs}s{R}  {tag}")


def log_step(step: str):
    print(f"\n{B}{CY}STEP{R}  {B}{step.upper()}{R}")


def log_bet_base(bet_base: int):
    print(f"  {DIM}bet_base={B}{bet_base}{R}")


def log_active_players(players: list):
    ids = "  ".join(short(p) for p in players)
    print(f"  {DIM}active ({len(players)}): {ids}{R}")


def log_action_sent(action: str):
    data = json.loads(action)
    t = data.get("type", "?").upper()
    extra = f"  amount={data['amount']}" if t == "RAISE" else ""
    print(f"  {YL}>{R} {B}{t}{R}{extra}")


def log_result(winners: list, prizes: list):
    print(f"\n{B}{MG}== RESULT =={R}")
    for uid, prize in zip(winners, prizes):
        print(f"   {GR}winner{R} {short(uid)}  prize={B}{prize}{R}")


def log_warning(warning_type: dict, message: str):
    wt = warning_type.get("type", "?") if isinstance(warning_type, dict) else str(warning_type)
    print(f"  {RD}! WARNING{R} [{wt}] {message}")


def log_rtt(rtt: int):
    colour = GR if rtt < 50 else (YL if rtt < 150 else RD)
    print(f"  {DIM}rtt={colour}{rtt}ms{R}")


def log_terminate():
    print(f"\n{RD}{B}SESSION TERMINATED{R}")


def log_player_action(player_id: str, action_name: str, bet_base: int):
    print(f"  {MG}*{R} {B}{action_name}{R}  player={short(player_id)}  base={bet_base}")


def log_unknown(msg_type: str, raw: str):
    print(f"  {DIM}? [{msg_type}] {raw}{R}")


# ── Message handling ──────────────────────────────────────────────────────────

async def handle_message(
    websocket,
    raw: str,
    verbose: bool,
    state: Optional[PlayerState] = None,
    no_fold: bool = False,
) -> None:
    try:
        data = json.loads(raw)
    except json.JSONDecodeError:
        if verbose:
            log_verbose("recv", raw)
        else:
            print(f"  {DIM}(non-JSON) {raw}{R}")
        return

    msg_type = data.get("type")

    if verbose and msg_type != MSG_PONG_ACK:
        log_verbose("recv", raw)

    if msg_type == MSG_PING:
        pong = encode_pong(now_ms(), data.get("server_ts", 0))
        payload = json.dumps(pong)
        await websocket.send(payload)
        if verbose:
            log_verbose("send", payload)

    elif msg_type == MSG_PONG_ACK:
        c_ts = data.get("client_ts", 0)
        s_ts = data.get("server_ts", 0)
        ack_ts = data.get("server_ack_ts", 0)
        rtt = now_ms() - c_ts
        if verbose:
            log_verbose("recv", raw)
            print(f"    rtt={rtt}ms  server_processing={ack_ts - s_ts}ms")
        else:
            log_rtt(rtt)

    elif msg_type == MSG_SESSION:
        player_id = data.get("player_id", "")
        if state is not None:
            state.my_id = player_id
        if not verbose:
            log_session(player_id)

        join = json.dumps(encode_update(True))
        await websocket.send(join)
        if verbose:
            log_verbose("send", join)

    elif msg_type == MSG_STEP:
        new_step = data.get("step", "?")
        if state is not None:
            if new_step in STEPS_RESET_BET:
                state.bet_base = 0
                state.current_bet = 0
            state.step = new_step
        if not verbose:
            log_step(new_step)

    elif msg_type == MSG_BET_BASE:
        new_bet_base = data.get("bet_base", 0)
        if state is not None:
            state.bet_base = new_bet_base
        if not verbose:
            log_bet_base(new_bet_base)

    elif msg_type == MSG_ACTIVE_PLAYERS:
        players = data.get("players", [])
        if not verbose:
            log_active_players(players)

    elif msg_type == MSG_TURN:
        player_id = data.get("player_id", "")
        timeout = data.get("timeout", 0)
        is_mine = state is not None and state.my_id is not None and state.my_id == player_id
        if state is not None:
            state.turn_player_id = player_id
        if not verbose:
            log_turn(player_id, timeout, is_mine)
        if is_mine:
            async def think_and_act():
                await asyncio.sleep(random.uniform(0.5, 2.5))
                action_data = choose_smart_action(state, no_fold)
                action_type = action_data["type"].upper()
                amount = action_data.get("amount", 0)
                if state is not None:
                    state.apply_action(action_type, amount)
                action = json.dumps(action_data)
                if verbose:
                    log_verbose("send", action)
                else:
                    log_action_sent(action)
                await websocket.send(action)

            asyncio.create_task(think_and_act())

    elif msg_type == MSG_PLAYER_ACTION:
        action_name, new_bet_base = parse_player_action(
            data.get("action"), data.get("bet_base", 0)
        )
        if state is not None:
            state.bet_base = new_bet_base
        if not verbose:
            log_player_action(data.get("player_id", ""), action_name, new_bet_base)

    elif msg_type == MSG_RESULT:
        if state is not None:
            state.reset_hand()
        if not verbose:
            log_result(data.get("winners", []), data.get("prizes", []))

    elif msg_type == MSG_WARNING:
        if not verbose:
            log_warning(data.get("warning_type", {}), data.get("message", ""))

    elif msg_type == MSG_TERMINATE_SESSION:
        if not verbose:
            log_terminate()

    else:
        if not verbose:
            log_unknown(msg_type or "?", raw)


# ── Tasks ─────────────────────────────────────────────────────────────────────

async def recv_messages(websocket, verbose: bool, state: Optional[PlayerState] = None, no_fold: bool = False):
    async for message in websocket:
        await handle_message(websocket, message, verbose, state, no_fold)


async def send_actions_fuzz(websocket, verbose: bool, no_fold: bool = False):
    while True:
        await asyncio.sleep(2)
        action = json.dumps(random_action(no_fold))
        if verbose:
            log_verbose("send", action)
        else:
            log_action_sent(action)
        await websocket.send(action)


# ── Entry point ───────────────────────────────────────────────────────────────

async def run(verbose: bool, fuzz: bool, no_fold: bool):
    uri = "ws://localhost:3000/ws"
    state = None if fuzz else PlayerState()
    mode_label = "FUZZ" if fuzz else "SMART"
    try:
        async with websockets.connect(uri) as websocket:
            if verbose:
                print(f"connected  uri={uri}  mode={mode_label}")
            else:
                print(f"{GR}{B}Connected{R}  {DIM}{uri}{R}  mode={B}{mode_label}{R}\n")
            tasks = [recv_messages(websocket, verbose, state, no_fold)]
            if fuzz:
                tasks.append(send_actions_fuzz(websocket, verbose, no_fold))
            await asyncio.gather(*tasks)
    except ConnectionClosedOK as e:
        print(f"connection closed: {e}")
    except Exception as e:
        print(f"{RD}connection failed:{R} {e}")


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description="Mini-poker test client")
    parser.add_argument("--verbose", action="store_true", help="dump raw JSON for all messages")
    parser.add_argument("--fuzz", action="store_true", help="fuzz mode: blast random actions every 2s ignoring turn order")
    parser.add_argument("--no-fold", action="store_true", dest="no_fold", help="never fold")
    args = parser.parse_args()
    asyncio.run(run(args.verbose, args.fuzz, args.no_fold))

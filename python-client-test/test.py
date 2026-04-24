import argparse
import asyncio
import json
import random
import time
from dataclasses import dataclass
from typing import Optional

import websockets
from websockets.exceptions import ConnectionClosedOK

# ── ANSI colours ──────────────────────────────────────────────────────────────
R  = "\033[0m"
B  = "\033[1m"
DIM= "\033[2m"
CY = "\033[36m"
YL = "\033[33m"
GR = "\033[32m"
RD = "\033[31m"
MG = "\033[35m"

# ── Game state (smart mode only) ──────────────────────────────────────────────

@dataclass
class GameState:
    my_id: Optional[str] = None
    step: Optional[str] = None
    bet_base: int = 0
    turn_player_id: Optional[str] = None

# ── Action helpers ─────────────────────────────────────────────────────────────

def choose_smart_action(state: GameState, no_fold: bool = False) -> str:
    if state.bet_base == 0:
        choice = random.choices(["check", "raise"], weights=[65, 35])[0]
    elif no_fold:
        choice = random.choices(["call", "raise"], weights=[65, 35])[0]
    else:
        choice = random.choices(["call", "raise", "fold"], weights=[60, 30, 10])[0]
    if choice == "raise":
        return json.dumps({"type": "raise", "amount": random.randint(10, 200)})
    return json.dumps({"type": choice})

def random_action(no_fold: bool = False) -> str:
    if no_fold:
        choice = random.choices(["call", "check", "raise"], weights=[40, 40, 20])[0]
    else:
        choice = random.choices(["fold", "call", "check", "raise"], weights=[10, 35, 35, 20])[0]
    if choice == "raise":
        return json.dumps({"type": "raise", "amount": random.randint(1, 1000)})
    return json.dumps({"type": choice})

def now_ms() -> int:
    return int(time.time() * 1000)

def short(uuid: str) -> str:
    return uuid[:8] if uuid else "?"

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

def log_player_action(player_id: str, action, bet_base: int):
    if isinstance(action, dict):
        name = next(iter(action)).upper()
        amount = next(iter(action.values()))
        extra = f"  amount={B}{amount}{R}  base={bet_base}"
    else:
        name = str(action).upper()
        extra = f"  base={bet_base}" if name in ("CALL", "CHECK") else ""
    print(f"  {MG}*{R} {B}{name}{R}  player={short(player_id)}{extra}")

def log_unknown(msg_type: str, raw: str):
    print(f"  {DIM}? [{msg_type}] {raw}{R}")

# ── Message handling ──────────────────────────────────────────────────────────

async def handle_message(websocket, raw: str, verbose: bool, state: Optional[GameState] = None, no_fold: bool = False) -> None:
    try:
        data = json.loads(raw)
    except json.JSONDecodeError:
        if verbose:
            log_verbose("recv", raw)
        else:
            print(f"  {DIM}(non-JSON) {raw}{R}")
        return

    msg_type = data.get("type")

    if verbose and msg_type != "pong_ack":
        log_verbose("recv", raw)

    if msg_type == "ping":
        server_ts = data.get("server_ts", 0)
        client_ts = now_ms()
        pong = json.dumps({"type": "pong", "client_ts": client_ts, "server_ts": server_ts})
        await websocket.send(pong)
        if verbose:
            log_verbose("send", pong)

    elif msg_type == "pong_ack":
        c_ts   = data.get("client_ts", 0)
        s_ts   = data.get("server_ts", 0)
        ack_ts = data.get("server_ack_ts", 0)
        rtt    = now_ms() - c_ts
        if verbose:
            log_verbose("recv", raw)
            print(f"    rtt={rtt}ms  server_processing={ack_ts - s_ts}ms")
        else:
            log_rtt(rtt)

    elif msg_type == "session":
        player_id = data.get("player_id", "")
        if state is not None:
            state.my_id = player_id
        if not verbose:
            log_session(player_id)

    elif msg_type == "step":
        new_step = data.get("step", "?")
        if state is not None:
            if new_step in ("blind", "betting_round"):
                state.bet_base = 0
            state.step = new_step
        if not verbose:
            log_step(new_step)

    elif msg_type == "bet_base":
        new_bet_base = data.get("bet_base", 0)
        if state is not None:
            state.bet_base = new_bet_base
        if not verbose:
            log_bet_base(new_bet_base)

    elif msg_type == "active_players":
        players = data.get("players", [])
        if not verbose:
            log_active_players(players)

    elif msg_type == "turn":
        player_id = data.get("player_id", "")
        timeout   = data.get("timeout", 0)
        is_mine   = state is not None and state.my_id is not None and state.my_id == player_id
        if state is not None:
            state.turn_player_id = player_id
        if not verbose:
            log_turn(player_id, timeout, is_mine)
        if is_mine:
            async def think_and_act():
                await asyncio.sleep(random.uniform(0.5, 2.5))
                action = choose_smart_action(state, no_fold)
                if verbose:
                    log_verbose("send", action)
                else:
                    log_action_sent(action)
                await websocket.send(action)
            asyncio.create_task(think_and_act())

    elif msg_type == "player_action":
        raw_bet_base = data.get("bet_base", 0)
        action = data.get("action")
        # Server broadcasts the pre-raise bet_base; add the raise amount to get the real value
        if isinstance(action, dict) and "Raise" in action:
            new_bet_base = raw_bet_base + action["Raise"]
        else:
            new_bet_base = raw_bet_base
        if state is not None:
            state.bet_base = new_bet_base
        if not verbose:
            log_player_action(data.get("player_id", ""), action, new_bet_base)

    elif msg_type == "result":
        if state is not None:
            state.bet_base = 0
        if not verbose:
            log_result(data.get("winners", []), data.get("prizes", []))

    elif msg_type == "warning":
        if not verbose:
            log_warning(data.get("warning_type", {}), data.get("message", ""))

    elif msg_type == "terminate_session":
        if not verbose:
            log_terminate()

    else:
        if not verbose:
            log_unknown(msg_type or "?", raw)

# ── Tasks ─────────────────────────────────────────────────────────────────────

async def recv_messages(websocket, verbose: bool, state: Optional[GameState] = None, no_fold: bool = False):
    async for message in websocket:
        await handle_message(websocket, message, verbose, state, no_fold)

async def send_actions_fuzz(websocket, verbose: bool, no_fold: bool = False):
    while True:
        await asyncio.sleep(2)
        action = random_action(no_fold)
        if verbose:
            log_verbose("send", action)
        else:
            log_action_sent(action)
        await websocket.send(action)

# ── Entry point ───────────────────────────────────────────────────────────────

async def run(verbose: bool, fuzz: bool, no_fold: bool):
    uri = "ws://localhost:3000/ws"
    state = None if fuzz else GameState()
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

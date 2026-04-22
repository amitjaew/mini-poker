import argparse
import asyncio
import json
import random
import time

import websockets
from websockets.exceptions import ConnectionClosedOK

# ── ANSI colours (pretty mode only) ──────────────────────────────────────────
R  = "\033[0m"
B  = "\033[1m"
DIM= "\033[2m"
CY = "\033[36m"
YL = "\033[33m"
GR = "\033[32m"
RD = "\033[31m"
MG = "\033[35m"

ACTIONS = [
    # {"type": "fold"},
    {"type": "call"},
    {"type": "check"},
    {"type": "raise", "amount": 0},
]

def random_action() -> str:
    action = random.choice(ACTIONS)
    if action["type"] == "raise":
        action = {"type": "raise", "amount": random.randint(50, 500)}
    return json.dumps(action)

def now_ms() -> int:
    return int(time.time() * 1000)

def short(uuid: str) -> str:
    return uuid[:8] if uuid else "?"

# ── Logging helpers ───────────────────────────────────────────────────────────

def log_verbose(direction: str, raw: str):
    arrow = f"{CY}>>{R}" if direction == "recv" else f"{YL}<<{R}"
    print(f"{arrow} {raw}")

def log_turn(player_id: str, timeout_ts: int):
    secs = max(0, round((timeout_ts - now_ms()) / 1000))
    print(f"\n{B}{GR}TURN{R}  player={B}{short(player_id)}{R}  expires_in={B}{secs}s{R}")

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

def log_debug(content: str):
    print(f"  {DIM}~ {content}{R}")

def log_rtt(rtt: int):
    colour = GR if rtt < 50 else (YL if rtt < 150 else RD)
    print(f"  {DIM}rtt={colour}{rtt}ms{R}")

def log_terminate():
    print(f"\n{RD}{B}SESSION TERMINATED{R}")

def log_player_action(player_id: str, action, bet_base: int):
    if isinstance(action, dict):
        amount = next(iter(action.values()))
        name = next(iter(action)).upper()
        extra = f"  amount={B}{amount}{R}  base={bet_base}"
    else:
        name = str(action).upper()
        extra = f"  base={bet_base}" if name in ("CALL", "CHECK") else ""
    print(f"  {MG}*{R} {B}{name}{R}  player={short(player_id)}{extra}")

def log_unknown(msg_type: str, raw: str):
    print(f"  {DIM}? [{msg_type}] {raw}{R}")

# ── Message handling ──────────────────────────────────────────────────────────

async def handle_ping(websocket, data: dict, verbose: bool) -> None:
    server_ts = data.get("server_ts", 0)
    client_ts = now_ms()
    pong = json.dumps({"type": "pong", "client_ts": client_ts, "server_ts": server_ts})
    await websocket.send(pong)
    if verbose:
        timer = data.get("data", {}).get("timer", "?")
        log_verbose("recv", json.dumps(data))
        log_verbose("send", pong)

async def handle_message(websocket, raw: str, verbose: bool) -> None:
    try:
        data = json.loads(raw)
    except json.JSONDecodeError:
        if verbose:
            log_verbose("recv", raw)
        else:
            print(f"  {DIM}(non-JSON) {raw}{R}")
        return

    msg_type = data.get("type")

    if verbose:
        if msg_type != "pong_ack":  # pong_ack logged after RTT calc
            log_verbose("recv", raw)

    if msg_type == "ping":
        await handle_ping(websocket, data, verbose)

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

    elif msg_type == "turn":
        if not verbose:
            log_turn(data.get("player_id", ""), data.get("timeout", 0))

    elif msg_type == "result":
        if not verbose:
            log_result(data.get("winners", []), data.get("prizes", []))

    elif msg_type == "warning":
        if not verbose:
            log_warning(data.get("warning_type", {}), data.get("message", ""))

    elif msg_type == "debug":
        if not verbose:
            log_debug(data.get("content", ""))

    elif msg_type == "terminate_session":
        if not verbose:
            log_terminate()

    elif msg_type == "player_action":
        if not verbose:
            log_player_action(
                data.get("player_id", ""),
                data.get("action"),
                data.get("bet_base", 0),
            )

    else:
        if not verbose:
            log_unknown(msg_type or "?", raw)

# ── Tasks ─────────────────────────────────────────────────────────────────────

async def send_actions(websocket, verbose: bool):
    while True:
        await asyncio.sleep(2)
        action = random_action()
        if verbose:
            log_verbose("send", action)
        else:
            log_action_sent(action)
        await websocket.send(action)

async def recv_messages(websocket, verbose: bool):
    async for message in websocket:
        await handle_message(websocket, message, verbose)

# ── Entry point ───────────────────────────────────────────────────────────────

async def run(verbose: bool):
    uri = "ws://localhost:3000/ws"
    try:
        async with websockets.connect(uri) as websocket:
            if verbose:
                print(f"connected  uri={uri}")
            else:
                print(f"{GR}{B}Connected{R}  {DIM}{uri}{R}\n")
            await asyncio.gather(
                recv_messages(websocket, verbose),
                send_actions(websocket, verbose),
            )
    except ConnectionClosedOK as e:
        print(f"connection closed: {e}")
    except Exception as e:
        print(f"{RD}connection failed:{R} {e}")

if __name__ == "__main__":
    parser = argparse.ArgumentParser(description="Mini-poker test client")
    parser.add_argument("--verbose", action="store_true", help="dump raw JSON for all messages")
    args = parser.parse_args()
    asyncio.run(run(args.verbose))

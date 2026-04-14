import asyncio
import json
import random
import time

import websockets
from websockets.exceptions import ConnectionClosedOK

ACTIONS = [
    {"type": "fold"},
    {"type": "call"},
    {"type": "check"},
    {"type": "raise", "amount": random.randint(50, 500)},
]

def random_action():
    action = random.choice(ACTIONS)
    if action["type"] == "raise":
        action = {"type": "raise", "amount": random.randint(50, 500)}
    return json.dumps(action)

def now_ms() -> int:
    return int(time.time() * 1000)

async def send_actions(websocket):
    while True:
        await asyncio.sleep(2)
        action = random_action()
        print(f"Sending action: {action}")
        await websocket.send(action)

async def recv_messages(websocket):
    async for message in websocket:
        try:
            data = json.loads(message)
        except json.JSONDecodeError:
            print(f"Received (non-JSON): {message}")
            continue

        msg_type = data.get("type")

        if msg_type == "ping":
            server_ts = data.get("server_ts", 0)
            client_ts = now_ms()
            pong = json.dumps({"type": "pong", "client_ts": client_ts, "server_ts": server_ts})
            await websocket.send(pong)
            timer = data.get("data", {}).get("timer", "?")
            print(f"Ping (timer={timer}, server_ts={server_ts}) -> Pong sent (client_ts={client_ts})")

        elif msg_type == "pong_ack":
            s_ts   = data.get("server_ts", 0)
            c_ts   = data.get("client_ts", 0)
            ack_ts = data.get("server_ack_ts", 0)
            rtt    = now_ms() - c_ts
            trip   = ack_ts - s_ts   # server processing time
            print(f"PongAck: RTT={rtt}ms, server_processing={trip}ms")

        else:
            print(f"Received: {message}")

async def test_client():
    uri = "ws://localhost:3000/ws"
    try:
        async with websockets.connect(uri) as websocket:
            print("Connected to the server.")
            await asyncio.gather(
                recv_messages(websocket),
                send_actions(websocket),
            )
    except ConnectionClosedOK as e:
        print(f"Server closed connection: {e}")
    except Exception as e:
        print(f"Connection failed: {e}")

asyncio.run(test_client())

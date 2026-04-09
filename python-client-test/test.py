import asyncio
import json
import random

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

async def send_actions(websocket):
    while True:
        await asyncio.sleep(2)
        action = random_action()
        print(f"Sending action: {action}")
        await websocket.send(action)

async def recv_messages(websocket):
    async for message in websocket:
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

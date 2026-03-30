import asyncio

import websockets
from websockets.exceptions import ConnectionClosedOK


async def test_client():
    uri = "ws://localhost:3000/ws"
    try:
        async with websockets.connect(uri) as websocket:
            print("Connected to the server.")
            # Optionally send something
            await websocket.send("Hello from Python client")
            print("Message sent.")

            # Wait for a response (optional)
            while True:
                response = await websocket.recv()
                print(f"Received from server: {response}")
                if response == "close":
                    await websocket.close()
                    return

    except ConnectionClosedOK as e:
        print(f"Server closed connection: {e}")
    except Exception as e:
        print(f"Connection failed: {e}")

asyncio.run(test_client())

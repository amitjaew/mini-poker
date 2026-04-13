# P2P Audio Chat (WebRTC)

## Approach

Use **WebRTC for audio** (peer-to-peer between clients) with the **existing WebSocket as the signaling channel**.  
The server never touches audio — it only relays small signaling messages to establish the connection.

**Why not server-relay (SFU)?**  
For a poker room (2–6 players) there's no need for proximity audio, server-side moderation, or 100-player scale. P2P is simpler and costs nothing in bandwidth. SFUs (like Vivox, used by Roblox) are justified when you need spatial audio, anti-cheat recording, or large lobbies.

---

## Server Changes

Extend the existing message enums to carry signaling through the `PlayerPayload → GameRoomMessage → PlayerMessage` pipeline:

```rust
// player.rs — incoming from client
pub enum PlayerPayload {
    Fold, Check, Call, Raise { amount: u32 },
    RtcOffer  { to: Uuid, sdp: String },
    RtcAnswer { to: Uuid, sdp: String },
    RtcIce    { to: Uuid, candidate: String },
}

// gameroom.rs — route to target player by Uuid
pub enum GameRoomMessage {
    PlayerPayload { payload: PlayerPayload, from: Uuid },
    PlayerJoin    { id: Uuid, sender: mpsc::Sender<PlayerMessage> },
    RtcSignal     { from: Uuid, to: Uuid, signal: RtcSignalKind },
}

// player.rs — outgoing to client
pub enum PlayerMessage {
    // ... existing ...
    RtcSignal { from: Uuid, signal: RtcSignalKind },
}
```

The gameroom handler looks up the `to` player's `sender` and forwards — no audio processing.

---

## Signaling Flow

```
Player A                   Server (GameRoom)               Player B
   |                             |                             |
   |── RtcOffer { to: B } ──────>|                             |
   |                             |── RtcSignal { from: A } ──>|
   |                             |                             |
   |                             |<── RtcAnswer { to: A } ─────|
   |<── RtcSignal { from: B } ───|                             |
   |                             |                             |
   |  [ICE candidates exchanged the same way]                  |
   |                             |                             |
   |<════════ direct audio stream (P2P, no server) ═══════════>|
```

---

## Client Side

```js
const pc = new RTCPeerConnection({
    iceServers: [
        { urls: 'stun:stun.l.google.com:19302' }  // free STUN, covers most cases
    ]
});

// Add local microphone
const stream = await navigator.mediaDevices.getUserMedia({ audio: true });
stream.getTracks().forEach(track => pc.addTrack(track, stream));

// Play incoming audio
pc.ontrack = e => {
    const audio = new Audio();
    audio.srcObject = e.streams[0];
    audio.play();
};

// Forward ICE candidates through your WebSocket
pc.onicecandidate = e => {
    if (e.candidate) ws.send(JSON.stringify({
        type: 'rtc_ice', to: targetPlayerId, candidate: JSON.stringify(e.candidate)
    }));
};

// --- Initiator (Player A) ---
const offer = await pc.createOffer();
await pc.setLocalDescription(offer);
ws.send(JSON.stringify({ type: 'rtc_offer', to: targetPlayerId, sdp: offer.sdp }));

// --- Receiver (Player B) ---
// on receiving offer:
await pc.setRemoteDescription({ type: 'offer', sdp: msg.sdp });
const answer = await pc.createAnswer();
await pc.setLocalDescription(answer);
ws.send(JSON.stringify({ type: 'rtc_answer', to: msg.from, sdp: answer.sdp }));

// both sides: on receiving ice candidate:
await pc.addIceCandidate(JSON.parse(msg.candidate));
```

---

## NAT Traversal: STUN and TURN

### The Problem

Players are behind routers with private IPs. A direct connection needs both sides to know each other's **public-facing address and port** — which the router hides.

### STUN — Discover your public address

A STUN server tells you what your connection looks like from outside your router. Your browser sends this address to the other player via WebSocket (signaling), and the router's NAT table is primed to accept the reply — this is **hole punching**.

```
Browser → STUN: "what's my public address?"
STUN    → Browser: "you look like 81.23.4.5:54321"
Browser → (via WebSocket) → Player B: "connect to me at 81.23.4.5:54321"
```

Works for most home routers. The Google STUN server is free and reliable.

### Symmetric NAT — when hole punching fails

Some routers (corporate networks, some mobile carriers) assign a **different external port per destination**:

```
Browser → STUN server:  you look like 81.23.4.5:54321
Browser → Player B:     you look like 81.23.4.5:99999  ← different!
```

Player B tries `54321` — the router doesn't recognize it. Connection fails.

### TURN — Relay fallback

A TURN server is a middleman both players connect *to*. It forwards packets between them — always works, but all audio goes through your server (bandwidth cost, extra latency).

```
Player A ──> TURN ──> Player B
Player A <── TURN <── Player B
```

### ICE — Automatic negotiation

WebRTC's ICE tries options in order, automatically:

1. Direct LAN (same network)
2. STUN hole punch (different networks, normal NAT)
3. TURN relay (symmetric NAT / strict firewall)

```js
iceServers: [
    { urls: 'stun:stun.l.google.com:19302' },       // hole punch attempt
    { urls: 'turn:your-turn.server.com',             // fallback relay
      username: 'user', credential: 'secret' }
]
```

### Coverage at a glance

| Player network         | Needs       |
|------------------------|-------------|
| Home broadband         | STUN        |
| Mobile data            | STUN / TURN |
| Corporate / strict NAT | TURN        |
| Same LAN               | Neither     |

STUN-only covers ~85–90% of real users. Add a TURN server (e.g. [Coturn](https://github.com/coturn/coturn), self-hosted) only when users report audio failing to connect.

---

## Codec

WebRTC uses **Opus** by default — the same codec used by CS2, Discord, and most VoIP. No configuration needed; the browser handles it automatically.

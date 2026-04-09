# Communications Protocol

WebSocket-based protocol between the game server and clients. All messages are JSON text frames.

---

## Server → Client Messages

Defined as `PlayerMessage` in `player.rs`. Each variant serializes to a JSON object with a `"type"` discriminant field.

### `timer`

Sent every second during an active betting round to all players. Contains the authoritative remaining time for the current player's turn.

```json
{ "type": "timer", "remaining": 27.4 }
```

| Field | Type | Description |
|---|---|---|
| `remaining` | `f32` | Seconds remaining, decremented by actual elapsed time |

> See `FURTHER_CONSIDERATION.md` for notes on client-side interpolation between ticks and the clock-offset approach as an alternative.

---

### `turn_start`

Sent when it becomes a player's turn to act. Tells all clients whose turn it is and when it expires.

```json
{ "type": "turn_start", "player_id": "uuid-string", "expires_at": 1712345678123 }
```

| Field | Type | Description |
|---|---|---|
| `player_id` | `string` (UUID) | The player whose turn it is |
| `expires_at` | `u64` | Unix timestamp in milliseconds when the turn auto-expires |

---

### `action_error`

Sent to a specific player when their submitted action is invalid (e.g. check when the bet base has changed).

```json
{ "type": "action_error", "reason": "cannot check after raise" }
```

| Field | Type | Description |
|---|---|---|
| `reason` | `string` | Human-readable description of why the action was rejected |

---

### `pong`

Response to a client `ping`. Used for clock offset / latency estimation. See [Latency Calibration](#latency-calibration) below.

```json
{
  "type": "pong",
  "client_ts": 1712345678000,
  "server_ts": 1712345678042,
  "server_ack_ts": 1712345678043
}
```

| Field | Type | Description |
|---|---|---|
| `client_ts` | `u64` | Echoed from the client's ping |
| `server_ts` | `u64` | Unix ms when the server received the ping |
| `server_ack_ts` | `u64` | Unix ms when the server sent this pong |

---

### `terminate`

Sent when the server closes the session. Followed immediately by a WebSocket close frame (code `1000`).

```json
{ "type": "terminate" }
```

---

## Client → Server Messages

Parsed from JSON in `player_socket_recv_loop` (`player.rs`) using `serde_json`. Deserialized into `PlayerGameAction` (`gameroom.rs`) and forwarded as `GameRoomMessage::PlayerAction`.

Tagged with `"type"` using `#[serde(tag = "type", rename_all = "snake_case")]`.

### `fold`

Player forfeits their hand for this round.

```json
{ "type": "fold" }
```

---

### `call`

Player matches the current bet base.

```json
{ "type": "call" }
```

---

### `check`

Player passes without betting. Only valid when the player's current bet equals the bet base (i.e. no raise has occurred).

```json
{ "type": "check" }
```

Server responds with `action_error` if a check is invalid.

---

### `raise`

Player increases the bet base by `amount`.

```json
{ "type": "raise", "amount": 200 }
```

| Field | Type | Description |
|---|---|---|
| `amount` | `u32` | Amount added on top of the current bet base |

---

### `ping`

Client-initiated latency probe. See [Latency Calibration](#latency-calibration) below.

```json
{ "type": "ping", "client_ts": 1712345678000 }
```

| Field | Type | Description |
|---|---|---|
| `client_ts` | `u64` | Unix ms when the client sent this message |

---

## Latency Calibration

Used to estimate the clock offset between client and server so that `expires_at` timestamps in `turn_start` can be correctly interpreted by the client.

### Exchange

```
Client                          Server
  |                               |
  |-- ping { client_ts: T0 } ---> |  (server records T1)
  |                               |  (server records T2, sends pong)
  | <-- pong { client_ts: T0,  ---|
  |            server_ts: T1,     |
  |            server_ack_ts: T2} |
  | (client records T3)           |
```

### Clock Offset Formula

The NTP-style offset estimates how far the server clock is ahead of the client:

```
rtt          = T3 - T0
clock_offset = T1 - T0 - rtt / 2
             = T1 - T0 - (T3 - T0) / 2
```

A positive `clock_offset` means the server clock is ahead of the client.

### Correcting `expires_at`

```js
const corrected_now  = Date.now() + clock_offset;
const remaining_ms   = msg.expires_at - corrected_now;
```

### Latency Asymmetry Estimate (diagnostic)

From `FURTHER_CONSIDERATION.md`:

```
latency_estimate = (T2 - T1) - rtt * 0.5
```

This estimates asymmetric delay and server processing overhead. Useful for diagnostics but not required for the timer display.

### Calibration Procedure

1. Send 5 pings spaced ~200ms apart on connection (and on reconnect).
2. Compute `clock_offset` for each sample.
3. Sort, drop the min and max, average the rest.
4. Store as `clock_offset` for the session duration.
5. Optionally recalibrate every ~60s in the background (clock drift is slow).

---

## Implementation Checklist

### Server (`webserver/src/`)

- [ ] `Cargo.toml` — add `serde = { version = "1", features = ["derive"] }` and `serde_json = "1"`
- [ ] `player.rs` — replace `PlayerMessage::GameRoomPayload { content: String }` with typed variants: `Timer`, `TurnStart`, `ActionError`, `Pong`, `TerminateSession`
- [ ] `player.rs` — serialize each variant to JSON in `player_message_recv_loop`
- [ ] `player.rs` — parse inbound JSON into `PlayerGameAction` in `player_socket_recv_loop`, send `GameRoomMessage::PlayerAction`
- [ ] `player.rs` — handle `ping` in `player_socket_recv_loop`, respond with `pong` immediately
- [ ] `gameroom.rs` — derive `Deserialize` on `PlayerGameAction`, change `Raise(u32)` to `Raise { amount: u32 }`
- [ ] `gameroom.rs` — add `GameRoomMessage::PlayerAction { action: PlayerGameAction, from: uuid::Uuid }` variant, remove `PlayerPayload`
- [ ] `gameroom.rs` — update `handle_gameroom_message` to match `PlayerAction` and assign `player.state.action`
- [ ] `gameroom.rs` — replace `PlayerMessage::GameRoomPayload` send sites with typed variants

### Client

- [ ] On connect: run calibration ping sequence, store `clock_offset`
- [ ] On `turn_start`: compute `remaining_ms = expires_at - (Date.now() + clock_offset)`, start `requestAnimationFrame` countdown
- [ ] On `timer`: snap local display to `remaining` value (do not accumulate client-side drift)
- [ ] On `action_error`: display `reason` to the acting player
- [ ] On `terminate`: clean up session state

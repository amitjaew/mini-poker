# Further Considerations

## Timer Client Notifications

**Current approach** (`gameroom.rs:216-277`): server sends `timer: {turn_timer}` (remaining seconds as `f32`) at the start of each 1-second tick, decremented by actual elapsed time.

**1s server ticks + client-side counters is adequate for poker** — turn timers are 30-60s, so <1s drift is imperceptible.

### What the client must do correctly

- On each `timer:` message → **snap** local counter to the server value (don't accumulate offset)
- Between ticks → count down smoothly (e.g. `requestAnimationFrame`)
- Never trust its own counter past ~1.2s without a server update

### Potential issue with current approach

`turn_timer` is sent *before* the 1s sleep, so on a slow network the client receives a value already slightly stale. At 200ms latency, client briefly shows `30.0` when true remaining is `29.8`.

### More robust alternative: send expiry deadline

```
timer: {unix_timestamp_when_turn_expires}
```

Client computes `deadline - now` locally — immune to network jitter, no client-side counters needed. Requires clock sync between client and server.

For a small/local game the current tick approach is fine, as long as the client snaps to the server value on each update rather than running an independent countdown.

---

## Network Latency Estimation & Clock Compensation

When sending a deadline timestamp instead of remaining seconds, the client needs to know how far its clock differs from the server's. This formula estimates one-way network latency without requiring synchronized clocks.

### Variables

| Symbol | Meaning |
|---|---|
| `client_ts` | Timestamp when client sends a ping |
| `server_ts` | Timestamp when server receives that ping |
| `server_ack_ts` | Timestamp when server sends its response |
| `client_ack_ts` | Timestamp when client receives the response |

### The Formula

```
latency_estimate = (server_ack_ts - server_ts) - ((client_ack_ts - client_ts) * 0.5)
```

### Math Reasoning

**Term 1: `(server_ack_ts - server_ts)`**

This is the time the message spends on the server side — processing plus outbound transmission startup:

```
T_server = processing_time + T_(server→client)
```

**Term 2: `(client_ack_ts - client_ts)`**

This is the full round-trip time (RTT) as observed by the client:

```
RTT = T_(client→server) + processing_time + T_(server→client)
```

**Halving RTT — the symmetry assumption:**

The formula assumes symmetric network paths:

```
T_(client→server) ≈ T_(server→client)
```

So:

```
RTT / 2 ≈ one_way_delay + processing_time / 2
```

**Substituting both terms:**

```
latency_estimate
  = (processing_time + T_(server→client))
  - (processing_time + T_(client→server) + T_(server→client)) / 2

  = processing_time/2 + (T_(server→client) - T_(client→server)) / 2
```

If the network is symmetric (`T_(client→server) = T_(server→client)`), this collapses to:

```
latency_estimate ≈ processing_time / 2
```

So the formula's core insight is: **subtract half the round-trip from the server-observed delay to isolate the asymmetric component**. It does not need synchronized clocks because both client timestamps are on the same clock, and both server timestamps are on the same clock.

### Clock Offset Estimation

The related quantity — **how far the client clock is ahead of the server clock** — is:

```
clock_offset = server_ts - client_ts - RTT / 2
             = server_ts - client_ts - (client_ack_ts - client_ts) / 2
```

This is the NTP clock offset formula. A positive value means the server clock is ahead of the client.

### Limitations

- **Symmetry assumption is often wrong.** Uplink and downlink can differ significantly (e.g. mobile networks, asymmetric ISP plans). In that case the estimate is biased.
- **Server processing time pollutes the estimate.** If the server takes variable time between receiving and responding, it adds noise. Minimize this by having the server respond immediately (echo the ping before doing any work).
- **Single sample is noisy.** Average over several measurements and discard outliers.

---

### Integration into mini-poker

The goal is to let the client accurately display a countdown from a server-issued deadline, without needing synchronized clocks.

#### Server side

Add a ping/pong message pair. The server records receipt and send timestamps, then includes them in the pong:

```rust
// New message variant in PlayerMessage or a dedicated WS message type
// Client sends: { "type": "ping", "client_ts": 1712345678123 }
// Server replies: { "type": "pong", "client_ts": <echo>, "server_ts": <recv>, "server_ack_ts": <now> }
```

In the WebSocket handler (around `player.rs:105`), intercept ping messages:

```rust
"ping" => {
    let server_ts = unix_ms_now();           // time of receipt
    let server_ack_ts = unix_ms_now();       // time of response (minimize gap)
    let pong = format!(
        r#"{{"type":"pong","client_ts":{},"server_ts":{},"server_ack_ts":{}}}"#,
        client_ts, server_ts, server_ack_ts
    );
    socket.send(Message::Text(pong.into())).await?;
}
```

When a player's turn starts, send an absolute expiry timestamp instead of remaining seconds:

```rust
// In handle_step_betting_round, instead of:
//   format!("timer: {}", turn_timer)
// Send:
let expires_at = unix_ms_now() + (turn_duration_secs * 1000) as u64;
format!(r#"{{"type":"turn_start","expires_at":{},"player_id":"{}"}}"#, expires_at, player_id)
```

#### Client side

On connection (or reconnection), run the ping exchange and compute the clock offset:

```js
function measureClockOffset() {
    const client_ts = Date.now();
    ws.send(JSON.stringify({ type: "ping", client_ts }));

    ws.onmessage = (event) => {
        const msg = JSON.parse(event.data);
        if (msg.type !== "pong") return;

        const client_ack_ts = Date.now();
        const rtt = client_ack_ts - msg.client_ts;

        // Clock offset: how many ms the server clock is ahead of client
        const clock_offset = msg.server_ts - msg.client_ts - rtt / 2;

        // Latency asymmetry estimate (optional diagnostic)
        const latency_est = (msg.server_ack_ts - msg.server_ts) - rtt * 0.5;

        store.clockOffset = clock_offset; // persist for turn timer use
    };
}
```

Average over 3-5 samples and discard outliers (e.g. anything more than 2× the median RTT):

```js
async function calibrateOffset(samples = 5) {
    const offsets = [];
    for (let i = 0; i < samples; i++) {
        offsets.push(await measureClockOffset());
        await sleep(200);
    }
    offsets.sort((a, b) => a - b);
    const trimmed = offsets.slice(1, -1);                      // drop min and max
    store.clockOffset = trimmed.reduce((s, v) => s + v, 0) / trimmed.length;
}
```

When a `turn_start` message arrives, compute remaining time using the corrected clock:

```js
ws.on("turn_start", (msg) => {
    const correctedNow = Date.now() + store.clockOffset;
    const remainingMs = msg.expires_at - correctedNow;
    startCountdown(remainingMs);
});

function startCountdown(remainingMs) {
    const deadline = Date.now() + remainingMs;  // local deadline, no offset needed hereafter
    function tick() {
        const left = Math.max(0, deadline - Date.now());
        renderTimer(left / 1000);  // seconds
        if (left > 0) requestAnimationFrame(tick);
    }
    requestAnimationFrame(tick);
}
```

#### When to recalibrate

- On initial WebSocket connection
- On reconnection after a drop
- Optionally every ~60s in the background (clock drift is slow, ~1-2ms/min at worst)

#### Practical threshold for mini-poker

For a 60-second turn timer, a clock offset error of even 500ms is only 0.8% of the total duration — well within acceptable UX. Recalibrating once on connect is sufficient. The multi-sample averaging and outlier removal matter more on mobile or high-jitter connections.

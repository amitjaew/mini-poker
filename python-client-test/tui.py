from __future__ import annotations

import argparse
import asyncio
import datetime
from typing import Optional

import agent as agent_module
from textual import work
from textual.app import App, ComposeResult
from textual.containers import Horizontal, Vertical
from textual.widgets import (
    Button,
    Checkbox,
    DataTable,
    Footer,
    Header,
    Input,
    Label,
    RichLog,
    Rule,
    Select,
)


class CheckMark(Checkbox):
    BUTTON_INNER = "✓"
from messages import (
    CommunityCardsUpdated,
    GameEvent,
    PlayerConnected,
    PlayerFundsChanged,
    PlayerStateUpdated,
    SessionStopped,
)

TABLE_COLUMNS = [
    ("#",        "num"),
    ("Short ID", "short_id"),
    ("Funds",    "funds"),
    ("Status",   "status"),
    ("Action",   "action"),
    ("Bet",      "bet"),
    ("Cards",    "cards"),
    ("Latency",  "latency"),
]

N_OPTIONS = [(str(i), i) for i in range(1, 11)]

DEFAULT_URL = "ws://localhost:3000/ws"
DEFAULT_N   = 2


class PokerTUI(App):
    CSS = """
    #controls {
        height: auto;
        background: $panel;
        padding: 0 1;
    }
    #inputs-row {
        height: 3;
        align: right middle;
        margin-bottom: 1;
        margin-top: 1;
    }
    #inputs-row Label {
        width: auto;
        height: 3;
        content-align: center middle;
        margin-right: 1;
    }
    #url-input {
        width: 32;
        margin-right: 3;
    }
    #n-players {
        width: 16;
    }
    #actions-row {
        height: 3;
        align: right middle;
        margin-bottom: 1;
    }
    #actions-row Button {
        min-width: 10;
        height: 3;
        margin-right: 1;
    }
    #actions-row Checkbox {
        height: 3;
    }
    DataTable {
        height: auto;
        max-height: 20;
    }
    #community-row {
        height: 1;
        background: $panel;
        padding: 0 1;
    }
    Rule {
        margin: 0;
        color: $accent;
    }
    #log-label {
        height: 1;
        background: $panel;
        padding: 0 1;
    }
    RichLog {
        height: 1fr;
        background: $surface;
        padding: 0 1;
    }
    """

    BINDINGS = [
        ("q", "quit", "Quit"),
    ]

    def __init__(self, default_url: str = DEFAULT_URL, default_n: int = DEFAULT_N) -> None:
        super().__init__()
        self._url: str = default_url
        self._n_players: int = default_n
        self._no_fold: bool = False
        self._running: bool = False
        self._session_worker = None

    def compose(self) -> ComposeResult:
        yield Header(show_clock=True)
        with Vertical():
            with Vertical(id="controls"):
                with Horizontal(id="inputs-row"):
                    yield Label("URL:")
                    yield Input(id="url-input", value=self._url,
                                placeholder="ws://host:port/path")
                    yield Label("Players:")
                    yield Select(
                        options=N_OPTIONS,
                        id="n-players",
                        value=self._n_players,
                        allow_blank=False,
                    )
                with Horizontal(id="actions-row"):
                    yield Button("▶ Start", id="btn-start", variant="success")
                    yield Button("■ Stop",  id="btn-stop",  variant="error",
                                 disabled=True)
                    yield CheckMark("No fold", id="no-fold")
            yield DataTable(id="player-table", show_cursor=False,
                            zebra_stripes=True)
            yield Label(" Community: —", id="community-row")
            yield Rule()
            yield Label(" Event Log", id="log-label")
            yield RichLog(id="event-log", markup=True, highlight=False,
                          max_lines=500, wrap=True, auto_scroll=True)
        yield Footer()

    def on_mount(self) -> None:
        table = self.query_one(DataTable)
        for label, key in TABLE_COLUMNS:
            table.add_column(label, key=key)

    # ── Control event handlers ────────────────────────────────────────────────

    def on_input_changed(self, event: Input.Changed) -> None:
        if event.input.id == "url-input":
            self._url = event.value

    def on_select_changed(self, event: Select.Changed) -> None:
        if event.select.id == "n-players" and event.value is not Select.BLANK:
            self._n_players = int(event.value)

    def on_checkbox_changed(self, event: Checkbox.Changed) -> None:
        if event.checkbox.id == "no-fold":
            self._no_fold = event.value

    def on_button_pressed(self, event: Button.Pressed) -> None:
        if event.button.id == "btn-start":
            self._start_session()
        elif event.button.id == "btn-stop":
            if self._running:
                self._stop_session()
            else:
                self._start_session()

    # ── Session lifecycle ─────────────────────────────────────────────────────

    def _start_session(self) -> None:
        self._running = True
        stop_btn = self.query_one("#btn-stop", Button)
        stop_btn.disabled = False
        stop_btn.label = "■ Stop"
        self.query_one("#btn-start", Button).disabled = True
        self.query_one("#url-input", Input).disabled  = True
        self.query_one("#n-players", Select).disabled = True

        table = self.query_one(DataTable)
        table.clear()
        self.query_one(RichLog).clear()
        self.query_one("#community-row", Label).update(" Community: —")

        n = self._n_players
        session_id = datetime.datetime.now().strftime("%Y%m%d_%H%M%S")
        for i in range(n):
            table.add_row(
                str(i + 1), "...", "~1000", "Waiting", "None", "0", "", "-",
                key=str(i),
            )

        self.query_one(RichLog).write(
            f"[bold green]Starting {n} agent(s) → {self._url}[/]"
        )
        self._session_worker = self._run_all_agents(self._url, n, self._no_fold, session_id)

    def _stop_session(self) -> None:
        if self._session_worker is not None:
            self._session_worker.cancel()
            self._session_worker = None
        self._on_session_ended()

    def _on_session_ended(self) -> None:
        self._running = False
        self.query_one("#btn-start", Button).disabled = False
        stop_btn = self.query_one("#btn-stop", Button)
        stop_btn.disabled = False
        stop_btn.label = "↺ Renew"
        self.query_one("#url-input", Input).disabled  = False
        self.query_one("#n-players", Select).disabled = False
        self.query_one(RichLog).write("[dim]--- session ended ---[/dim]")

    # ── Background worker ─────────────────────────────────────────────────────

    @work(exclusive=True, exit_on_error=False)
    async def _run_all_agents(self, url: str, n: int, no_fold: bool, session_id: str) -> None:
        try:
            await asyncio.gather(
                *[agent_module.run_agent(i, url, self, no_fold=no_fold, session_id=session_id) for i in range(n)],
                return_exceptions=True,
            )
            self.post_message(SessionStopped(-1, "all agents done"))
        except asyncio.CancelledError:
            pass

    # ── Message handlers ──────────────────────────────────────────────────────

    def on_player_connected(self, msg: PlayerConnected) -> None:
        try:
            self.query_one(DataTable).update_cell(
                str(msg.player_index), "short_id", msg.player_id[:8],
            )
        except Exception:
            pass

    def on_player_state_updated(self, msg: PlayerStateUpdated) -> None:
        col_map = {
            "status":  "status",
            "action":  "action",
            "bet":     "bet",
            "cards":   "cards",
            "latency": "latency",
            "funds":   "funds",
        }
        col_key = col_map.get(msg.field)
        if col_key is None:
            return
        try:
            self.query_one(DataTable).update_cell(
                str(msg.player_index), col_key, str(msg.value),
            )
        except Exception:
            pass

    def on_player_funds_changed(self, msg: PlayerFundsChanged) -> None:
        try:
            self.query_one(DataTable).update_cell(
                str(msg.player_index), "funds", f"~{msg.new_funds}",
            )
        except Exception:
            pass

    def on_community_cards_updated(self, msg: CommunityCardsUpdated) -> None:
        text = f" Community: {msg.text}" if msg.text else " Community: —"
        self.query_one("#community-row", Label).update(text)

    def on_game_event(self, msg: GameEvent) -> None:
        ts = datetime.datetime.now().strftime("%H:%M:%S")
        self.query_one(RichLog).write(f"[dim]{ts}[/dim] {msg.text}")

    def on_session_stopped(self, msg: SessionStopped) -> None:
        if msg.player_index == -1:
            if self._running:
                self._on_session_ended()
        else:
            try:
                self.query_one(DataTable).update_cell(
                    str(msg.player_index), "status", "Disconnected",
                )
            except Exception:
                pass

    # ── Actions ───────────────────────────────────────────────────────────────

    def action_quit(self) -> None:
        if self._running:
            self._stop_session()
        self.exit()


if __name__ == "__main__":
    parser = argparse.ArgumentParser(description="Mini-Poker multi-agent TUI")
    parser.add_argument("--url",     default=DEFAULT_URL,
                        help="WebSocket server URL")
    parser.add_argument("--players", type=int, default=DEFAULT_N,
                        help="Number of agents to spawn (1-10)")
    args = parser.parse_args()
    app = PokerTUI(default_url=args.url, default_n=args.players)
    app.run()

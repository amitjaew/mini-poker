from __future__ import annotations

from dataclasses import dataclass, field
from typing import Optional


@dataclass
class PlayerState:
    my_id: Optional[str] = None
    step: Optional[str] = None
    bet_base: int = 0
    turn_player_id: Optional[str] = None
    funds: int = 1000
    status: str = "Waiting"
    last_action: str = "None"
    current_bet: int = 0
    latency_ms: int = 0
    hole_cards_text: str = ""
    community_cards: list = field(default_factory=list)

    # ── Fund bookkeeping ──────────────────────────────────────────────────────
    def apply_blind(self, amount: int) -> None:
        """Deduct a posted blind (small or big) from funds."""
        self.current_bet = amount
        self.funds -= amount

    def apply_action(self, action_type: str, amount: int = 0) -> None:
        """Update funds/current_bet/bet_base for an action this player takes.

        Mirrors the server's Call/Raise accounting (incl. all-in Call) so the
        local balance never goes negative and stays in sync with the server.
        """
        self.last_action = action_type
        if action_type == "CALL":
            delta = max(0, self.bet_base - self.current_bet)
            if self.funds >= delta:
                self.funds -= delta
                self.current_bet = self.bet_base
            elif self.funds > 0:
                self.current_bet += self.funds
                self.funds = 0
        elif action_type == "RAISE":
            new_bet = self.bet_base + amount
            delta = max(0, new_bet - self.current_bet)
            if self.funds >= delta:
                self.funds -= delta
                self.current_bet = new_bet
                self.bet_base = new_bet

    def apply_game_state(self, players: list, step: Optional[str] = None) -> None:
        """Sync authoritative funds/bet from the join-time `game_state` snapshot."""
        if step is not None:
            self.step = step
        for p in players:
            if p.get("id") == self.my_id:
                self.funds = int(p.get("funds", self.funds))
                self.current_bet = int(p.get("bet", 0))
                break

    def apply_prize(self, prize: int) -> None:
        self.funds += prize

    def apply_refund(self, amount: int) -> None:
        self.funds += amount

    def reset_hand(self) -> None:
        self.bet_base = 0
        self.current_bet = 0
        self.hole_cards_text = ""
        self.community_cards = []

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
    def apply_action(self, action_type: str, amount: int = 0) -> None:
        """Update funds/current_bet/bet_base for an action this player takes."""
        self.last_action = action_type
        if action_type == "CALL":
            delta = max(0, self.bet_base - self.current_bet)
            self.current_bet = self.bet_base
            self.funds -= delta
        elif action_type == "RAISE":
            new_bet = self.bet_base + amount
            delta = max(0, new_bet - self.current_bet)
            self.current_bet = new_bet
            self.funds -= delta
            self.bet_base = new_bet

    def apply_prize(self, prize: int) -> None:
        self.funds += prize

    def reset_hand(self) -> None:
        self.bet_base = 0
        self.current_bet = 0
        self.hole_cards_text = ""
        self.community_cards = []

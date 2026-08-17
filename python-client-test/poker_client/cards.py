from __future__ import annotations

# Rank::Two=0 … Rank::Ace=12
RANK_NAMES = ["2", "3", "4", "5", "6", "7", "8", "9", "T", "J", "Q", "K", "A"]
SUIT_SYMBOLS = {"c": "♣", "d": "♦", "h": "♥", "s": "♠"}


def fmt_card(suit: str, rank: int) -> str:
    r = RANK_NAMES[rank] if 0 <= rank < 13 else "?"
    s = SUIT_SYMBOLS.get(suit, suit)
    return f"{r}{s}"


def fmt_cards(cards: list[dict]) -> str:
    return " ".join(fmt_card(c.get("suit", "?"), c.get("rank", 0)) for c in cards)

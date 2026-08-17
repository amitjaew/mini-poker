from __future__ import annotations

import random


def choose_smart_action(state, no_fold: bool = False) -> dict:
    """Pick a legal action consistent with the player's bet vs. the bet base.

    Check is only available when the player has already matched the current bet
    (``current_bet == bet_base``); otherwise they must call, raise or fold.
    """
    can_check = state.current_bet == state.bet_base
    if can_check:
        choice = random.choices(["check", "raise"], weights=[65, 35])[0]
    elif no_fold:
        choice = random.choices(["call", "raise"], weights=[65, 35])[0]
    else:
        choice = random.choices(["call", "raise", "fold"], weights=[60, 30, 10])[0]

    if choice == "raise":
        return {"type": "raise", "amount": random.randint(10, 200)}
    return {"type": choice}


def random_action(no_fold: bool = False) -> dict:
    """Random (fuzz) action; deliberately ignores turn/bet coherence."""
    if no_fold:
        choice = random.choices(["call", "check", "raise"], weights=[40, 40, 20])[0]
    else:
        choice = random.choices(
            ["fold", "call", "check", "raise"], weights=[10, 35, 35, 20]
        )[0]

    if choice == "raise":
        return {"type": "raise", "amount": random.randint(1, 1000)}
    return {"type": choice}

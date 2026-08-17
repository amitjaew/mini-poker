from __future__ import annotations

import random


def choose_smart_action(state, no_fold: bool = False) -> dict:
    """Pick a legal, funds-aware action consistent with the bet base.

    Check is only available when the player has already matched the current bet
    (``current_bet == bet_base``). Raises are capped so they never exceed the
    player's funds, and the player folds when they cannot afford the call.
    """
    to_call = max(0, state.bet_base - state.current_bet)
    can_check = state.current_bet == state.bet_base
    max_raise = state.funds - to_call

    if can_check:
        if max_raise <= 0:
            return {"type": "check"}
        choice = random.choices(["check", "raise"], weights=[65, 35])[0]
    elif max_raise < 0:
        return {"type": "fold"} if not no_fold else {"type": "call"}
    elif max_raise == 0:
        if no_fold:
            return {"type": "call"}
        choice = random.choices(["call", "fold"], weights=[90, 10])[0]
    else:
        if no_fold:
            choice = random.choices(["call", "raise"], weights=[65, 35])[0]
        else:
            choice = random.choices(["call", "raise", "fold"], weights=[60, 30, 10])[0]

    if choice == "raise":
        amount = min(random.randint(10, 200), max(1, max_raise))
        return {"type": "raise", "amount": amount}
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

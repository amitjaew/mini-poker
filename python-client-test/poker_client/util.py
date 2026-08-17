from __future__ import annotations

import time


def now_ms() -> int:
    return int(time.time() * 1000)


def short(uuid: str) -> str:
    return uuid[:8] if uuid else "?"

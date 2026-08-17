from __future__ import annotations

import datetime
import json
import os
from typing import TextIO


class SessionLogger:
    """Per-player session log that dumps every TX/RX message to ``logs/``."""

    def __init__(self, player_index: int, session_id: str = "") -> None:
        package_dir = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
        log_dir = os.path.join(package_dir, "logs")
        os.makedirs(log_dir, exist_ok=True)

        sid = session_id or datetime.datetime.now().strftime("%Y%m%d_%H%M%S")
        path = os.path.join(log_dir, f"session_{sid}_P{player_index + 1}.log")
        self._file: TextIO = open(path, "w", encoding="utf-8")

    def dump(self, direction: str, payload) -> None:
        ts = datetime.datetime.now().isoformat(timespec="milliseconds")
        self._file.write(f"{ts} {direction} {json.dumps(payload)}\n")
        self._file.flush()

    def close(self) -> None:
        self._file.close()

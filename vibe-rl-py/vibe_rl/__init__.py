"""VibeCody RL-OS sidecar — see ../README.md."""

from pathlib import Path as _Path

__version__ = (_Path(__file__).resolve().parent.parent / "VERSION").read_text().strip()

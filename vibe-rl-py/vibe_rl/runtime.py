"""Run lifecycle helpers — signal handling, stop semantics, error reporting.

The daemon talks to us via signals:
- `SIGTERM`  → graceful stop. Finish current update, write final checkpoint, exit 0.
- `SIGINT`   → same as SIGTERM. Useful when the user hits Ctrl-C in a tty.
- `SIGKILL`  → cannot be caught; daemon uses this for `cancel`.

We expose a `should_stop()` poll that algorithms call between updates. When
true, the algorithm finishes its current step, writes a checkpoint, and
returns — `cli.py` then emits the `finished` JSON-Line.
"""

from __future__ import annotations

import signal
import sys
import traceback
from contextlib import contextmanager
from typing import Iterator

_STOP_REQUESTED = False


def install_signal_handlers() -> None:
    def _handler(signum: int, _frame) -> None:  # type: ignore[no-untyped-def]
        global _STOP_REQUESTED
        _STOP_REQUESTED = True

    signal.signal(signal.SIGTERM, _handler)
    try:
        signal.signal(signal.SIGINT, _handler)
    except ValueError:
        # not in main thread — that's fine
        pass


def should_stop() -> bool:
    return _STOP_REQUESTED


def reset_stop() -> None:
    """For tests."""
    global _STOP_REQUESTED
    _STOP_REQUESTED = False


@contextmanager
def report_errors(streamer) -> Iterator[None]:  # type: ignore[no-untyped-def]
    """Emit a `finished` line with the exception message if anything raises.

    Without this wrapper, an uncaught exception would surface as a
    silent process exit and the daemon would only see the run's pipe
    close. This way, the daemon gets a structured error string to
    persist on the run row.
    """
    try:
        yield
    except KeyboardInterrupt:
        streamer.finished(reason="cancelled")
        sys.exit(130)
    except SystemExit:
        raise
    except Exception as e:  # noqa: BLE001 — we genuinely want any exception
        streamer.finished(
            reason="error",
            error=f"{type(e).__name__}: {e}\n{traceback.format_exc(limit=20)}",
        )
        sys.exit(1)

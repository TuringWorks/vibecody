#!/usr/bin/env python3
"""Resident Kokoro-82M TTS sidecar — same wire protocol as tts.swift.

Speaks the identical stdin/stdout contract as the macOS AVSpeechSynthesizer
sidecar, so the daemon needs no new plumbing: point `[voice] tts_sidecar` at an
interpreter and this script, and the existing `Sidecar` transport carries it.

    stdin   one JSON object per line — {"text":…,"voice":…,"rate":…}
                                     | {"cmd":"cancel"}
    stdout  "AUR" u32(bytes) u32(rate) f32le[]   audio
            "END" u32(0)                         end of utterance

`AUR` rather than `AUD` because Kokoro produces 24 kHz and the Swift sidecar
22.05 kHz. A wrong sample rate does not fail — it plays at the wrong pitch —
so the rate travels with the samples instead of being assumed by the reader.

**Clause splitting is the reason this is usable.** Kokoro is non-autoregressive:
a sentence is produced in one pass, so first audio is the whole sentence's
synthesis time. Measured on an M-series Mac, a full sentence takes 386-416 ms;
split at commas and the first clause goes out in 165-228 ms while the rest
synthesises behind it. Total synthesis rises ~15%, which costs nothing because
the real-time factor is ~0.1 — playback never catches up with generation.

The prosody cost is real and is the tradeoff: every clause gets sentence-final
intonation. `{"cmd":"clauses","on":false}` turns it off.
"""
import json
import re
import struct
import sys
import threading
import queue

RATE = 24_000
DEFAULT_VOICE = "af_heart"
# Below this many characters a clause is not worth a separate synthesis pass:
# the fixed overhead per pass outweighs anything gained by starting sooner.
MIN_CLAUSE = 22


def log(msg):
    """Diagnostics go to stderr. stdout is a binary frame stream and anything
    printed there corrupts the next frame the daemon reads."""
    print(msg, file=sys.stderr, flush=True)


def emit(tag: bytes, payload: bytes) -> None:
    out = sys.stdout.buffer
    out.write(tag)
    out.write(struct.pack("<I", len(payload)))
    out.write(payload)
    out.flush()


def emit_audio(samples, rate: int = RATE) -> None:
    import numpy as np

    pcm = np.clip(np.asarray(samples, dtype=np.float32).reshape(-1), -1.0, 1.0)
    emit(b"AUR", struct.pack("<I", rate) + pcm.tobytes())


def clauses(text: str, minlen: int = MIN_CLAUSE):
    """Split after commas, keeping every part long enough to be worth a pass.

    A trailing fragment is glued onto the previous clause rather than sent on
    its own — "…, and it." as a separate pass is three round trips of overhead
    for a word and a half.
    """
    parts, buf = [], ""
    for piece in re.split(r"(?<=[,;:])\s+", text):
        buf = f"{buf} {piece}".strip() if buf else piece
        if len(buf) >= minlen:
            parts.append(buf)
            buf = ""
    if buf:
        if parts:
            parts[-1] += " " + buf
        else:
            parts.append(buf)
    return parts or [text]


class Engine:
    def __init__(self):
        from mlx_audio.tts.utils import load_model

        self.model = load_model("prince-canuma/Kokoro-82M")
        self.split = True
        self.q = queue.Queue()
        # Bumped on cancel. Work tagged with a stale generation is dropped
        # without emitting, so a barge-in cannot be spoken over by the reply it
        # interrupted.
        self.gen = 0
        self.lock = threading.Lock()
        threading.Thread(target=self._worker, daemon=True).start()

    def warm(self):
        """The first synthesis in a process builds the lazy graph. Pay it before
        anyone is listening — the daemon's spawn sends 'ok' and drains, and a
        cold first turn would otherwise charge the user for it."""
        list(self.model.generate(text="ok", voice=DEFAULT_VOICE, speed=1.0, lang_code="a"))

    def enqueue(self, text, voice, speed):
        with self.lock:
            self.q.put((self.gen, text, voice, speed))

    def cancel(self):
        with self.lock:
            self.gen += 1
            try:
                while True:
                    self.q.get_nowait()
            except queue.Empty:
                pass
        # The interrupted utterance still owes the daemon a terminator, or its
        # reader waits out the full timeout before moving on.
        emit(b"END", b"")

    def _stale(self, g) -> bool:
        with self.lock:
            return g != self.gen

    def _worker(self):
        while True:
            g, text, voice, speed = self.q.get()
            if self._stale(g):
                continue
            try:
                self._speak(g, text, voice, speed)
            except Exception as e:  # noqa: BLE001 — a bad utterance must not kill the process
                log(f"kokoro: synthesis failed: {e}")
            if not self._stale(g):
                emit(b"END", b"")

    def _speak(self, g, text, voice, speed):
        # Nothing speakable produces no audio and no callback. Terminate at once
        # rather than leaving the daemon's reader to time out.
        if not any(c.isalnum() for c in text):
            return
        for part in (clauses(text) if self.split else [text]):
            if self._stale(g):
                return
            for seg in self.model.generate(
                text=part, voice=voice or DEFAULT_VOICE, speed=speed, lang_code="a"
            ):
                if self._stale(g):
                    return
                rate = getattr(seg, "sample_rate", RATE)
                # The daemon rejects a frame over 4 MB as implausible, and it is
                # right to: the length arrives over a pipe and sizes an
                # allocation. 4 MB is ~43 s at 24 kHz, which a single clause
                # will not reach — but "will not" is not "cannot", and a
                # rejected frame desynchronises the stream for the rest of the
                # session rather than dropping one utterance.
                import numpy as np

                pcm = np.asarray(seg.audio, dtype=np.float32).reshape(-1)
                span = 240_000  # 10 s at 24 kHz
                for off in range(0, len(pcm), span):
                    if self._stale(g):
                        return
                    emit_audio(pcm[off:off + span], rate)


def selftest() -> int:
    """`--selftest` — check the clause splitter without loading a model.

    Deliberately importable and runnable on a machine with no mlx-audio: the
    splitting rule is where words can silently go missing from a reply, and that
    is worth checking everywhere, not only where the engine runs.
    """
    cases = [
        ("That change looks safe, but it will need a migration for the existing rows.",
         ["That change looks safe,", "but it will need a migration for the existing rows."]),
        # A short tail joins the clause before it: a separate synthesis pass for
        # two words costs more than starting them sooner saves.
        ("I checked the tests, and they pass, ok.", ["I checked the tests, and they pass, ok."]),
        ("The daemon is running.", ["The daemon is running."]),
        ("", [""]),
        ("...", ["..."]),
    ]
    bad = 0
    for text, want in cases:
        got = clauses(text)
        if got != want:
            bad += 1
            log(f"FAIL {text!r}\n  want {want}\n  got  {got}")
    for text, _ in cases:
        # Every clause together must be the input. A splitter that drops a word
        # drops it from what the assistant says, silently.
        if " ".join(clauses(text)).split() != text.split():
            bad += 1
            log(f"FAIL reconstruction {text!r}")
    log("selftest: FAILED" if bad else "selftest: ok")
    return 1 if bad else 0


def main():
    if "--selftest" in sys.argv:
        return selftest()
    try:
        eng = Engine()
    except Exception as e:  # noqa: BLE001
        # Exit non-zero and say why. The daemon falls back to the platform
        # engine, and a silent fallback would leave the user wondering why the
        # voice they configured never arrived.
        log(f"kokoro: unavailable: {e}")
        return 1
    eng.warm()

    for line in sys.stdin:
        line = line.strip()
        if not line:
            continue
        try:
            o = json.loads(line)
        except json.JSONDecodeError:
            continue
        if o.get("cmd") == "cancel":
            eng.cancel()
        elif o.get("cmd") == "clauses":
            eng.split = bool(o.get("on", True))
        elif "text" in o:
            # `rate` is the Swift sidecar's AVSpeechUtterance rate, where 0.5 is
            # normal. Kokoro's `speed` is a multiplier where 1.0 is normal.
            r = float(o.get("rate", 0.52))
            eng.enqueue(o["text"], o.get("voice"), max(0.5, min(2.0, r / 0.52)))
    return 0


if __name__ == "__main__":
    sys.exit(main())

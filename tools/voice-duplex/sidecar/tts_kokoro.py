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

# Kokoro speaks nine language variants and nothing else, which is fewer than the
# 99 the recogniser can detect. Each needs its own phonemizer *and* a voice
# trained on that language: an English voice fed Devanagari does not read it
# with an accent, it reads the wrong sounds entirely.
#
# Support is decided at runtime, not from this table. `misaki[en]` covers
# English; Spanish, French, Hindi, Italian and Portuguese fall back to espeak,
# which misaki bundles; Japanese and Chinese need `misaki[ja]` / `misaki[zh]`
# and raise ImportError without them. Assuming a language works because it is
# listed here is exactly the failure this comment exists to prevent.
LANGS = {
    "en": ("a", "af_heart"),
    "en-gb": ("b", "bf_emma"),
    "es": ("e", "ef_dora"),
    "fr": ("f", "ff_siwis"),
    "hi": ("h", "hf_alpha"),
    "it": ("i", "if_sara"),
    "ja": ("j", "jf_alpha"),
    "pt": ("p", "pf_dora"),
    "zh": ("z", "zf_xiaobei"),
}


def for_language(lang):
    """`(lang_code, voice)` for a detected language, or `None` if unsupported.

    Silence is a better answer than confident mispronunciation: a Japanese reply
    read by an English voice is not accented Japanese, it is noise.
    """
    if not lang:
        return LANGS["en"]
    return LANGS.get(lang.lower()) or LANGS.get(lang.split("-")[0].lower())
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
        # Languages already reported as unspeakable, so the log records the
        # problem once rather than once per sentence.
        self.warned = set()
        self.lock = threading.Lock()
        threading.Thread(target=self._worker, daemon=True).start()

    def warm(self):
        """The first synthesis in a process builds the lazy graph. Pay it before
        anyone is listening — the daemon's spawn sends 'ok' and drains, and a
        cold first turn would otherwise charge the user for it."""
        list(self.model.generate(text="ok", voice=DEFAULT_VOICE, speed=1.0, lang_code="a"))

    def enqueue(self, text, voice, speed, lang):
        with self.lock:
            self.q.put((self.gen, text, voice, speed, lang))

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

    def _unsupported(self, lang):
        """Say once, to the log, that this language cannot be spoken.

        Once rather than per sentence: a whole conversation in an unsupported
        language would otherwise fill the daemon log with the same line. The
        daemon notices the silence on its own and tells the user — this is the
        record of *why*.
        """
        if lang in self.warned:
            return
        self.warned.add(lang)
        log(
            f"kokoro: no voice for language {lang!r}. Kokoro speaks "
            f"{', '.join(sorted(LANGS))} and nothing else; Japanese and Chinese "
            f"additionally need misaki[ja] / misaki[zh]. Set "
            f"[voice] tts_engine = \"system\" for wider language coverage."
        )

    def _stale(self, g) -> bool:
        with self.lock:
            return g != self.gen

    def _worker(self):
        while True:
            g, text, voice, speed, lang = self.q.get()
            if self._stale(g):
                continue
            try:
                self._speak(g, text, voice, speed, lang)
            except Exception as e:  # noqa: BLE001 — a bad utterance must not kill the process
                log(f"kokoro: synthesis failed: {e}")
            if not self._stale(g):
                emit(b"END", b"")

    def _speak(self, g, text, voice, speed, lang):
        # Nothing speakable produces no audio and no callback. Terminate at once
        # rather than leaving the daemon's reader to time out.
        if not any(c.isalnum() for c in text):
            return
        picked = for_language(lang)
        if picked is None:
            self._unsupported(lang)
            return
        code, default_voice = picked
        # A voice the caller pinned is only honoured for the language it belongs
        # to. `af_heart` reading Hindi is the bug this whole path exists to
        # avoid, and a session voice chosen while speaking English must not
        # follow the user into another language.
        chosen = voice if (voice and voice[:1] == code) else default_voice
        for part in (clauses(text) if self.split else [text]):
            if self._stale(g):
                return
            try:
                segs = self.model.generate(
                    text=part, voice=chosen, speed=speed, lang_code=code
                )
                segs = list(segs)
            except ImportError as e:
                # Japanese and Chinese need a misaki extra that is not installed
                # until someone installs it. The table above says Kokoro *has* a
                # voice; only trying tells you whether it can be reached.
                self._unsupported(lang)
                log(f"kokoro: {lang} phonemizer unavailable: {e}")
                return
            for seg in segs:
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
    lang_cases = [
        ("hi", ("h", "hf_alpha")),
        ("en", ("a", "af_heart")),
        # Whisper reports region tags; the base language is what selects a voice.
        ("en-US", ("a", "af_heart")),
        ("pt-BR", ("p", "pf_dora")),
        # Kokoro has no Korean or Tamil voice. Silence beats reading Hangul with
        # an English voice, which is not accented Korean but the wrong sounds.
        ("ko", None),
        ("ta", None),
        (None, ("a", "af_heart")),
    ]
    bad = 0
    for lang, want in lang_cases:
        got = for_language(lang)
        if got != want:
            bad += 1
            log(f"FAIL language {lang!r}: want {want}, got {got}")

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


# Named voices per language. Kokoro ships more, but these are the ones with a
# distinct enough character to be worth offering: a settings list of 54
# near-identical names is not a choice, it is a scrolling exercise.
VOICES = {
    "en": ["af_heart", "af_bella", "af_nicole", "am_michael", "am_fenrir", "am_puck"],
    "en-gb": ["bf_emma", "bf_isabella", "bm_george", "bm_lewis"],
    "es": ["ef_dora", "em_alex"],
    "fr": ["ff_siwis"],
    "hi": ["hf_alpha", "hf_beta", "hm_omega", "hm_psi"],
    "it": ["if_sara", "im_nicola"],
    "ja": ["jf_alpha", "jf_gongitsune", "jm_kumo"],
    "pt": ["pf_dora", "pm_alex"],
    "zh": ["zf_xiaobei", "zf_xiaoni", "zm_yunjian", "zm_yunxi"],
}


def list_voices() -> int:
    """`--voices` — what a settings screen can offer, as JSON.

    Language is part of the identity, not a filter applied afterwards: a voice
    is trained on one language and reading another with it produces the wrong
    sounds rather than an accent.
    """
    out = []
    for lang, names in VOICES.items():
        for n in names:
            out.append({
                "id": n,
                # `af_heart` → `Heart`. The prefix encodes language and gender
                # and is noise in a list already grouped by language.
                "name": n.split("_", 1)[1].replace("_", " ").title(),
                "lang": lang,
                "quality": "neural",
                "gender": "female" if n[1] == "f" else "male",
            })
    print(json.dumps({"engine": "kokoro", "voices": out,
                      "languages": sorted(LANGS)}))
    return 0


def main():
    if "--selftest" in sys.argv:
        return selftest()
    if "--voices" in sys.argv:
        return list_voices()
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
            # `lang` is the language the recogniser actually detected, not a
            # session setting: a bilingual speaker switches mid-conversation and
            # the voice has to switch with them.
            # `rate` is the Swift sidecar's AVSpeechUtterance rate, where 0.5 is
            # normal. Kokoro's `speed` is a multiplier where 1.0 is normal.
            r = float(o.get("rate", 0.52))
            eng.enqueue(
                o["text"], o.get("voice"), max(0.5, min(2.0, r / 0.52)), o.get("lang")
            )
    return 0


if __name__ == "__main__":
    sys.exit(main())

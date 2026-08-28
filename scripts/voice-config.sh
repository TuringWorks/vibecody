#!/usr/bin/env bash
#
# voice-config.sh — select the speech engine, and report what will actually run.
#
# `--status` answers the question that mattered and had no answer: *which engine
# is the daemon going to use?* For most of this feature's life the honest answer
# was "neither" — no sidecar was ever built or installed, `tts_sidecar` was unset
# on every real machine, and the daemon fell through to `say`. Nothing said so.
set -euo pipefail

CFG="${VIBECLI_CONFIG:-$HOME/.vibecli/config.toml}"
BIN_DIR="${VIBECLI_BIN_DIR:-$HOME/.local/bin}"
KOKORO_PY="$HOME/.vibecli/tts/bin/python"
KOKORO_SC="$HOME/.vibecli/sidecars/tts_kokoro.py"

status() {
  echo "config:  $CFG"
  local engine="system"
  if [ -f "$CFG" ]; then
    engine="$(awk -F'"' '/^[[:space:]]*tts_engine[[:space:]]*=/{print $2}' "$CFG" | tail -1)"
    engine="${engine:-system}"
  else
    echo "         (none — defaults apply)"
  fi
  echo "engine:  $engine"
  echo

  if [ "$engine" = "kokoro" ]; then
    [ -x "$KOKORO_PY" ] && echo "  ok    interpreter  $KOKORO_PY" \
                        || echo "  MISS  interpreter  $KOKORO_PY   → make voice-kokoro"
    [ -f "$KOKORO_SC" ] && echo "  ok    sidecar      $KOKORO_SC" \
                        || echo "  MISS  sidecar      $KOKORO_SC   → make voice-kokoro"
  fi

  # The streaming system sidecar is what `discover_sidecar` looks for, and its
  # absence is the difference between streaming speech and one `say` process per
  # utterance. Reported even when Kokoro is selected: it is the fallback.
  if [ -x "$BIN_DIR/vibecli-tts" ]; then
    echo "  ok    system sidecar  $BIN_DIR/vibecli-tts"
  else
    echo "  MISS  system sidecar  $BIN_DIR/vibecli-tts   → make voice-sidecar"
    echo "        Without it every reply is synthesised by one \`say\` process per"
    echo "        utterance, in the system default voice. That is the mechanical one."
  fi
}

set_engine() {
  local engine="$1"
  mkdir -p "$(dirname "$CFG")"
  touch "$CFG"
  # Rewrite the [voice] keys in place rather than appending a second [voice]
  # table — TOML takes the first and silently ignores the duplicate, so an
  # appended block reads as "the setting did nothing".
  python3 - "$CFG" "$engine" "$KOKORO_PY" "$KOKORO_SC" <<'PY'
import re, sys
path, engine, py, sc = sys.argv[1:5]
src = open(path).read()
want = {
    "tts_engine": f'"{engine}"',
    "tts_sidecar": f'"{py}"',
    "tts_sidecar_args": f'["{sc}"]',
}
m = re.search(r'^\[voice\]\s*$', src, re.M)
if not m:
    body = "\n".join(f"{k} = {v}" for k, v in want.items())
    src = src.rstrip("\n") + ("\n\n" if src.strip() else "") + "[voice]\n" + body + "\n"
else:
    start = m.end()
    nxt = re.search(r'^\[', src[start:], re.M)
    end = start + (nxt.start() if nxt else len(src) - start)
    section = src[start:end]
    for k, v in want.items():
        line = f"{k} = {v}"
        if re.search(rf'^\s*{k}\s*=', section, re.M):
            section = re.sub(rf'^\s*{k}\s*=.*$', line, section, count=1, flags=re.M)
        else:
            section = section.rstrip("\n") + "\n" + line + "\n"
    src = src[:start] + section + src[end:]
open(path, "w").write(src)
print(f"wrote {path}: tts_engine = {engine}")
PY
}

case "${1:---status}" in
  --status) status ;;
  system)   set_engine system ;;
  kokoro)   set_engine kokoro ;;
  *) echo "usage: voice-config.sh [--status|system|kokoro]" >&2; exit 1 ;;
esac

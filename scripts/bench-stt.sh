#!/usr/bin/env bash
# STT bench: whisper-base.en (the shipped default, via the app's exact
# transcribe code path) vs Confucius4-R2T2 (experimental MLX sidecar model),
# over the checked-in fixtures in apps/desktop/src-tauri/tests/fixtures/stt.
#
# Usage:            bash scripts/bench-stt.sh
# Whisper leg:      needs the whisper-base model in the app's model store
#                   (or the dev vendor tree) — same resolution as the app.
# R2T2 leg:         needs a python with mlx-audio and the R2T2 model dir;
#                   picked up from the app's provisioned runtime by default.
#                   Skipped (loudly, not silently) when absent.
# Env knobs:
#   FUNBUTTON_R2T2_PY   python interpreter with mlx-audio (default: the app
#                       runtime venv under Application Support)
#   FUNBUTTON_R2T2_DIR  R2T2 model dir (see scripts/bench-stt.py)
set -euo pipefail

REPO="$(cd "$(dirname "$0")/.." && pwd)"
TAURI="$REPO/apps/desktop/src-tauri"
OUT="$(mktemp -t fb-bench-stt)"
trap 'rm -f "$OUT"' EXIT

echo "→ whisper leg (app code path, cargo test --release)" >&2
(cd "$TAURI" && cargo test --release --lib bench_stt_fixtures -- --ignored --nocapture 2>/dev/null) \
  | grep '^BENCH ' >> "$OUT" || { echo "whisper leg FAILED (is the whisper-base model downloaded?)" >&2; exit 1; }

R2T2_PY="${FUNBUTTON_R2T2_PY:-$HOME/Library/Application Support/ai.funbutton.desktop/r2t2-runtime/venv/bin/python}"
if [[ -x "$R2T2_PY" ]]; then
  echo "→ r2t2 leg ($R2T2_PY)" >&2
  if ! "$R2T2_PY" "$REPO/scripts/bench-stt.py" >> "$OUT"; then
    echo "r2t2 leg SKIPPED (model/runtime missing — activate the R2T2 model in Settings → Models first)" >&2
  fi
else
  echo "r2t2 leg SKIPPED (no runtime python at $R2T2_PY — set FUNBUTTON_R2T2_PY or activate R2T2 in the app)" >&2
fi

echo
echo "file · engine · arm · ms · transcript"
echo "-------------------------------------"
# Plain-text table from the BENCH json lines; python3 is always on macOS.
sed 's/^BENCH //' "$OUT" | python3 -c '
import json, sys
rows = [json.loads(l) for l in sys.stdin if l.strip()]
rows.sort(key=lambda r: (r["file"], r["engine"], r["arm"]))
for r in rows:
    print(f'"'"'{r["file"]:<10} {r["engine"]:<18} {r["arm"]:<10} {r["ms"]:>5}ms  {r["text"]}'"'"')
'

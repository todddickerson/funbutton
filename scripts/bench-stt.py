#!/usr/bin/env python3
"""R2T2 leg of the STT bench — transcribe the repo fixtures with
Confucius4-R2T2 (MLX), plain and with hotwords, and print one
`BENCH {json}` line per run (same shape as the Rust whisper leg).

Must run under a python that has mlx-audio installed (the app's provisioned
runtime venv, or any venv with mlx-audio==0.5.5). scripts/bench-stt.sh picks
the interpreter; this file has no CLI deps beyond mlx-audio itself.

Env:
  FUNBUTTON_R2T2_DIR   local model directory (default: the app's model store
                       at ~/Library/Application Support/ai.funbutton.desktop/
                       models/confucius4-r2t2-mlx-4bit)
  FUNBUTTON_BENCH_HOTWORDS  comma-separated hotword list override
"""
from __future__ import annotations

import json
import os
import sys
import time
from pathlib import Path

REPO = Path(__file__).resolve().parent.parent
FIXTURES = REPO / "apps/desktop/src-tauri/tests/fixtures/stt"
DEFAULT_MODEL_DIR = (
    Path.home()
    / "Library/Application Support/ai.funbutton.desktop/models/confucius4-r2t2-mlx-4bit"
)
# Mirrors the spike: terms a dev-mode dictation plausibly needs spelled right.
DEFAULT_HOTWORDS = [
    "ClickFunnels",
    "auth_middleware.rs",
    "kubectl",
    "git push origin main",
]


def main() -> int:
    model_dir = Path(os.environ.get("FUNBUTTON_R2T2_DIR", DEFAULT_MODEL_DIR))
    if not (model_dir / "model.safetensors").exists():
        print(f"SKIP r2t2: no model at {model_dir}", file=sys.stderr)
        return 2

    hot = os.environ.get("FUNBUTTON_BENCH_HOTWORDS")
    hotwords = [h.strip() for h in hot.split(",") if h.strip()] if hot else DEFAULT_HOTWORDS

    from mlx_audio.stt.utils import load_audio, load_model

    t0 = time.perf_counter()
    model = load_model(str(model_dir))
    print(f"# r2t2 model loaded in {time.perf_counter() - t0:.1f}s", file=sys.stderr)

    wavs = sorted(FIXTURES.glob("*.wav"))
    if not wavs:
        print(f"SKIP r2t2: no fixtures in {FIXTURES}", file=sys.stderr)
        return 2
    for wav in wavs:
        audio = load_audio(str(wav))
        for arm, kwargs in (("plain", {}), ("hotwords", {"hotwords": hotwords})):
            t0 = time.perf_counter()
            out = model.generate(audio, language="English", **kwargs)
            ms = int((time.perf_counter() - t0) * 1000)
            print(
                "BENCH "
                + json.dumps(
                    {
                        "engine": "confucius4-r2t2",
                        "file": wav.name,
                        "arm": arm,
                        "ms": ms,
                        "text": getattr(out, "text", str(out)),
                    }
                ),
                flush=True,
            )
    return 0


if __name__ == "__main__":
    sys.exit(main())

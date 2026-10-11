# FunButton R2T2 sidecar — a tiny localhost HTTP server wrapping the
# Confucius4-R2T2 (Qwen3-ASR, MLX) model via mlx-audio.
#
# Baked into the Rust binary (include_str!) and written into the runtime dir
# at spawn time, so the script version always matches the app binary. Runs
# under the app-provisioned venv in Application Support — stdlib HTTP only,
# no web-framework deps.
#
# Contract (mirrors how the app talks to llama-server):
#   GET  /health               -> 200 {"status":"ok"} once the model is loaded,
#                                 503 while loading, 500 if the load failed.
#   POST /transcribe           -> body: raw WAV bytes.
#        X-Funbutton-Hotwords: percent-encoded, comma-separated hotword list
#                                 (optional). Response: 200 {"text": "..."}.
#
# PRIVACY: hotwords and transcripts are NEVER printed or persisted here.
# Binds 127.0.0.1 only.

from __future__ import annotations

import json
import sys
import tempfile
import threading
import urllib.parse
from http.server import BaseHTTPRequestHandler, ThreadingHTTPServer
from pathlib import Path

MODEL = {"model": None, "error": None}
LOCK = threading.Lock()


def load(model_dir: str) -> None:
    try:
        from mlx_audio.stt.utils import load_model

        MODEL["model"] = load_model(model_dir)
        print("r2t2: model loaded", flush=True)
    except Exception as e:  # noqa: BLE001 — surface any load failure via /health
        MODEL["error"] = str(e)
        print(f"r2t2: model load FAILED: {e}", flush=True)


class Handler(BaseHTTPRequestHandler):
    def log_message(self, *args):  # silence default request logging (privacy)
        pass

    def _json(self, code: int, obj: dict) -> None:
        body = json.dumps(obj).encode()
        self.send_response(code)
        self.send_header("Content-Type", "application/json")
        self.send_header("Content-Length", str(len(body)))
        self.end_headers()
        self.wfile.write(body)

    def do_GET(self):  # noqa: N802 — http.server API
        if self.path != "/health":
            return self._json(404, {"error": "not found"})
        if MODEL["model"] is not None:
            return self._json(200, {"status": "ok"})
        if MODEL["error"] is not None:
            return self._json(500, {"status": "failed", "error": MODEL["error"]})
        return self._json(503, {"status": "loading"})

    def do_POST(self):  # noqa: N802 — http.server API
        if self.path != "/transcribe":
            return self._json(404, {"error": "not found"})
        model = MODEL["model"]
        if model is None:
            return self._json(503, {"error": MODEL["error"] or "model not loaded"})
        length = int(self.headers.get("Content-Length") or 0)
        if length <= 0:
            return self._json(400, {"error": "empty body"})
        wav = self.rfile.read(length)

        hotwords: list[str] = []
        raw_hot = self.headers.get("X-Funbutton-Hotwords")
        if raw_hot:
            try:
                decoded = urllib.parse.unquote(raw_hot)
                hotwords = [h.strip() for h in decoded.split(",") if h.strip()]
            except Exception:  # noqa: BLE001 — malformed header: drop bias, still transcribe
                hotwords = []

        try:
            from mlx_audio.stt.utils import load_audio

            # miniaudio decodes from a file path; keep the temp file inside our
            # runtime dir (0600 by default) and remove it immediately.
            with tempfile.NamedTemporaryFile(
                suffix=".wav", dir=str(TMP_DIR), delete=False
            ) as f:
                f.write(wav)
                tmp = Path(f.name)
            try:
                audio = load_audio(str(tmp))
            finally:
                tmp.unlink(missing_ok=True)

            # One request at a time: MLX generate isn't re-entrant-safe and the
            # app only ever sends one dictation at a time anyway.
            with LOCK:
                if hotwords:
                    out = model.generate(audio, language="English", hotwords=hotwords)
                else:
                    out = model.generate(audio, language="English")
            text = getattr(out, "text", str(out))
            return self._json(200, {"text": text})
        except Exception as e:  # noqa: BLE001 — degrade to the app's fallback chain
            return self._json(500, {"error": str(e)})


def main() -> int:
    if len(sys.argv) != 3:
        print("usage: r2t2_server.py <port> <model_dir>", file=sys.stderr)
        return 2
    port = int(sys.argv[1])
    model_dir = sys.argv[2]

    global TMP_DIR
    TMP_DIR = Path(tempfile.mkdtemp(prefix="fb-r2t2-"))

    # Bind first so the app's /health poll connects immediately (503 while the
    # model loads on a background thread), mirroring llama-server's behavior.
    server = ThreadingHTTPServer(("127.0.0.1", port), Handler)
    threading.Thread(target=load, args=(model_dir,), daemon=True).start()
    print(f"r2t2: serving on 127.0.0.1:{port}", flush=True)
    try:
        server.serve_forever()
    except KeyboardInterrupt:
        pass
    return 0


TMP_DIR = Path(tempfile.gettempdir())

if __name__ == "__main__":
    sys.exit(main())

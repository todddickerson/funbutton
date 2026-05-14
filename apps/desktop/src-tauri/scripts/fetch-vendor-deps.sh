#!/usr/bin/env bash
# Fetch the bundled local-inference runtime (llama-server binary + Qwen 2.5 GGUF).
# Idempotent — re-running is a no-op once files are present.
#
# Run this once before `pnpm tauri build` / `pnpm tauri dev`. CI workflow
# should also run it before the build step.

set -euo pipefail

HERE="$(cd "$(dirname "$0")" && pwd)"
VENDOR="${HERE%/scripts}/vendor/llama"
mkdir -p "$VENDOR"

LLAMA_TAG="b9151"
LLAMA_TAR="llama-${LLAMA_TAG}-bin-macos-arm64.tar.gz"
LLAMA_URL="https://github.com/ggml-org/llama.cpp/releases/download/${LLAMA_TAG}/${LLAMA_TAR}"

MODEL_NAME="qwen2.5-1.5b-instruct-q4_k_m.gguf"
MODEL_URL="https://huggingface.co/Qwen/Qwen2.5-1.5B-Instruct-GGUF/resolve/main/${MODEL_NAME}?download=true"

cd "$VENDOR"

# --- llama-server + dylibs ---
if [[ ! -x "$VENDOR/llama-server" ]]; then
  echo "→ downloading llama.cpp ${LLAMA_TAG} (macOS arm64)…"
  curl -sSfL --retry 3 "$LLAMA_URL" -o "/tmp/${LLAMA_TAR}"
  TMPDIR_X="$(mktemp -d)"
  tar -xzf "/tmp/${LLAMA_TAR}" -C "$TMPDIR_X"
  EXTRACTED="$(find "$TMPDIR_X" -maxdepth 2 -type d -name "llama-${LLAMA_TAG}")"
  if [[ -z "$EXTRACTED" ]]; then
    echo "❌ extraction failed, no llama-${LLAMA_TAG}/ in tarball" >&2
    exit 1
  fi
  cp -a "$EXTRACTED"/* "$VENDOR"/
  chmod +x "$VENDOR/llama-server"
  rm -rf "$TMPDIR_X" "/tmp/${LLAMA_TAR}"
  echo "  ✓ llama-server installed at $VENDOR/llama-server"
else
  echo "  ✓ llama-server already present"
fi

# --- GGUF model ---
MODEL_PATH="$VENDOR/$MODEL_NAME"
if [[ ! -s "$MODEL_PATH" ]]; then
  echo "→ downloading $MODEL_NAME (~1.1 GB)…"
  # huggingface.co serves a redirect to a CDN; -L follows it
  curl -L --retry 3 --fail --output "$MODEL_PATH" "$MODEL_URL"
  echo "  ✓ model installed at $MODEL_PATH"
else
  echo "  ✓ model already present ($(du -h "$MODEL_PATH" | awk '{print $1}'))"
fi

# --- sanity checks ---
if ! "$VENDOR/llama-server" --version >/dev/null 2>&1; then
  echo "⚠ llama-server --version failed — Gatekeeper may be blocking. Run once manually:"
  echo "    xattr -dr com.apple.quarantine $VENDOR"
fi

echo "✅ vendor deps ready at $VENDOR"

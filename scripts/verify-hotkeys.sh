#!/usr/bin/env bash
# Physical hotkey verification for FunButton.
#
# An autonomous build can prove the keyboard detection and the DOWN/UP decision
# logic (see `cargo test --release --lib` + the `detect_on_this_machine` and
# `listener_maps_injected_cgevents` ignored tests), and it can confirm the taps
# install with Input Monitoring granted — but it can't physically hold a key,
# and CGEvent injection into a HID-level tap is privileged. So this last mile —
# hold the real key, watch the real log line — is a ~1-minute human check.
#
# Usage:  scripts/verify-hotkeys.sh [/Applications/FunButton.app]
#
# Then: open Settings → The Button, click (or "press") a key — Fn, Right Option,
# Right Control — and HOLD it ~1s, release. Each hold should print exactly one
# DOWN then one UP for the ARMED kind, and nothing for the others.
set -uo pipefail

APP="${1:-/Applications/FunButton.app}"
BIN="$APP/Contents/MacOS/funbutton"

if [ ! -x "$BIN" ]; then
  echo "error: binary not found or not executable: $BIN" >&2
  echo "hint: pass the .app path as arg 1, or build one with 'npm run tauri build -- --bundles app'." >&2
  exit 1
fi

echo "==> Quit any running FunButton (menu-bar icon → Quit), then press Enter."
read -r _

cat <<'EOS'
==> Launching FunButton with RUST_LOG=info.
    In Settings → The Button:
      1. Click a key in the diagram (or "Press the key you want" and press it).
      2. HOLD that physical key for ~1s, then release.
      3. Watch below for:   hotkey: <Kind> DOWN   then   hotkey: <Kind> UP
    Repeat for Fn, Right Option, Right Control. Ctrl-C when finished.

    Filtered log (tap install + hotkey edges + capture + arm changes):
--------------------------------------------------------------------------
EOS

RUST_LOG=info "$BIN" 2>&1 \
  | grep --line-buffered -E "tap installed|tap creation FAILED|hotkey:|armed hotkey|hotkey capture|keyboard detected"

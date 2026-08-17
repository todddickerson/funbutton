#!/usr/bin/env bash
#
# FunButton one-line installer — https://funbutton.ai/install.sh
#
#   curl -fsSL https://funbutton.ai/install.sh | bash
#
# What it does, in plain sight (read it — it's short on purpose):
#   1. Confirms you're on an Apple Silicon Mac.
#   2. Downloads the latest FunButton .dmg from https://funbutton.ai/download.
#   3. Mounts it, copies FunButton.app into /Applications, unmounts.
#   4. Clears the com.apple.quarantine flag from *that one app bundle* so macOS
#      doesn't mislabel the unsigned alpha as "damaged." Nothing else is touched.
#
# What it never does: no sudo, no `spctl --master-disable`, no global Gatekeeper
# changes, no telemetry, no piping anything else into a shell. If /Applications
# isn't writable by you, it stops and tells you — it will not escalate silently.
#
# The permanent fix is a real Developer ID signature (see SIGNING.md); this just
# removes the friction until that lands. GPLv3 — github.com/todddickerson/funbutton
set -euo pipefail

APP="FunButton.app"
DEST="/Applications/${APP}"
DOWNLOAD_URL="https://funbutton.ai/download"

# --- pretty, dependency-free logging -----------------------------------------
red()  { printf '\033[31m%s\033[0m\n' "$*"; }
grn()  { printf '\033[32m%s\033[0m\n' "$*"; }
dim()  { printf '\033[2m%s\033[0m\n'  "$*"; }
step() { printf '\033[1;36m==>\033[0m %s\n' "$*"; }
die()  { red "error: $*"; exit 1; }

# --- 1. platform gate --------------------------------------------------------
[ "$(uname -s)" = "Darwin" ] || die "FunButton is macOS-only (this is $(uname -s))."
if [ "$(uname -m)" != "arm64" ]; then
  die "FunButton ships for Apple Silicon (arm64) only. This Mac reports '$(uname -m)'.
     An Intel build isn't available yet — watch https://funbutton.ai for updates."
fi

# --- 2. refuse to clobber a running copy -------------------------------------
# The on-disk executable is Contents/MacOS/funbutton, so the process name is the
# lowercase "funbutton" (not the display name "FunButton").
if pgrep -x "funbutton" >/dev/null 2>&1; then
  die "FunButton is currently running. Quit it (menu-bar icon -> Quit) and re-run this installer."
fi

# --- 3. scratch space, always cleaned up -------------------------------------
TMP="$(mktemp -d "${TMPDIR:-/tmp}/funbutton.XXXXXX")"
MNT="${TMP}/mnt"
DMG="${TMP}/FunButton.dmg"
cleanup() {
  [ -d "$MNT" ] && hdiutil detach "$MNT" -quiet >/dev/null 2>&1 || true
  rm -rf "$TMP" >/dev/null 2>&1 || true
}
trap cleanup EXIT

# --- 4. download -------------------------------------------------------------
step "Downloading the latest FunButton build"
dim  "    from ${DOWNLOAD_URL}"
if ! curl -fL --progress-bar -o "$DMG" "$DOWNLOAD_URL"; then
  die "download failed. Check your connection, or grab the .dmg manually from
     https://funbutton.ai and drag FunButton to Applications."
fi
[ -s "$DMG" ] || die "downloaded file is empty — the release may be mid-publish. Try again shortly."

# --- 5. mount ----------------------------------------------------------------
step "Mounting the disk image"
mkdir -p "$MNT"
hdiutil attach "$DMG" -nobrowse -noverify -noautoopen -mountpoint "$MNT" -quiet \
  || die "couldn't mount the .dmg. It may be a partial download — try again."
[ -d "${MNT}/${APP}" ] || die "the .dmg didn't contain ${APP} (unexpected layout)."

# --- 6. install into /Applications (no sudo) ---------------------------------
if [ ! -w "/Applications" ]; then
  die "/Applications isn't writable by your user, so this script won't force it.
     Open the downloaded .dmg yourself and drag FunButton to Applications, then run:
       xattr -dr com.apple.quarantine \"${DEST}\""
fi
if [ -e "$DEST" ]; then
  step "Replacing the existing ${APP}"
  rm -rf "$DEST" || die "couldn't remove the old ${DEST}."
fi
step "Installing ${APP} to /Applications"
ditto "${MNT}/${APP}" "$DEST" || die "copy into /Applications failed."

# --- 7. the whole point: clear quarantine on OUR bundle only -----------------
step "Clearing the quarantine flag on ${APP} (and nothing else)"
xattr -dr com.apple.quarantine "$DEST" >/dev/null 2>&1 || true

# --- 8. done -----------------------------------------------------------------
grn "FunButton is installed — no \"damaged\" dialog, no manual xattr needed."
echo
echo "Next steps:"
echo "  1. Open FunButton (it lives in your menu bar)."
echo "  2. Grant the three permissions it asks for:"
echo "       - Microphone       (to hear you)"
echo "       - Accessibility    (to type at your cursor)"
echo "       - Input Monitoring (to detect your push-to-talk key)"
echo "  3. Open Settings from the menu-bar icon and pick your hotkey."
echo
dim "The on-device models (~1.1 GB) download once on first run, then it's fully offline."
echo
step "Launching FunButton…"
open "$DEST" || dim "Couldn't auto-launch — open FunButton from /Applications yourself."

#!/usr/bin/env bash
#
# update-cask.sh — keep the Homebrew tap (todddickerson/homebrew-funbutton) in
# lockstep with the latest funbutton GitHub release.
#
#   scripts/update-cask.sh           # sync the tap to the latest release
#   scripts/update-cask.sh v0.1.9    # sync the tap to a specific tag
#
# This is the cask half of the release flow — the mirror image of
# scripts/update-landing-version.sh (which repoints funbutton.ai). Run BOTH after
# every release so `brew install --cask todddickerson/funbutton/funbutton` never
# serves a stale version or a mismatched sha256.
#
# Idempotent: it recomputes the sha256 from the *published* .dmg and only commits
# when the tap's version or sha256 actually changes. Safe to re-run any number of
# times.
#
# Auth: the `gh` CLI (already logged in as todddickerson). We deliberately do NOT
# read or source ~/clawd/.env here — gh owns GitHub auth for both the API reads
# and the tap push, so there is no secret to grep. `gh auth setup-git` wires the
# git credential helper so the push authenticates the same way.
set -euo pipefail

REPO="todddickerson/funbutton"
TAP_REPO="todddickerson/homebrew-funbutton"
CASK_PATH="Casks/funbutton.rb"

command -v gh >/dev/null 2>&1 || { echo "!! gh CLI not found — install it or run the update by hand" >&2; exit 1; }

# 1. Resolve the target tag (arg, else the latest release).
TAG="${1:-}"
if [ -z "$TAG" ]; then
  TAG=$(gh release view --repo "$REPO" --json tagName -q .tagName 2>/dev/null || true)
fi
[ -n "$TAG" ] || { echo "!! could not resolve a release tag" >&2; exit 1; }
BARE="${TAG#v}"
echo "target release : $TAG  (version $BARE)"

# 2. Resolve the macOS arm64 .dmg asset for this tag.
DMG=$(gh release view "$TAG" --repo "$REPO" --json assets -q '.assets[].name' 2>/dev/null \
        | grep -i '\.dmg$' | grep -iE 'aarch64|arm64' | head -1 || true)
[ -z "$DMG" ] && DMG=$(gh release view "$TAG" --repo "$REPO" --json assets -q '.assets[].name' 2>/dev/null \
        | grep -i '\.dmg$' | head -1 || true)
[ -n "$DMG" ] || { echo "!! no .dmg asset found on $TAG" >&2; exit 1; }
echo "dmg asset      : $DMG"

WORK=$(mktemp -d)
trap 'rm -rf "$WORK"' EXIT

# 3. Compute sha256 from the PUBLISHED asset (never a local build — the tap must
#    match exactly what users download).
echo "downloading asset to compute sha256 ..."
gh release download "$TAG" --repo "$REPO" --pattern "$DMG" --dir "$WORK" --clobber
SHA=$(shasum -a 256 "$WORK/$DMG" | awk '{print $1}')
[ -n "$SHA" ] || { echo "!! failed to compute sha256" >&2; exit 1; }
echo "sha256         : $SHA"

# 4. Clone the tap, rewrite version + sha256.
gh auth setup-git >/dev/null 2>&1 || true
git clone --depth 1 "https://github.com/${TAP_REPO}.git" "$WORK/tap" >/dev/null 2>&1 \
  || { echo "!! could not clone $TAP_REPO" >&2; exit 1; }
CASK="$WORK/tap/$CASK_PATH"
[ -f "$CASK" ] || { echo "!! $CASK_PATH not found in tap" >&2; exit 1; }

# Rewrite only the two lines that change per release. `sed -i ''` is BSD/macOS.
sed -i '' -E "s/^  version \".*\"/  version \"${BARE}\"/" "$CASK"
sed -i '' -E "s/^  sha256 \".*\"/  sha256 \"${SHA}\"/" "$CASK"

# 5. Commit + push only if something changed. Stage first, then compare the index
#    against HEAD (same guard as update-landing-version.sh: `sed -i` bumps mtime
#    even on a no-op rewrite, so compare on content, not stat).
cd "$WORK/tap"
git add "$CASK_PATH"
if git diff --cached --quiet; then
  echo "tap already at $BARE / $SHA — no commit needed"
  exit 0
fi
git -c user.name="funbutton-release" -c user.email="release@funbutton.ai" \
    commit -q -m "funbutton ${BARE}"
git push -q origin HEAD || { echo "!! push failed — check \`gh auth status\`" >&2; exit 1; }
echo "tap updated to funbutton $BARE and pushed to $TAP_REPO"

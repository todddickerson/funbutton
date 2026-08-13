#!/bin/bash
# update-landing-version.sh — keep funbutton.ai pointing at the latest GitHub release.
#
# Run after any new release tag, or let the cron run it periodically.
# Safe to re-run: the version rewrite is idempotent, and the prod deploy ALWAYS
# fires (the old script early-exited "already current" before deploying, so a
# release could ship without the site ever redeploying — the caveat noted in
# PROGRESS.md). The download CTA itself is version-agnostic (the /download route
# resolves the current .dmg at request time), so this script only refreshes the
# visible version label and guarantees a redeploy.
set -euo pipefail

REPO="$HOME/src/Github/funbutton"
WEB="$REPO/apps/web"
VERSION_FILE="$WEB/app/version.ts"
PAGE="$WEB/app/page.tsx"
STATE="$HOME/clawd/memory/funbutton-landing-state.json"

cd "$REPO"

# 1. Latest release tag (e.g. v0.1.5) and the bare version (0.1.5).
LATEST=$(gh release view --json tagName -q .tagName 2>/dev/null || true)
[ -z "$LATEST" ] && { echo "no release found"; exit 1; }
BARE="${LATEST#v}"

# 2. Resolve the release's macOS .dmg asset name. The page's CTA uses /download,
#    so this is only for the state file and the belt-and-suspenders rewrite.
DMG=$(gh release view "$LATEST" --json assets -q '.assets[].name' 2>/dev/null \
        | grep -i '\.dmg$' | grep -iE 'aarch64|arm64' | head -1 || true)
[ -z "$DMG" ] && DMG=$(gh release view "$LATEST" --json assets -q '.assets[].name' 2>/dev/null \
        | grep -i '\.dmg$' | head -1 || true)
[ -z "$DMG" ] && { echo "no .dmg asset in $LATEST — aborting"; exit 1; }
echo "latest release: $LATEST · dmg asset: $DMG"

# 3. Rewrite the single version constant, plus any lingering hardcoded DMG link
#    (there should be none — the CTA is /download — but repoint it if it returns).
sed -i '' -E "s/(APP_VERSION = \")[0-9]+\.[0-9]+\.[0-9]+(\")/\1${BARE}\2/" "$VERSION_FILE"
sed -i '' -E "s#(releases/latest/download/)[^\"']*\.dmg#\1${DMG}#g" "$PAGE"
echo "version.ts -> $(grep -oE 'APP_VERSION = \"[0-9.]+\"' "$VERSION_FILE")"

# 4. Commit + push only if something actually changed (a no-op on a fresh ship
#    where the branch already carries the bumped version).
# Stage first, then compare the INDEX against HEAD. `sed -i` rewrites the file
# (new mtime/inode) even when the content is unchanged, which makes a bare
# `git diff --quiet` report a false "dirty" on stat alone — that sent us into
# the commit branch, and `git commit` finding nothing to commit exits non-zero,
# which under `set -e` aborted the script BEFORE the deploy ever ran. Staging
# normalizes on content, so `git diff --cached --quiet` is a true no-op check.
git add "$VERSION_FILE" "$PAGE"
if git diff --cached --quiet; then
  echo "landing already at $BARE — no commit needed"
else
  BRANCH=$(git rev-parse --abbrev-ref HEAD)
  git commit -q -m "chore(web): funbutton.ai -> $LATEST alpha"
  git push -q origin "$BRANCH"
  echo "committed + pushed landing version bump to $BRANCH"
fi

# 5. Deploy to prod — ALWAYS, regardless of whether step 4 committed anything.
# Read ONLY the token line from the dotenv rather than `source`-ing the whole
# file: sourcing runs every line in this shell, and under `set -u` any line that
# references an unbound variable aborts the script (a nounset expansion error is
# NOT rescued by `|| true`). Grepping one line has no such side effects.
# TOKEN SCOPE MATTERS (burned 2026-08-10..13, cost 3 days of a stale site):
# `VERCEL_TOKEN` is scoped to the `bootstrapped-cf259a39` team, which owns a
# DIFFERENT funbutton project (funbutton.vercel.app only). funbutton.ai lives
# under team_WGP9MIPM09U2lDW7YDqVIDsK (`todddickerson`), which that token cannot
# see — the deploy fails with "Could not retrieve Project Settings", which reads
# like a broken link but is really a scope error. `VERCEL_TOKEN_TODD` can see it.
# So: prefer VERCEL_TOKEN_TODD, and always pass --scope explicitly.
VERCEL_SCOPE="${VERCEL_SCOPE:-team_WGP9MIPM09U2lDW7YDqVIDsK}"
if [ -z "${VERCEL_TOKEN_TODD:-}" ] && [ -f "$HOME/clawd/.env" ]; then
  VERCEL_TOKEN_TODD=$(grep -E '^VERCEL_TOKEN_TODD=' "$HOME/clawd/.env" | head -1 \
    | sed -E 's/^VERCEL_TOKEN_TODD=//; s/^"//; s/"$//; s/^'\''//; s/'\''$//')
fi
if [ -z "${VERCEL_TOKEN:-}" ] && [ -f "$HOME/clawd/.env" ]; then
  VERCEL_TOKEN=$(grep -E '^VERCEL_TOKEN=' "$HOME/clawd/.env" | head -1 \
    | sed -E 's/^VERCEL_TOKEN=//; s/^"//; s/"$//; s/^'\''//; s/'\''$//')
fi
# VERCEL_TOKEN_TODD wins — it is the only one that can reach funbutton.ai.
DEPLOY_TOKEN="${VERCEL_TOKEN_TODD:-${VERCEL_TOKEN:-}}"
if [ -z "$DEPLOY_TOKEN" ]; then
  echo "!! No Vercel token (looked for VERCEL_TOKEN_TODD then VERCEL_TOKEN in \$HOME/clawd/.env) — cannot deploy" >&2
  exit 1
fi
VERCEL_BIN="vercel"
command -v vercel >/dev/null 2>&1 || VERCEL_BIN="npx --yes vercel"
echo "deploying apps/web to prod via: $VERCEL_BIN"
VLOG=$(mktemp)
if ! ( cd "$WEB" && $VERCEL_BIN --prod --yes --token "$DEPLOY_TOKEN" --scope "$VERCEL_SCOPE" ) >"$VLOG" 2>&1; then
  echo "!! vercel deploy FAILED:" >&2
  cat "$VLOG" >&2
  exit 1
fi
tail -5 "$VLOG"
DEPLOY_URL=$(grep -Eo 'https://[a-zA-Z0-9._/-]+\.vercel\.app' "$VLOG" | tail -1 || true)
echo "deployed: ${DEPLOY_URL:-<no url parsed — see output above>}"

# 6. Record state.
mkdir -p "$(dirname "$STATE")"
cat > "$STATE" <<EOF
{"last_updated":"$(date -u +%Y-%m-%dT%H:%M:%SZ)","version":"$LATEST","dmg":"$DMG","deploy":"${DEPLOY_URL:-}"}
EOF

# 7. Verify live: the version label is present and /download does not 404.
echo "verifying https://funbutton.ai ..."
sleep 5
LIVE_VER=$(curl -fsSL https://funbutton.ai 2>/dev/null | grep -oE 'v[0-9]+\.[0-9]+\.[0-9]+-alpha' | head -1 || true)
echo "  live version label: ${LIVE_VER:-<none found>}"
# No -L here: we want OUR /download route's own status (a 302 redirect), not to
# follow through and download the ~1.1 GB asset.
DL_CODE=$(curl -s -o /dev/null -w '%{http_code}' https://funbutton.ai/download || echo 000)
echo "  https://funbutton.ai/download -> HTTP $DL_CODE (expect 3xx redirect)"

echo "funbutton.ai updated to $LATEST"

#!/bin/bash
# update-landing-version.sh — keep funbutton.ai pointing at the latest GitHub release.
# Run after any new release tag, or let the cron run it periodically.
set -euo pipefail

REPO="$HOME/src/Github/funbutton"
PAGE="$REPO/apps/web/app/page.tsx"
STATE="$HOME/clawd/memory/funbutton-landing-state.json"

cd "$REPO"

# 1. Get the latest release tag from GitHub
LATEST=$(gh release view --json tagName -q .tagName 2>/dev/null || true)
[ -z "$LATEST" ] && { echo "no release found"; exit 0; }

# 2. What version is the landing page currently showing?
CURRENT=$(grep -oE 'v[0-9]+\.[0-9]+\.[0-9]+-alpha' "$PAGE" | head -1 || true)
[ "$CURRENT" = "${LATEST}-alpha" ] && { echo "already current: $CURRENT"; exit 0; }

echo "updating landing page: $CURRENT -> ${LATEST}-alpha"

# 3. Derive the DMG filename from the release assets
DMG=$(gh release view "$LATEST" --json assets -q '.assets[].name' 2>/dev/null | grep '\.dmg$' | head -1)
[ -z "$DMG" ] && { echo "no .dmg in $LATEST — skipping"; exit 0; }

DMG_URL="https://github.com/todddickerson/funbutton/releases/latest/download/$DMG"

# 4. Update version string + all DMG links in page.tsx
sed -i '' "s|v[0-9]*\.[0-9]*\.[0-9]*-alpha|${LATEST}-alpha|g" "$PAGE"
sed -i '' "s|releases/latest/download/[^\"']*|releases/latest/download/$DMG|g" "$PAGE"
sed -i '' "s|releases/tag/v[0-9]*\.[0-9]*\.[0-9]*|releases/tag/$LATEST|g" "$PAGE"

# 5. Commit, push, deploy
git add apps/web/app/page.tsx
git diff --cached --quiet && { echo "no changes to commit"; exit 0; }
git commit -q -m "chore(web): funbutton.ai -> $LATEST alpha download"
git push -q origin main

source "$HOME/clawd/.env" 2>/dev/null || true
cd apps/web
vercel --prod --yes --token "$VERCEL_TOKEN" 2>&1 | tail -3

# 6. Record state
mkdir -p "$(dirname "$STATE")"
cat > "$STATE" <<EOF
{"last_updated":"$(date -u +%Y-%m-%dT%H:%M:%SZ)","version":"$LATEST","dmg":"$DMG"}
EOF

echo "funbutton.ai updated to $LATEST"

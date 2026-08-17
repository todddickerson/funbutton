#!/usr/bin/env bash
#
# sync-install-sh.sh — regenerate the web copy of the curl installer.
#
# scripts/install.sh is the single source of truth. The landing page serves it
# verbatim at https://funbutton.ai/install.sh via a route handler that imports
# apps/web/app/install.sh/installer.json. That JSON is generated from this script
# so the two can never drift. Run this whenever you edit scripts/install.sh:
#
#   scripts/sync-install-sh.sh
#
# It's idempotent and prints whether anything changed.
set -euo pipefail

REPO_ROOT="$(cd "$(dirname "$0")/.." && pwd)"
SRC="$REPO_ROOT/scripts/install.sh"
OUT="$REPO_ROOT/apps/web/app/install.sh/installer.json"

[ -f "$SRC" ] || { echo "!! $SRC not found" >&2; exit 1; }
mkdir -p "$(dirname "$OUT")"

node -e '
  const fs = require("fs");
  const [src, out] = process.argv.slice(1);
  const script = fs.readFileSync(src, "utf8");
  const next = JSON.stringify({ script }) + "\n";
  const prev = fs.existsSync(out) ? fs.readFileSync(out, "utf8") : "";
  fs.writeFileSync(out, next);
  console.log(next === prev ? "installer.json already in sync" : "installer.json regenerated from scripts/install.sh");
' "$SRC" "$OUT"

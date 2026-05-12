#!/usr/bin/env bash
# One-shot DNS migration: funbutton.ai zone from Spaceship → Cloudflare,
# attach api.funbutton.ai as the Worker custom hostname, preserve Vercel
# landing page traffic.
#
# Prerequisites:
#   - CLOUDFLARE_API_TOKEN with: Account:Zones:Edit + Zone:Zones:Edit + Zone:DNS:Edit
#     (the Workers-only token in ~/clawd/.env lacks Zones:Edit — mint a new one
#      at dash.cloudflare.com → My Profile → API Tokens → "Edit zone DNS" template
#      and add "Account → Zones → Edit" to its scope.)
#   - SPACESHIP_API_KEY + SPACESHIP_API_SECRET in env or ~/clawd/.env.
#   - CF_ACCOUNT_ID defaults to Spontent LLC (e03523c149209369c46ebc10b8a30b43).
#
# Usage:
#   export CLOUDFLARE_API_TOKEN=<token-with-zones-edit>
#   bash apps/worker/scripts/setup-dns.sh
#
# The script is idempotent — re-running after a partial failure is safe.

set -euo pipefail

DOMAIN="funbutton.ai"
CF_ACCOUNT_ID="${CF_ACCOUNT_ID:-e03523c149209369c46ebc10b8a30b43}"
WORKER_NAME="${WORKER_NAME:-funbutton-api}"
VERCEL_IP="76.76.21.21"

# Load creds from ~/clawd/.env if not already exported.
if [[ -z "${SPACESHIP_API_KEY:-}" ]] && [[ -f "$HOME/clawd/.env" ]]; then
  SPACESHIP_API_KEY=$(grep '^SPACESHIP_API_KEY' "$HOME/clawd/.env" | cut -d= -f2-)
  SPACESHIP_API_SECRET=$(grep '^SPACESHIP_API_SECRET' "$HOME/clawd/.env" | cut -d= -f2-)
fi
: "${CLOUDFLARE_API_TOKEN:?need CLOUDFLARE_API_TOKEN with Zones:Edit scope}"
: "${SPACESHIP_API_KEY:?need SPACESHIP_API_KEY}"
: "${SPACESHIP_API_SECRET:?need SPACESHIP_API_SECRET}"

CF_API="https://api.cloudflare.com/client/v4"
SS_API="https://spaceship.dev/api/v1"

cf() {
  local method="$1" path="$2"
  shift 2
  curl -s -X "$method" "$CF_API$path" \
    -H "Authorization: Bearer $CLOUDFLARE_API_TOKEN" \
    -H "Content-Type: application/json" "$@"
}

ss() {
  local method="$1" path="$2"
  shift 2
  curl -s -X "$method" "$SS_API$path" \
    -H "X-API-Key: $SPACESHIP_API_KEY" \
    -H "X-API-Secret: $SPACESHIP_API_SECRET" \
    -H "Content-Type: application/json" "$@"
}

require_jq() {
  if ! command -v jq >/dev/null; then
    echo "❌ jq is required. brew install jq" >&2
    exit 1
  fi
}
require_jq

echo "Step 1/6 — Ensure $DOMAIN zone exists on CF account $CF_ACCOUNT_ID"
ZONE_JSON=$(cf GET "/zones?name=$DOMAIN&account.id=$CF_ACCOUNT_ID")
ZONE_ID=$(echo "$ZONE_JSON" | jq -r '.result[0].id // empty')

if [[ -z "$ZONE_ID" ]]; then
  echo "  → creating zone…"
  CREATE_RES=$(cf POST "/zones" --data "{\"name\":\"$DOMAIN\",\"account\":{\"id\":\"$CF_ACCOUNT_ID\"},\"type\":\"full\"}")
  if [[ "$(echo "$CREATE_RES" | jq -r '.success')" != "true" ]]; then
    echo "❌ zone create failed:" >&2
    echo "$CREATE_RES" | jq . >&2
    exit 1
  fi
  ZONE_ID=$(echo "$CREATE_RES" | jq -r '.result.id')
  CF_NS=$(echo "$CREATE_RES" | jq -r '.result.name_servers[]')
else
  echo "  → zone already exists ($ZONE_ID)"
  CF_NS=$(echo "$ZONE_JSON" | jq -r '.result[0].name_servers[]')
fi
echo "  → CF nameservers:"
echo "$CF_NS" | sed 's/^/      /'

echo "Step 2/6 — Pre-create existing Vercel DNS records on CF (DNS-only / gray-cloud)"
ensure_record() {
  local type="$1" name="$2" content="$3" proxied="$4"
  local existing
  existing=$(cf GET "/zones/$ZONE_ID/dns_records?type=$type&name=$name")
  local id
  id=$(echo "$existing" | jq -r '.result[0].id // empty')
  local payload
  payload=$(jq -nc \
    --arg t "$type" --arg n "$name" --arg c "$content" --argjson p "$proxied" \
    '{type:$t, name:$n, content:$c, ttl:3600, proxied:$p}')
  if [[ -z "$id" ]]; then
    cf POST "/zones/$ZONE_ID/dns_records" --data "$payload" >/dev/null
    echo "  + $type $name → $content (proxied=$proxied)"
  else
    cf PUT "/zones/$ZONE_ID/dns_records/$id" --data "$payload" >/dev/null
    echo "  ~ $type $name → $content (proxied=$proxied)"
  fi
}
ensure_record A "$DOMAIN" "$VERCEL_IP" false
ensure_record A "www.$DOMAIN" "$VERCEL_IP" false

echo "Step 3/6 — Update Spaceship nameservers to Cloudflare"
NS_ITEMS=$(echo "$CF_NS" | jq -R . | jq -s .)
SS_PAYLOAD=$(jq -nc --argjson ns "$NS_ITEMS" '{nameservers: $ns}')
SS_RES=$(ss PUT "/dns/nameservers/$DOMAIN" --data "$SS_PAYLOAD" -o /dev/null -w "%{http_code}\n" || true)
if [[ "$SS_RES" == "204" || "$SS_RES" == "200" ]]; then
  echo "  → Spaceship NS update accepted (HTTP $SS_RES)"
else
  echo "  ⚠ Spaceship NS update returned HTTP $SS_RES — verify in Spaceship UI"
fi

echo "Step 4/6 — Wait for CF to mark zone active (max 10 min, poll every 30s)"
for i in $(seq 1 20); do
  STATUS=$(cf GET "/zones/$ZONE_ID" | jq -r '.result.status')
  echo "    poll $i/20: $STATUS"
  if [[ "$STATUS" == "active" ]]; then
    break
  fi
  sleep 30
done
if [[ "$STATUS" != "active" ]]; then
  echo "  ⚠ zone still '$STATUS' after 10 min — DNS propagation can take up to 24h."
  echo "  ⚠ Continue manually once active: api.funbutton.ai custom domain attach + wrangler deploy."
  exit 0
fi

echo "Step 5/6 — Attach api.$DOMAIN as Worker custom domain"
DOMAIN_LIST=$(cf GET "/accounts/$CF_ACCOUNT_ID/workers/domains?hostname=api.$DOMAIN")
EXISTING=$(echo "$DOMAIN_LIST" | jq -r '.result[0].id // empty')
if [[ -z "$EXISTING" ]]; then
  cf POST "/accounts/$CF_ACCOUNT_ID/workers/domains" \
    --data "{\"zone_id\":\"$ZONE_ID\",\"hostname\":\"api.$DOMAIN\",\"service\":\"$WORKER_NAME\",\"environment\":\"production\"}" \
    | jq -r '.success, .errors' >&2 || true
  echo "  + attached api.$DOMAIN → $WORKER_NAME (production)"
else
  echo "  ~ api.$DOMAIN custom domain already attached ($EXISTING)"
fi

echo "Step 6/6 — Smoke test"
sleep 10
HTTP=$(curl -sI -o /dev/null -w "%{http_code}" "https://api.$DOMAIN/health" || true)
echo "  → GET https://api.$DOMAIN/health → HTTP $HTTP"
if [[ "$HTTP" == "200" ]]; then
  echo "✅ Done. Re-enable the route in apps/worker/wrangler.toml and 'wrangler deploy --env production'."
else
  echo "⚠ Health check returned $HTTP — cert may still be provisioning (up to ~15 min)."
fi

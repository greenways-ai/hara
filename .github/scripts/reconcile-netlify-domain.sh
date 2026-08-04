#!/usr/bin/env bash

set -euo pipefail

: "${NETLIFY_AUTH_TOKEN:?NETLIFY_AUTH_TOKEN is required}"
: "${NETLIFY_SITE_ID:?NETLIFY_SITE_ID is required}"
: "${NETLIFY_CUSTOM_DOMAIN:?NETLIFY_CUSTOM_DOMAIN is required}"

api="https://api.netlify.com/api/v1/sites/${NETLIFY_SITE_ID}"
auth="Authorization: Bearer ${NETLIFY_AUTH_TOKEN}"
site="$(curl --fail --silent --show-error -H "$auth" "$api")"

aliases="$(
  jq -c \
    --arg primary "$NETLIFY_CUSTOM_DOMAIN" \
    --arg extras "${NETLIFY_DOMAIN_ALIASES:-}" '
      [(.domain_aliases // [])[] | select(endswith(".hara-long.org") | not)]
      + [$primary]
      + ($extras | split(" ") | map(select(length > 0)))
      | unique
    ' <<<"$site"
)"

curl --fail --silent --show-error \
  -X PATCH \
  -H "$auth" \
  -H 'Content-Type: application/json' \
  --data "$(jq -nc --argjson aliases "$aliases" '{domain_aliases: $aliases}')" \
  "$api" >/dev/null

if [[ "$(jq -r '.custom_domain // ""' <<<"$site")" != "$NETLIFY_CUSTOM_DOMAIN" ]]; then
  response="$(mktemp)"
  for attempt in 1 2 3 4; do
    status="$({ curl --silent --show-error \
      -o "$response" \
      -w '%{http_code}' \
      -X PATCH \
      -H "$auth" \
      -H 'Content-Type: application/json' \
      --data "$(jq -nc --arg domain "$NETLIFY_CUSTOM_DOMAIN" '{custom_domain: $domain}')" \
      "$api"; } || true)"
    if [[ "$status" =~ ^2 ]]; then
      break
    fi
    if [[ "$status" != "422" ]] || ! grep -qi 'provisioning a certificate' "$response"; then
      cat "$response" >&2
      exit 1
    fi
    if [[ "$attempt" -lt 4 ]]; then
      echo "Netlify is finishing an older certificate request; retrying domain update."
      sleep 15
    fi
  done
fi

curl --silent --show-error -X POST -H "$auth" "$api/ssl" >/dev/null || true

site="$(curl --fail --silent --show-error -H "$auth" "$api")"
while read -r domain; do
  [[ -z "$domain" ]] && continue
  if ! jq -e --arg domain "$domain" '
    .custom_domain == $domain or ((.domain_aliases // []) | index($domain) != null)
  ' <<<"$site" >/dev/null; then
    echo "Netlify did not retain ${domain}." >&2
    exit 1
  fi
done < <(printf '%s\n' "$NETLIFY_CUSTOM_DOMAIN" ${NETLIFY_DOMAIN_ALIASES:-})

echo "Netlify now routes ${NETLIFY_CUSTOM_DOMAIN}."

for url in ${NETLIFY_HEALTH_URLS:-}; do
  page="$(mktemp)"
  healthy=false
  for attempt in {1..20}; do
    if curl --fail --silent --show-error --location --max-time 20 "$url" >"$page" \
      && grep -q 'property="og:image"' "$page" \
      && grep -q 'property="og:image:width" content="3840"' "$page"; then
      healthy=true
      echo "Verified 3840px OG metadata at ${url}."
      break
    fi
    if [[ "$attempt" -lt 20 ]]; then
      echo "Waiting for ${url} to become healthy."
      sleep 15
    fi
  done
  if [[ "$healthy" != true ]]; then
    echo "${url} did not publish the expected OG metadata." >&2
    exit 1
  fi
done

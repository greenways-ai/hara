#!/usr/bin/env bash

set -euo pipefail

: "${HARA_SITE_ORIGIN:?HARA_SITE_ORIGIN is required}"
: "${HARA_IDENTITY_ORIGIN:?HARA_IDENTITY_ORIGIN is required}"

site_origin="${HARA_SITE_ORIGIN%/}"
identity_origin="${HARA_IDENTITY_ORIGIN%/}"
tmpdir="$(mktemp -d)"
trap 'rm -rf "$tmpdir"' EXIT

header_value() {
  local name="$1"
  local file="$2"
  awk -v target="${name,,}" '
    BEGIN { IGNORECASE = 1 }
    {
      key = $1
      sub(/:$/, "", key)
      if (tolower(key) == target) {
        $1 = ""
        sub(/^[[:space:]]+/, "")
        sub(/\r$/, "")
        print
        exit
      }
    }
  ' "$file"
}

fetch_page() {
  local path="$1"
  local output="$2"
  local attempt
  for attempt in {1..20}; do
    if curl --fail --silent --show-error --location \
      --connect-timeout 10 \
      --max-time 20 \
      --header 'Cache-Control: no-cache' \
      "$site_origin$path" >"$output"; then
      return 0
    fi
    if [[ "$attempt" -eq 20 ]]; then
      echo "${site_origin}${path} did not become available." >&2
      return 1
    fi
    sleep 5
  done
}

home="$tmpdir/home.html"
docs="$tmpdir/docs.html"
fetch_page "/" "$home"
fetch_page "/docs/start/orientation/" "$docs"

for marker in 'hara-identity-mode' 'content="popup"' 'data-hara-identity' 'data-hara-identity-fallback' '/assets/identity-loader.js'; do
  if ! grep -Fq "$marker" "$home"; then
    echo "${site_origin}/ is missing the popup identity marker: ${marker}" >&2
    exit 1
  fi
done
for marker in 'hara-identity-auto' '/assets/identity-loader.js'; do
  if ! grep -Fq "$marker" "$docs"; then
    echo "${site_origin}/docs/start/orientation/ is missing the shared identity marker: ${marker}" >&2
    exit 1
  fi
done

discovery="$tmpdir/discovery.json"
curl --fail --silent --show-error --location \
  --connect-timeout 10 \
  --max-time 20 \
  "$identity_origin/.well-known/hara-session" >"$discovery"

if ! jq -e \
  --arg identity "$identity_origin" \
  --arg site "$site_origin" '
    .contractVersion == 1
    and .clientVersion == 1
    and .issuer == $identity
    and .clientEndpoint == ($identity + "/v1/identity-client.js")
    and .sessionEndpoint == ($identity + "/session")
    and .logoutEndpoint == ($identity + "/logout")
    and .configured == true
    and (.allowedOrigins | index($site) != null)
  ' "$discovery" >/dev/null; then
  echo "The central Identity contract is not ready for ${site_origin}." >&2
  cat "$discovery" >&2
  exit 1
fi

session_body="$tmpdir/session.json"
session_headers="$tmpdir/session.headers"
session_status="$(curl --silent --show-error \
  --connect-timeout 10 \
  --max-time 20 \
  --header "Origin: ${site_origin}" \
  --dump-header "$session_headers" \
  --output "$session_body" \
  --write-out '%{http_code}' \
  "$identity_origin/session")"

if [[ "$session_status" != "200" ]]; then
  echo "Expected central session discovery to return HTTP 200, got ${session_status}." >&2
  cat "$session_body" >&2 || true
  exit 1
fi
if [[ "$(header_value Access-Control-Allow-Origin "$session_headers")" != "$site_origin" ]]; then
  echo "Identity did not return exact CORS for ${site_origin}." >&2
  exit 1
fi
if [[ "$(header_value Access-Control-Allow-Credentials "$session_headers")" != "true" ]]; then
  echo "Identity did not enable credentialed session reads for ${site_origin}." >&2
  exit 1
fi
if ! jq -e '
  .authenticated == false
  and .configured == true
  and .issuer == "hara-id"
  and .profile == null
  and .identity == null
' "$session_body" >/dev/null; then
  echo "The unauthenticated session response does not match the shared identity contract." >&2
  cat "$session_body" >&2
  exit 1
fi

untrusted_headers="$tmpdir/untrusted.headers"
untrusted_status="$(curl --silent --show-error \
  --connect-timeout 10 \
  --max-time 20 \
  --header 'Origin: https://untrusted.example' \
  --dump-header "$untrusted_headers" \
  --output /dev/null \
  --write-out '%{http_code}' \
  "$identity_origin/session")"
if [[ "$untrusted_status" != "403" ]]; then
  echo "Expected an untrusted origin to receive HTTP 403, got ${untrusted_status}." >&2
  exit 1
fi
if grep -qi '^access-control-allow-origin:' "$untrusted_headers"; then
  echo "Identity exposed CORS to an untrusted origin." >&2
  exit 1
fi

client="$tmpdir/identity-client.js"
curl --fail --silent --show-error --location \
  --connect-timeout 10 \
  --max-time 20 \
  "$identity_origin/v1/identity-client.js" >"$client"
if ! grep -q 'HaraIdentity' "$client"; then
  echo "The versioned shared identity client is unavailable." >&2
  exit 1
fi
if ! grep -q 'hara_identity_popup' "$client"; then
  echo "The versioned shared identity client does not support popup completion." >&2
  exit 1
fi

echo "Verified popup Hara identity contract v1 for www and Docs at ${site_origin}."

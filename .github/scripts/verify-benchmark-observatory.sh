#!/usr/bin/env bash

set -euo pipefail

base="${1:?usage: verify-benchmark-observatory.sh URL}"
base="${base%/}"
page="$(mktemp)"
catalog="$(mktemp)"
runs="$(mktemp)"
trap 'rm -f "$page" "$catalog" "$runs"' EXIT

healthy=false
for attempt in {1..20}; do
  if curl --fail --silent --show-error --location --max-time 20 \
      -H 'Cache-Control: no-cache' "$base/" >"$page" \
    && curl --fail --silent --show-error --location --max-time 20 \
      -H 'Cache-Control: no-cache' "$base/data/catalog.json" >"$catalog" \
    && curl --fail --silent --show-error --location --max-time 20 \
      -H 'Cache-Control: no-cache' "$base/data/runs.json" >"$runs" \
    && grep -Fq '<title>Hara Benchmarks</title>' "$page" \
    && grep -Fq 'id="class-comparison"' "$page" \
    && grep -Fq 'id="language-shootout"' "$page" \
    && grep -Fq 'rust-prepared' "$page" \
    && grep -Fq '"rust"' "$catalog" \
    && grep -Fq '"runtime":"rust-prepared"' "$runs"; then
    healthy=true
    echo "Verified the Rust-enabled Astro benchmark site and canonical evidence at ${base}/."
    break
  fi
  if [[ "$attempt" -lt 20 ]]; then
    echo "Waiting for the Astro benchmark site at ${base}/."
    sleep 15
  fi
done

if [[ "$healthy" != true ]]; then
  echo "${base}/ did not expose the Rust-enabled Astro benchmark site and evidence." >&2
  exit 1
fi

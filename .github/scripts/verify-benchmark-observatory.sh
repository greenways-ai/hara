#!/usr/bin/env bash

set -euo pipefail

base="${1:?usage: verify-benchmark-observatory.sh URL}"
base="${base%/}"
page="$(mktemp)"
catalog="$(mktemp)"
runs="$(mktemp)"
headers="$(mktemp)"
trap 'rm -f "$page" "$catalog" "$runs" "$headers"' EXIT

healthy=false
for attempt in {1..20}; do
  if curl --fail --silent --show-error --location --max-time 20 \
      --dump-header "$headers" "$base/" >"$page" \
    && curl --fail --silent --show-error --location --max-time 20 \
      "$base/data/catalog.json" >"$catalog" \
    && curl --fail --silent --show-error --location --max-time 20 \
      "$base/data/runs.json" >"$runs" \
    && grep -Fq '<title>Hara Benchmarks</title>' "$page" \
    && grep -Fq 'id="class-comparison"' "$page" \
    && grep -Fq 'id="language-shootout"' "$page" \
    && grep -Fq 'rust-prepared' "$page" \
    && ! grep -Fq 'Hara Performance Observatory' "$page" \
    && grep -Fiq 'cache-control: no-store' "$headers" \
    && grep -Fq '"rust"' "$catalog" \
    && grep -Fq '"runtime":"rust-prepared"' "$runs"; then
    healthy=true
    echo "Verified the uncached Rust-enabled Astro benchmark site and canonical evidence at ${base}/."
    break
  fi
  if [[ "$attempt" -lt 20 ]]; then
    echo "Waiting for the Astro benchmark site at ${base}/."
    sleep 15
  fi
done

if [[ "$healthy" != true ]]; then
  echo "${base}/ did not expose the uncached Rust-enabled Astro benchmark site and evidence." >&2
  echo "Response markers:" >&2
  grep -E '<title>|Hara Performance Observatory|class-comparison|language-shootout' "$page" | head -n 12 >&2 || true
  echo "Response headers:" >&2
  sed -n '1,30p' "$headers" >&2 || true
  exit 1
fi

#!/usr/bin/env bash
# OpenResty vs Hoplite HTTP benchmark.
# Starts each embedded Nginx sequentially, warms up, measures with ab,
# and writes target/hoplite-openresty-benchmark.md.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/../../.." && pwd)"
HERE="$ROOT/lib/bench/hoplite-openresty"
WORK="$ROOT/target/bench-hoplite-openresty"
REPORT="$ROOT/target/hoplite-openresty-benchmark.md"
HOPLITE_NGINX="${HOPLITE_NGINX:-$ROOT/target/hoplite/nginx/sbin/nginx}"
OPENRESTY_BIN="${OPENRESTY_BIN:-$WORK/openresty-install/nginx/sbin/nginx}"

REQUESTS="${REQUESTS:-20000}"
CONCURRENCY="${CONCURRENCY:-32}"
WARMUP="${WARMUP:-500}"

die() { echo "error: $*" >&2; exit 1; }

[ -x "$HOPLITE_NGINX" ] || die "hoplite nginx not found at $HOPLITE_NGINX"
[ -x "$OPENRESTY_BIN" ] || die "openresty not found at $OPENRESTY_BIN (build it or set OPENRESTY_BIN)"
command -v ab >/dev/null || die "ab not found"
command -v curl >/dev/null || die "curl not found"

stop_servers() {
  [ -f "$WORK/hoplite/logs/nginx.pid" ] && "$HOPLITE_NGINX" -p "$WORK/hoplite" -c "$WORK/hoplite/nginx.conf" -s stop 2>/dev/null || true
  [ -f "$WORK/openresty/logs/nginx.pid" ] && "$OPENRESTY_BIN" -p "$WORK/openresty" -c "$WORK/openresty/nginx.conf" -s stop 2>/dev/null || true
}
trap stop_servers EXIT

render() { # <template> <target-dir>
  rm -rf "$2"
  mkdir -p "$2/logs" "$2/client_body_temp" "$2/proxy_temp" "$2/fastcgi_temp" "$2/uwsgi_temp" "$2/scgi_temp"
  sed "s|@@ROOT@@|$ROOT|g" "$1" > "$2/nginx.conf"
}

# bench <label> <base-url> <path> <expected-body-prefix>
bench() {
  local label="$1" base="$2" path="$3" expect="$4"
  local body
  body="$(curl -fsS -m 10 "$base$path")" || die "$label: $path not reachable"
  case "$body" in
    "$expect"*) : ;;
    *) die "$label: $path returned unexpected body: $body" ;;
  esac
  ab -n "$WARMUP" -c "$CONCURRENCY" "$base$path" > /dev/null 2>&1
  ab -k -n "$REQUESTS" -c "$CONCURRENCY" "$base$path"
}

parse() { # <ab-output-file> -> rps mean_ms p99_ms
  awk '
    /^Requests per second:/ { rps = $4 }
    /^Time per request:/ && /mean)$/ && !seen { mean = $4; seen = 1 }
    /^ *99%/ { p99 = $2 }
    END { printf "%s %s %s\n", rps, mean, p99 }
  ' "$1"
}

echo "== hoplite =="
render "$HERE/nginx.hoplite.conf.tmpl" "$WORK/hoplite"
"$HOPLITE_NGINX" -p "$WORK/hoplite" -c "$WORK/hoplite/nginx.conf" -t >/dev/null
"$HOPLITE_NGINX" -p "$WORK/hoplite" -c "$WORK/hoplite/nginx.conf"
sleep 1
for route in hello delay; do
  bench hoplite http://127.0.0.1:18081 "/$route" "$( [ "$route" = hello ] && echo 'Hello from Hoplite' || echo 'delayed 25ms')" \
    > "$WORK/hoplite-$route.ab"
  echo "hoplite /$route: $(parse "$WORK/hoplite-$route.ab")"
done
"$HOPLITE_NGINX" -p "$WORK/hoplite" -c "$WORK/hoplite/nginx.conf" -s stop
sleep 1

echo "== openresty =="
render "$HERE/nginx.openresty.conf.tmpl" "$WORK/openresty"
"$OPENRESTY_BIN" -p "$WORK/openresty" -c "$WORK/openresty/nginx.conf" -t >/dev/null
"$OPENRESTY_BIN" -p "$WORK/openresty" -c "$WORK/openresty/nginx.conf"
sleep 1
for route in hello delay; do
  bench openresty http://127.0.0.1:18082 "/$route" "$( [ "$route" = hello ] && echo 'Hello from OpenResty' || echo 'delayed 25ms')" \
    > "$WORK/openresty-$route.ab"
  echo "openresty /$route: $(parse "$WORK/openresty-$route.ab")"
done
"$OPENRESTY_BIN" -p "$WORK/openresty" -c "$WORK/openresty/nginx.conf" -s stop

{
  echo "# OpenResty vs Hoplite HTTP benchmark"
  echo
  echo "Generated: $(date -u +%Y-%m-%dT%H:%M:%SZ) on $(uname -srm)."
  echo
  echo "ab -k -n $REQUESTS -c $CONCURRENCY per route, after a $WARMUP-request warmup."
  echo "Single Nginx worker per server; loopback only."
  echo
  echo '| Route | Server | Requests/sec | Mean ms | p99 ms |'
  echo '|---|---|---:|---:|---:|'
  for route in hello delay; do
    for server in hoplite openresty; do
      read -r rps mean p99 < <(parse "$WORK/$server-$route.ab")
      echo "| /$route | $server | $rps | $mean | $p99 |"
    done
  done
  echo
  echo "Versions: $("$OPENRESTY_BIN" -v 2>&1); hoplite nginx: $("$HOPLITE_NGINX" -v 2>&1); ab: $(ab -V 2>&1 | head -1)"
} > "$REPORT"
echo "wrote $REPORT"

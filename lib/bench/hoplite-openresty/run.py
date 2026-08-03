#!/usr/bin/env python3
"""Single-worker HTTP comparison for Hoplite and peer frameworks."""
from __future__ import annotations
import json, os, platform, re, shutil, statistics, subprocess, time, urllib.request
from datetime import datetime, timezone
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
HERE, WORK = Path(__file__).resolve().parent, ROOT / "target/bench-hoplite-openresty"
OUTPUT = ROOT / "target/hara-http-frameworks.json"
HOPLITE = Path(os.environ.get("HOPLITE_NGINX", ROOT / "target/hoplite/nginx/sbin/nginx"))
OPENRESTY = Path(os.environ.get("OPENRESTY_BIN", WORK / "openresty-install/nginx/sbin/nginx"))
NGINX = Path(os.environ.get("NGINX_BIN", shutil.which("nginx") or "/missing/nginx"))
REQUESTS, CONCURRENCY = int(os.environ.get("REQUESTS", "20000")), int(os.environ.get("CONCURRENCY", "32"))
WARMUP, TRIALS = int(os.environ.get("WARMUP", "500")), int(os.environ.get("TRIALS", "5"))

def run(command, **kwargs):
    kwargs.setdefault("check", True)
    return subprocess.run(command, text=True, **kwargs)

def revision(path):
    result = run(["git", "-C", str(path), "rev-parse", "HEAD"], capture_output=True)
    return result.stdout.strip()

def binary_version(binary):
    result = run([str(binary), "-V"], capture_output=True, check=False)
    return (result.stderr or result.stdout).splitlines()[0]

def render(template, target):
    target.mkdir(parents=True, exist_ok=True)
    for name in ("logs", "client_body_temp", "proxy_temp", "fastcgi_temp", "uwsgi_temp", "scgi_temp"):
        (target / name).mkdir(exist_ok=True)
    (target / "nginx.conf").write_text(template.read_text().replace("@@ROOT@@", str(ROOT)))

def wait_ready(url):
    for _ in range(100):
        try:
            if urllib.request.urlopen(url, timeout=.25).status == 200: return
        except Exception: time.sleep(.05)
    raise RuntimeError(f"server did not become ready: {url}")

def validate_route(url, route):
    with urllib.request.urlopen(url, timeout=2) as response:
        body = response.read().decode()
        content_type = response.headers.get_content_type()
    if route == "hello" and not (body.startswith("Hello from ") and content_type == "text/plain"):
        raise RuntimeError(f"invalid hello response from {url}: {content_type} {body!r}")
    if route == "json" and not (json.loads(body).get("message", "").startswith("Hello from ") and content_type == "application/json"):
        raise RuntimeError(f"invalid JSON response from {url}: {content_type} {body!r}")
    if route == "delay" and not (body == "delayed 25ms\n" and content_type == "text/plain"):
        raise RuntimeError(f"invalid delay response from {url}: {content_type} {body!r}")

def parse_ab(text):
    def value(pattern):
        match = re.search(pattern, text, re.MULTILINE)
        if not match: raise RuntimeError(f"ab output is missing {pattern}")
        return float(match.group(1))
    return {"requests_per_second": value(r"^Requests per second:\s+([0-9.]+)"),
            "mean_ms": value(r"^Time per request:\s+([0-9.]+).+\(mean\)$"),
            "p50_ms": value(r"^\s*50%\s+([0-9.]+)"),
            "p95_ms": value(r"^\s*95%\s+([0-9.]+)"),
            "p99_ms": value(r"^\s*99%\s+([0-9.]+)")}

def benchmark(label, base, routes):
    rows = []
    for route in routes:
        url = f"{base}/{route}"; wait_ready(url)
        validate_route(url, route)
        run(["ab", "-k", "-n", str(WARMUP), "-c", str(CONCURRENCY), url], stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)
        trials = [parse_ab(run(["ab", "-k", "-n", str(REQUESTS), "-c", str(CONCURRENCY), url], capture_output=True).stdout) for _ in range(TRIALS)]
        rows.append({"server": label, "route": f"/{route}", "status": "ok",
                     **{key: statistics.median(row[key] for row in trials) for key in trials[0]}})
    return rows

def nginx_server(label, binary, template, port, routes):
    target = WORK / label
    if target.exists(): shutil.rmtree(target)
    render(template, target); command = [str(binary), "-p", str(target), "-c", str(target / "nginx.conf")]
    run(command + ["-t"], stdout=subprocess.DEVNULL); run(command)
    try: return benchmark(label, f"http://127.0.0.1:{port}", routes)
    finally: run(command + ["-s", "stop"], check=False, stdout=subprocess.DEVNULL, stderr=subprocess.DEVNULL)

def process_server(label, command, port):
    process = subprocess.Popen(command, cwd=HERE, stdout=subprocess.DEVNULL, stderr=subprocess.PIPE, text=True)
    try: return benchmark(label, f"http://127.0.0.1:{port}", ["hello", "json", "delay"])
    finally:
        process.terminate()
        try: process.wait(timeout=5)
        except subprocess.TimeoutExpired: process.kill()

def markdown(data):
    lines = ["# Hara HTTP framework benchmark", "", "Single worker/event loop; loopback HTTP/1.1 with keep-alive.", "",
             "| Route | Server | Requests/sec | Mean ms | p50 ms | p95 ms | p99 ms |", "|---|---|---:|---:|---:|---:|---:|"]
    for row in data["measurements"]:
        if row["status"] != "ok": lines.append(f"| {row['route']} | {row['server']} | — | — | — | — | — |")
        else: lines.append(f"| {row['route']} | {row['server']} | {row['requests_per_second']:.1f} | {row['mean_ms']:.3f} | {row['p50_ms']:.3f} | {row['p95_ms']:.3f} | {row['p99_ms']:.3f} |")
    return "\n".join(lines) + "\n"

def main():
    for binary in (HOPLITE, OPENRESTY, NGINX):
        if not binary.is_file(): raise SystemExit(f"missing server binary: {binary}")
    if not (HERE / "node_modules/fastify").is_dir(): run(["npm", "install", "--ignore-scripts"], cwd=HERE)
    run(["cargo", "build", "--release", "--manifest-path", str(HERE / "axum/Cargo.toml")], cwd=ROOT)
    rows = nginx_server("hoplite-hara-rust-full", HOPLITE, HERE / "nginx.hoplite.conf.tmpl", 18081, ["hello", "json", "delay"])
    rows += nginx_server("openresty", OPENRESTY, HERE / "nginx.openresty.conf.tmpl", 18082, ["hello", "json", "delay"])
    rows += nginx_server("nginx", NGINX, HERE / "nginx.core.conf.tmpl", 18083, ["hello", "json"])
    rows += process_server("fastify", ["node", str(HERE / "fastify.mjs")], 18084)
    rows += process_server("axum", [str(HERE / "axum/target/release/hara-axum-benchmark")], 18085)
    rows.append({"server": "nginx", "route": "/delay", "status": "not_applicable", "reason": "core Nginx has no application timer"})
    data = {"schema_version": 1, "generated_at": datetime.now(timezone.utc).isoformat(),
            "environment": {"platform": platform.platform(), "machine": platform.machine(),
                            "hara_revision": revision(ROOT),
                            "hoplite_revision": revision(ROOT.parent / "hoplite"),
                            "hoplite_server": binary_version(HOPLITE),
                            "openresty_server": binary_version(OPENRESTY),
                            "nginx_server": binary_version(NGINX)},
            "configuration": {"requests": REQUESTS, "concurrency": CONCURRENCY, "warmup": WARMUP, "trials": TRIALS, "workers": 1}, "measurements": rows}
    OUTPUT.parent.mkdir(parents=True, exist_ok=True); OUTPUT.write_text(json.dumps(data, indent=2) + "\n")
    OUTPUT.with_suffix(".md").write_text(markdown(data)); print(f"wrote {OUTPUT} and {OUTPUT.with_suffix('.md')}")

if __name__ == "__main__": main()

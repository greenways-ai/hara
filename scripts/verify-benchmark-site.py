#!/usr/bin/env python3
"""Verify that a deployed Hara site serves the evidence-gated dashboard."""
from __future__ import annotations

import json
import re
import sys
import time
import urllib.error
import urllib.request
from typing import Any

CANONICAL_URL = "https://www.hara-lang.org/benchmarks/"
REQUIRED_RUNTIMES = {
    "hara-rust-whole-wasm-prepared",
    "luajit-prepared",
    "pypy-prepared",
    "node-prepared",
    "ruby-yjit-prepared",
    "clojure-prepared",
    "sbcl-prepared",
    "chez-prepared",
    "guile-prepared",
    "bb-prepared",
    "python-prepared",
    "c-prepared",
    "java-prepared",
}
MIN_SHARED_WORKLOADS = 6


def fetch(url: str, attempts: int = 12) -> bytes:
    last_error: Exception | None = None
    for attempt in range(1, attempts + 1):
        try:
            request = urllib.request.Request(
                url,
                headers={
                    "User-Agent": "hara-benchmark-deploy-verifier/1.0",
                    "Cache-Control": "no-cache",
                },
            )
            with urllib.request.urlopen(request, timeout=30) as response:
                if response.status != 200:
                    raise RuntimeError(f"{url} returned HTTP {response.status}")
                return response.read()
        except (OSError, RuntimeError, urllib.error.URLError) as error:
            last_error = error
            if attempt == attempts:
                break
            time.sleep(min(5 * attempt, 30))
    raise RuntimeError(f"unable to fetch {url}: {last_error}")


def usable_workloads(run: dict[str, Any], runtime: str) -> set[str]:
    return {
        str(row["workload"])
        for row in run.get("measurements", [])
        if row.get("runtime") == runtime
        and row.get("status") == "ok"
        and row.get("steady_state", {}).get("samples_ns")
        and row.get("workload")
    }


def verify(base_url: str) -> None:
    base = base_url.rstrip("/") + "/"
    html = fetch(base).decode("utf-8")
    required_html = (
        "<title>Hara Performance Observatory</title>",
        "Hara against dynamic JIT peers",
        f'<link rel="canonical" href="{CANONICAL_URL}">',
    )
    missing_html = [marker for marker in required_html if marker not in html]
    if missing_html:
        raise RuntimeError(
            "deployed benchmark page is missing: " + ", ".join(missing_html)
        )

    payload = json.loads(fetch(base + "data/runs.json"))
    runs = payload.get("runs") or []
    if not runs:
        raise RuntimeError("deployed benchmark dashboard contains no runs")
    run = max(
        runs,
        key=lambda item: str(item.get("environment", {}).get("timestamp", "")),
    )
    revision = str(run.get("environment", {}).get("benchmark_revision") or "")
    if not re.fullmatch(r"[0-9a-f]{40}", revision):
        raise RuntimeError("latest deployed run lacks an immutable revision")

    workloads = {
        runtime: usable_workloads(run, runtime) for runtime in REQUIRED_RUNTIMES
    }
    missing = sorted(
        runtime
        for runtime, values in workloads.items()
        if len(values) < MIN_SHARED_WORKLOADS
    )
    if missing:
        raise RuntimeError(
            "deployed dashboard lacks verified runtime coverage for: "
            + ", ".join(missing)
        )

    hara = workloads["hara-rust-whole-wasm-prepared"]
    insufficient = sorted(
        runtime
        for runtime, values in workloads.items()
        if runtime != "hara-rust-whole-wasm-prepared"
        and len(hara & values) < MIN_SHARED_WORKLOADS
    )
    if insufficient:
        raise RuntimeError(
            "deployed dashboard lacks shared Hara comparisons for: "
            + ", ".join(insufficient)
        )

    print(
        f"verified {base}: Hara Performance Observatory, "
        f"{len(REQUIRED_RUNTIMES)} runtime lanes, benchmark revision {revision}"
    )


def main() -> int:
    if len(sys.argv) != 2:
        print(f"usage: {sys.argv[0]} BASE_URL", file=sys.stderr)
        return 2
    verify(sys.argv[1])
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

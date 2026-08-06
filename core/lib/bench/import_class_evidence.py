#!/usr/bin/env python3
"""Import the verified hara-benchmarks class run as lib/bench evidence.

The hara-benchmarks repository publishes verified comparison runs (dynamic-JIT
peers, Lisp family and native/managed references) on its ``benchmark-site``
branch. This importer converts the latest run into the lib/bench language
evidence shape consumed by the Astro benchmarks site, keeping the run
self-consistent: ratios are always computed against the run's own Hara
baseline (``hara-rust-whole-wasm-prepared``), never against evidence captured
in a different environment.

Usage:

    python lib/bench/import_class_evidence.py [source]

``source`` defaults to the published dashboard bundle and may be a local path
to a runs.json document for offline imports.
"""
from __future__ import annotations

import json
import statistics
import sys
import urllib.request
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
RESULT = ROOT / "lib/bench/results/class-reference.json"
DEFAULT_SOURCE = (
    "https://raw.githubusercontent.com/hara-lang/hara-benchmarks/"
    "benchmark-site/data/runs.json"
)
HARA_RUNTIME = "hara-rust-whole-wasm-prepared"


def load(source: str) -> dict:
    if source.startswith("https://") or source.startswith("http://"):
        request = urllib.request.Request(source, headers={"User-Agent": "hara-class-evidence-importer/1.0"})
        with urllib.request.urlopen(request, timeout=60) as response:
            return json.loads(response.read())
    return json.loads(Path(source).read_text(encoding="utf-8"))


def latest_run(document: dict) -> dict:
    runs = document.get("runs")
    if not runs:
        raise SystemExit("source document contains no runs")
    return max(runs, key=lambda run: run["environment"]["timestamp"])


def convert(run: dict) -> dict:
    measurements = []
    for row in run["measurements"]:
        converted = {
            "runtime": row["runtime"],
            "workload": row["workload"],
            "status": row["status"],
        }
        if row.get("first_call_ns") is not None:
            converted["first_ns"] = row["first_call_ns"]
        if row.get("prepare_ns") is not None:
            converted["prepare_ns"] = row["prepare_ns"]
        samples = (row.get("steady_state") or {}).get("samples_ns") or []
        if row["status"] == "ok" and samples:
            converted["samples_ns"] = samples
            converted["analysis"] = {"steady_ns": statistics.median(samples)}
        if row.get("reason"):
            converted["reason"] = row["reason"]
        measurements.append(converted)
    runtimes = sorted({row["runtime"] for row in measurements if row["runtime"] != HARA_RUNTIME})
    environment = run.get("environment", {})
    return {
        "schema_version": 2,
        "profile": "class-reference",
        "corpus": "hara-benchmarks verified run",
        "environment": {
            "timestamp": environment.get("timestamp"),
            "platform": environment.get("platform"),
            "machine": environment.get("machine"),
            "git_revision": environment.get("git_revision") or environment.get("benchmark_revision"),
        },
        "versions": run.get("versions", {}),
        "workload_ids": sorted({row["workload"] for row in measurements}),
        "runtime_order": [*runtimes, HARA_RUNTIME],
        "measurements": measurements,
    }


def main() -> None:
    source = sys.argv[1] if len(sys.argv) > 1 else DEFAULT_SOURCE
    document = load(source)
    run = latest_run(document)
    evidence = convert(run)
    RESULT.write_text(json.dumps(evidence, indent=2, sort_keys=False) + "\n", encoding="utf-8")
    ok = [row for row in evidence["measurements"] if row["status"] == "ok"]
    runtimes = sorted({row["runtime"] for row in ok})
    print(f"Imported run {evidence['environment']['timestamp']} from {source}")
    print(f"  {len(ok)} ok measurements across {len(runtimes)} runtimes -> {RESULT.relative_to(ROOT)}")


if __name__ == "__main__":
    main()

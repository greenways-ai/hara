#!/usr/bin/env python3
"""Validate complete, publishable hara-benchmark-evidence/v2 documents."""
from __future__ import annotations

import argparse
import json
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
CATALOG = json.loads((ROOT / "lib/bench/catalog.json").read_text())
ARTIFACTS = tuple(item["id"] for item in CATALOG["artifacts"])
WORKLOADS = tuple(CATALOG["corpus"]["workloads"])


def validate(document: dict) -> list[str]:
    errors: list[str] = []
    if document.get("schema") != "hara-benchmark-evidence/v2":
        errors.append("schema must be hara-benchmark-evidence/v2")
    if document.get("profile") != "standard":
        errors.append("only standard-profile evidence can be published")
    environment = document.get("environment", {})
    for key in ("run_id", "timestamp", "platform", "machine", "cpu", "git_revision", "browser"):
        if not environment.get(key): errors.append(f"environment.{key} is required")
    if document.get("corpus", {}).get("id") != CATALOG["corpus"]["id"]:
        errors.append(f"corpus.id must be {CATALOG['corpus']['id']}")

    rows = document.get("measurements", [])
    pairs = {(row.get("runtime"), row.get("workload")) for row in rows if row.get("status", "ok") == "ok"}
    expected = {(runtime, workload) for runtime in ARTIFACTS for workload in WORKLOADS}
    for runtime, workload in sorted(expected - pairs):
        errors.append(f"missing measurement: {runtime} / {workload}")
    for runtime, workload in sorted(pairs - expected):
        errors.append(f"noncanonical measurement: {runtime} / {workload}")
    for row in rows:
        if row.get("status", "ok") != "ok": errors.append(f"published row is not ok: {row.get('runtime')} / {row.get('workload')}")
        for key in ("prepare_ns", "first_ns", "steady_ns", "throughput_per_sec", "checksum"):
            if row.get(key) is None: errors.append(f"{row.get('runtime')} / {row.get('workload')} missing {key}")

    startup = document.get("startup", {})
    sizes = document.get("artifacts", {})
    for runtime in ARTIFACTS:
        if startup.get(runtime, {}).get("p50_ns") is None: errors.append(f"missing startup p50: {runtime}")
        for key in ("base_bytes", "workload_delta_bytes", "raw_total_bytes", "transfer_bytes"):
            if sizes.get(runtime, {}).get(key) is None: errors.append(f"missing artifact {key}: {runtime}")

    for row in document.get("language_measurements", []):
        if row.get("hara_runtime") != "hara-rust-full": errors.append("language comparisons must use hara-rust-full")
    hoplite_servers = {"hoplite-raw", "hoplite-request", "hoplite-request+hta"}
    http_rows = document.get("http_measurements", [])
    http_pairs = {(row.get("server"), row.get("route")) for row in http_rows}
    for server in sorted(hoplite_servers):
        for route in ("/hello", "/json", "/delay"):
            if (server, route) not in http_pairs:
                errors.append(f"missing HTTP measurement: {server} / {route}")
    for row in http_rows:
        if row.get("server", "").startswith("hoplite"):
            if row.get("server") not in hoplite_servers:
                errors.append("Hoplite HTTP rows must use a canonical adapter mode")
            if row.get("hara_runtime") != "hara-rust-full":
                errors.append("Hoplite HTTP rows must use hara-rust-full")
    return errors


def main() -> int:
    parser = argparse.ArgumentParser()
    parser.add_argument("evidence", type=Path)
    args = parser.parse_args()
    errors = validate(json.loads(args.evidence.read_text()))
    if errors:
        print("Evidence is not publishable:")
        for error in errors: print(f"- {error}")
        return 1
    print(f"validated complete evidence: {args.evidence}")
    return 0


if __name__ == "__main__": raise SystemExit(main())

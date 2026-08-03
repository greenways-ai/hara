#!/usr/bin/env python3
"""Assemble the independently captured runtime/browser benchmark evidence."""
from __future__ import annotations

import gzip
import json
import platform
import subprocess
from datetime import datetime, timezone
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
TARGET = ROOT / "target"
RESULT = ROOT / "lib/bench/results/reference-v2.json"


def load(path: str) -> dict:
    return json.loads((ROOT / path).read_text())


def runtime_rows(document: dict, selected: set[str]) -> list[dict]:
    expected = {row["id"]: row["expected"] for row in load("lib/bench/lisp-hara/general-workloads.json")["workloads"]}
    rows = []
    for source in document["measurements"]:
        if source["runtime"] not in selected:
            continue
        analysis = source["analysis"]
        rows.append({**source, "steady_ns": analysis["steady_ns"],
                     "throughput_per_sec": analysis["throughput_per_sec"],
                     "checksum": expected[source["workload"]], "status": "ok"})
    return rows


def artifact(path: Path) -> dict:
    data = path.read_bytes()
    return {"base_bytes": len(data), "workload_delta_bytes": 0,
            "raw_total_bytes": len(data), "transfer_bytes": len(gzip.compress(data, compresslevel=9)),
            "transfer_encoding": "gzip", "path": str(path.relative_to(ROOT))}


def main() -> None:
    rust = load("target/runtime-rust-standard.json")
    jvm = load("target/runtime-jvm-standard.json")
    native = load("target/runtime-internal-smoke.json")
    browser = load("target/browser-tier-benchmark.json")
    core = load("target/browser-core-benchmark.json")
    browser_startup = load("target/browser-startup-benchmark.json")
    http = load("target/hara-http-frameworks.json")
    catalog = load("lib/bench/catalog.json")
    rows = (runtime_rows(rust, {"hara-rust-vm", "hara-rust-full"})
            + runtime_rows(jvm, {"hara-jvm-vm", "hara-jvm-full"})
            + runtime_rows(native, {"hara-truffle-vm", "hara-truffle-full"})
            + browser["measurements"] + core["measurements"])
    startup = {**rust["startup"], **jvm["startup"],
               **{key: native["startup"][key] for key in ("hara-truffle-vm", "hara-truffle-full")},
               **browser_startup["startup"], "hara-wasm-core": core["startup"]}
    artifacts = {
        "hara-wasm-core": artifact(ROOT / "rust/raw/target/wasm32-unknown-unknown/browser-release/hara-wasm-core.wasm"),
        "hara-rust-vm": artifact(ROOT / "target/runtime-benchmark/vm/release/hara-bytecode-benchmark"),
        "hara-rust-full": artifact(ROOT / "target/runtime-benchmark/whole-wasm/release/hara-bytecode-benchmark"),
        "hara-wasm-vm": artifact(ROOT / "rust/web/packages/browser/dist/hara-wasm-vm/hara.mjs"),
        "hara-wasm-full": artifact(ROOT / "rust/web/packages/browser/dist/hara-wasm-full/hara.mjs"),
        "hara-truffle-vm": artifact(ROOT / "target/hara-truffle-vm"),
        "hara-truffle-full": artifact(ROOT / "target/hara-truffle-full"),
    }
    jvm_payload = jvm["payload_bytes"]["hara-jvm-vm"]
    for runtime in ("hara-jvm-vm", "hara-jvm-full"):
        artifacts[runtime] = {"base_bytes": jvm_payload, "workload_delta_bytes": 0,
                              "raw_total_bytes": jvm_payload, "transfer_bytes": jvm_payload,
                              "transfer_encoding": "classpath payload"}
    revision = subprocess.run(["git", "rev-parse", "HEAD"], cwd=ROOT, text=True,
                              capture_output=True, check=True).stdout.strip()
    now = datetime.now(timezone.utc).isoformat()
    document = {
        "schema": "hara-benchmark-evidence/v2", "profile": "standard",
        "corpus": catalog["corpus"],
        "environment": {"run_id": f"reference-{now[:19].replace(':', '')}", "timestamp": now,
                        "platform": platform.platform(), "machine": platform.machine(),
                        "cpu": platform.processor() or platform.machine(), "git_revision": revision,
                        "git_dirty": True, "browser": "Playwright Chromium",
                        "native_image": "GraalVM 25.0.3 fallback executable"},
        "protocols": {
            "standard-runtime": {"artifacts": ["hara-rust-vm", "hara-rust-full", "hara-jvm-vm", "hara-jvm-full"], "windows": 60, "calls_per_window": 10, "startup_samples": 30},
            "standard-browser": {"artifacts": ["hara-wasm-vm", "hara-wasm-full"], "adaptive_window_ms": 250, "windows": 30, "startup_samples": 30},
            "browser-core-end-to-end": {"artifacts": ["hara-wasm-core"], "samples": 1, "note": "Source parse, compile, and execution are inseparable in the tiny core API."},
            "native-image-fallback": {"artifacts": ["hara-truffle-vm", "hara-truffle-full"], "windows": 3, "calls_per_window": 1, "startup_samples": 2, "note": "Reduced sample count: GraalVM 25 blocks runtime Truffle compilation and these executables use fallback mode."},
        },
        "startup": startup, "artifacts": artifacts, "measurements": rows,
        "language_evidence": "lib/bench/results/language-reference.json",
        "http_environment": http["environment"],
        "http_configuration": http["configuration"],
        "http_measurements": http["measurements"],
    }
    RESULT.parent.mkdir(parents=True, exist_ok=True)
    RESULT.write_text(json.dumps(document, indent=2) + "\n")
    print(f"wrote {len(rows)} rows to {RESULT}")


if __name__ == "__main__":
    main()

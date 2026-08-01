#!/usr/bin/env python3
"""Lisp (SBCL / Chez Scheme / Guile) vs Hara (Rust native) comparison
benchmark coordinator.

Modelled on lib/bench/luajit-hara/run.py: windowed sampling, steady-state
median analysis, JSON + Markdown output. Results default to target/
(gitignored scratch) — this is comparison evidence, not regression gating.
"""

from __future__ import annotations

import argparse
import datetime as dt
import json
import os
import platform
import shutil
import statistics
import subprocess
import time
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
HERE = ROOT / "lib/bench/lisp-hara"
DEFAULT_CORPUS = HERE / "workloads.json"
CHEZ_RUNNER = HERE / "chez_runner.scm"
GUILE_RUNNER = HERE / "guile_runner.scm"
SBCL_RUNNER = HERE / "sbcl_runner.lisp"
HARA_BENCH = ROOT / "rust/target/release/hara-runtime-benchmark"

PROFILES = {
    "smoke": {"startup_samples": 2, "windows": 3, "calls": 1},
    "standard": {"startup_samples": 30, "windows": 60, "calls": 10},
}

LISP_RUNTIMES = {
    "sbcl": {"command": ["sbcl", "--script", str(SBCL_RUNNER)],
             "source_field": "cl_source", "binary": "sbcl"},
    "chez": {"command": ["chez", "--script", str(CHEZ_RUNNER)],
             "source_field": "scheme_source", "binary": "chez"},
    "guile": {"command": ["guile", "-s", str(GUILE_RUNNER)],
              "source_field": "scheme_source", "binary": "guile"},
}


def run(command, *, timeout=180, check=True):
    return subprocess.run(command, cwd=ROOT, text=True,
                          capture_output=True, timeout=timeout, check=check)


def version(command):
    try:
        result = run(command, check=False, timeout=20)
        text = (result.stdout or result.stderr).strip().splitlines()
        return text[0] if text else "unknown"
    except (OSError, subprocess.SubprocessError):
        return "unavailable"


def hex_payload(source):
    return source.encode().hex()


BYTECODE_VARIANTS = {
    "hara-rust-bytecode": ("bytecode-vm", "vm"),
    "hara-rust-trace-checked": ("tracing-jit", "trace-checked"),
    "hara-rust-trace-native": ("native-jit", "trace-native"),
}


def bytecode_binary(label):
    return ROOT / "target/runtime-benchmark" / label / "release/hara-bytecode-benchmark"


def build_bytecode(selected):
    for runtime, (features, label) in BYTECODE_VARIANTS.items():
        if runtime not in selected:
            continue
        env = os.environ.copy()
        env["CARGO_TARGET_DIR"] = str(ROOT / "target/runtime-benchmark" / label)
        subprocess.run(["cargo", "build", "--manifest-path", "rust/Cargo.toml",
                        "--release", "--features", features,
                        "--bin", "hara-bytecode-benchmark"],
                       cwd=ROOT, env=env, check=True, timeout=600)


def adapters():
    def bytecode(binary, runtime, workload, windows, calls):
        return [str(binary), "execute-only", workload["id"],
                hex_payload(workload["hara_source"]), workload["expected"],
                str(windows), str(calls), runtime]

    def lisp(name, workload, windows, calls):
        spec = LISP_RUNTIMES[name]
        return spec["command"] + [
            workload["id"], hex_payload(workload[spec["source_field"]]),
            workload["expected"], str(windows), str(calls)]

    result = {
        "hara-rust-native": lambda w, n, c: [
            str(HARA_BENCH), "hara-rust-native", w["id"],
            hex_payload(w["hara_source"]), w["expected"], str(n), str(c)],
    }
    for name in LISP_RUNTIMES:
        result[name] = lambda w, n, c, name=name: lisp(name, w, n, c)
    for runtime, (_, label) in BYTECODE_VARIANTS.items():
        result[runtime] = (
            lambda w, n, c, b=bytecode_binary(label), r=runtime:
            bytecode(b, r, w, n, c))
    return result


def percentile(values, fraction):
    ordered = sorted(values)
    return ordered[min(len(ordered) - 1, int((len(ordered) - 1) * fraction))]


def analyse(samples):
    tail = samples[-10:]
    reference = statistics.median(tail)
    converged = None
    for index in range(0, max(0, len(samples) - 4)):
        window = samples[index:index + 5]
        if all(abs(value - reference) <= reference * 0.05 for value in window):
            mean = statistics.mean(window)
            cv = statistics.pstdev(window) / mean if mean else 0
            if cv <= 0.10:
                converged = index
                break
    return {"steady_ns": int(reference),
            "throughput_per_sec": 1e9 / reference if reference else None,
            "converged_window": converged, "converged": converged is not None}


def timed(command):
    started = time.perf_counter_ns()
    result = run(command, timeout=1200)
    elapsed = time.perf_counter_ns() - started
    line = next(line for line in reversed(result.stdout.splitlines())
                if line.startswith("{"))
    return elapsed, json.loads(line)


def markdown(data):
    lisps = [name for name in data["runtime_order"] if name in LISP_RUNTIMES]
    hara_tiers = [name for name in data["runtime_order"]
                  if name not in LISP_RUNTIMES]
    lines = ["# Lisp vs Hara (Rust native) benchmark", "",
             f"Generated: `{data['environment']['timestamp']}` on "
             f"`{data['environment']['platform']}`.", "",
             "Values are machine-specific comparison evidence, not regression "
             "thresholds. All runtimes parse and evaluate the workload source "
             "on every call; the Scheme and Common Lisp sources are "
             "hand-written untyped equivalents checked against the same "
             "expected value.", "",
             "## Startup", "", "| Runtime | p50 ms | p95 ms |", "|---|---:|---:|"]
    for name, item in data["startup"].items():
        lines.append(f"| {name} | {item['p50_ns']/1e6:.2f} | {item['p95_ns']/1e6:.2f} |")
    lines += ["", "## Warm evaluation", "",
              "| Runtime / workload | First ms | Steady ms | ns/iteration | calls/s | Converged window |",
              "|---|---:|---:|---:|---:|---:|"]
    for row in data["measurements"]:
        convergence = row["analysis"]["converged_window"]
        per_iteration = row["analysis"].get("ns_per_iteration")
        per_iteration_text = "—" if per_iteration is None else f"{per_iteration:.2f}"
        throughput = row["analysis"]["throughput_per_sec"]
        throughput_text = "—" if throughput is None else f"{throughput:.1f}"
        lines.append(
            f"| {row['runtime']} / {row['workload']} | {row['first_ns']/1e6:.3f} "
            f"| {row['analysis']['steady_ns']/1e6:.3f} | {per_iteration_text} "
            f"| {throughput_text} "
            f"| {convergence if convergence is not None else '—'} |")
    lines += ["", "## Head-to-head (steady state, lisp / hara tier)", "",
              "| Workload | Lisp | Hara tier | Lisp steady ms | Hara steady ms | Ratio |",
              "|---|---|---|---:|---:|---:|"]
    index = {(m["runtime"], m["workload"]): m for m in data["measurements"]}
    for workload in data["workload_ids"]:
        for lisp in lisps:
            base = index.get((lisp, workload))
            for tier in hara_tiers:
                hara = index.get((tier, workload))
                if base and hara:
                    lisp_ns = base["analysis"]["steady_ns"]
                    hara_ns = hara["analysis"]["steady_ns"]
                    lines.append(f"| {workload} | {lisp} | {tier} | {lisp_ns/1e6:.3f} "
                                 f"| {hara_ns/1e6:.3f} | {lisp_ns/hara_ns:.4f} |")
    lines += ["", "Ratio < 1 means the Lisp runtime is faster. `hara-rust-native` "
              "and the Lisp runners re-parse and evaluate the source on every "
              "call (SBCL evals in :compile mode; Chez compiles eval'd forms; "
              "Guile compiles eval'd forms to bytecode); the `hara-rust-*` VM "
              "tiers compile once and execute only (their first value is the "
              "first execution, not compilation). Convergence is the first "
              "five-window run within ±5% of the final ten-window median with "
              "CV ≤10%.", ""]
    return "\n".join(lines)


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--profile", choices=PROFILES, default="smoke")
    parser.add_argument("--runtime", action="append")
    parser.add_argument("--corpus", type=Path, default=DEFAULT_CORPUS)
    parser.add_argument("--output", type=Path,
                        default=ROOT / "target/lisp-hara-benchmark.json")
    parser.add_argument("--no-build", action="store_true")
    args = parser.parse_args()
    profile = PROFILES[args.profile]
    runtime_adapters = adapters()
    selected = args.runtime or list(runtime_adapters)
    unknown = sorted(set(selected) - set(runtime_adapters))
    if unknown:
        parser.error("unknown runtime(s): " + ", ".join(unknown))

    if not args.no_build:
        if "hara-rust-native" in selected:
            run(["cargo", "build", "--manifest-path", "rust/Cargo.toml", "--release",
                 "--bin", "hara-runtime-benchmark"], timeout=600)
        build_bytecode(selected)
    if "hara-rust-native" in selected and not HARA_BENCH.is_file():
        parser.error(f"missing {HARA_BENCH} (build it or drop --no-build)")
    for runtime, (_, label) in BYTECODE_VARIANTS.items():
        if runtime in selected and not bytecode_binary(label).is_file():
            parser.error(f"missing {bytecode_binary(label)} (build it or drop --no-build)")
    for name, spec in LISP_RUNTIMES.items():
        if name in selected and not shutil.which(spec["binary"]):
            parser.error(f"{spec['binary']} not found on PATH "
                         f"(brew install {'chezscheme' if name == 'chez' else name})")

    corpus_path = args.corpus if args.corpus.is_absolute() else ROOT / args.corpus
    corpus = json.loads(corpus_path.read_text())["workloads"]

    measurements = []
    startup = {}
    for name in selected:
        adapter = runtime_adapters[name]
        elapsed = []
        for _ in range(profile["startup_samples"]):
            wall, _ = timed(adapter(corpus[0], 0, 1))
            elapsed.append(wall)
        startup[name] = {"samples_ns": elapsed,
                         "p50_ns": int(statistics.median(elapsed)),
                         "p95_ns": percentile(elapsed, 0.95)}
        for workload in corpus:
            _, result = timed(adapter(workload, profile["windows"], profile["calls"]))
            result["analysis"] = analyse(result["samples_ns"])
            if workload.get("iterations"):
                result["analysis"]["ns_per_iteration"] = (
                    result["analysis"]["steady_ns"] / workload["iterations"])
            measurements.append(result)
            print(f"{name:18} {workload['id']:18} "
                  f"{result['analysis']['steady_ns']/1e6:9.3f} ms")

    data = {"schema_version": 1, "profile": args.profile,
            "corpus": str(corpus_path.relative_to(ROOT)),
            "environment": {"timestamp": dt.datetime.now(dt.timezone.utc).isoformat(),
                            "platform": platform.platform(),
                            "machine": platform.machine(),
                            "python": platform.python_version(),
                            "git_revision": version(["git", "rev-parse", "HEAD"]),
                            "git_dirty": bool(run(["git", "status", "--porcelain"]).stdout)},
            "versions": {"sbcl": version(["sbcl", "--version"]),
                         "chez": version(["chez", "--version"]),
                         "guile": version(["guile", "--version"]),
                         "rust": version(["rustc", "--version"])},
            "workload_ids": [w["id"] for w in corpus],
            "runtime_order": selected,
            "startup": startup, "measurements": measurements}

    output = args.output if args.output.is_absolute() else ROOT / args.output
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_text(json.dumps(data, indent=2) + "\n")
    report_path = output.with_suffix(".md")
    report_path.write_text(markdown(data))
    print(f"wrote {output} and {report_path}")


if __name__ == "__main__":
    main()

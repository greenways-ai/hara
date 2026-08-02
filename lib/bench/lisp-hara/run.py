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
import math
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
BB_RUNNER = HERE / "bb_runner.clj"
PYTHON_RUNNER = HERE / "python_runner.py"
C_RUNNER = HERE / "c_runner.py"
JAVA_RUNNER = HERE / "java_runner.py"
LUA_RUNNER = ROOT / "lib/bench/luajit-hara/lua_runner.lua"
HARA_BENCH = ROOT / "rust/target/release/hara-runtime-benchmark"

PROFILES = {
    "smoke": {"startup_samples": 2, "windows": 3, "calls": 1},
    "algorithm": {"startup_samples": 10, "windows": 30, "calls": 3},
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

LANGUAGE_RUNTIMES = {**LISP_RUNTIMES,
                     "bb": {"command": ["bb", str(BB_RUNNER)],
                            "source_field": "bb_source", "binary": "bb"},
                     "python": {"command": ["python3", str(PYTHON_RUNNER)],
                                "source_field": "python_source", "binary": "python3"},
                     "c": {"command": ["python3", str(C_RUNNER)],
                           "source_field": "c_source", "binary": "cc",
                           "modes": ("prepared",)},
                     "java": {"command": ["python3", str(JAVA_RUNNER)],
                              "source_field": "java_source", "binary": "python3",
                              "modes": ("prepared",)},
                     "luajit": {"command": ["luajit", str(LUA_RUNNER)],
                                "source_field": "lua_source", "binary": "luajit"}}


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
    "hara-rust-vm": ("bytecode-vm", "vm"),
    "hara-rust-trace-checked": ("tracing-jit", "trace-checked"),
    "hara-rust-trace-native": ("native-jit", "trace-native"),
    "hara-rust-full": ("whole-wasm", "whole-wasm"),
}


def bytecode_binary(label):
    return ROOT / "target/runtime-benchmark" / label / "release/hara-bytecode-benchmark"


def build_bytecode(selected):
    for runtime, (features, label) in BYTECODE_VARIANTS.items():
        if (f"{runtime}-prepared" not in selected and
                not (runtime == "hara-rust-vm" and
                     "hara-rust-vm-eval" in selected)):
            continue
        env = os.environ.copy()
        env["CARGO_TARGET_DIR"] = str(ROOT / "target/runtime-benchmark" / label)
        subprocess.run(["cargo", "build", "--manifest-path", "rust/Cargo.toml",
                        "--release", "--features", features,
                        "--bin", "hara-bytecode-benchmark"],
                       cwd=ROOT, env=env, check=True, timeout=600)


def adapters():
    def bytecode(binary, runtime, mode, workload, windows, calls):
        return [str(binary), mode, workload["id"],
                hex_payload(workload["hara_source"]), workload["expected"],
                str(windows), str(calls), runtime]

    def language(name, mode, workload, windows, calls):
        spec = LANGUAGE_RUNTIMES[name]
        source = workload.get(spec["source_field"], workload["hara_source"])
        return spec["command"] + [
            mode, workload["id"], hex_payload(source),
            workload["expected"], str(windows), str(calls)]

    result = {
        "hara-rust-native-eval": lambda w, n, c: [
            str(HARA_BENCH), "hara-rust-native-eval", w["id"],
            hex_payload(w["hara_source"]), w["expected"], str(n), str(c)],
    }
    for name in LANGUAGE_RUNTIMES:
        for mode in LANGUAGE_RUNTIMES[name].get("modes", ("eval", "prepared")):
            label = f"{name}-{mode}"
            result[label] = lambda w, n, c, name=name, mode=mode: language(
                name, mode, w, n, c)
    for runtime, (_, label) in BYTECODE_VARIANTS.items():
        if runtime == "hara-rust-full":
            result[f"{runtime}-prepared"] = (
                lambda w, n, c, b=bytecode_binary(label), r=runtime:
                bytecode(b, f"{r}-prepared", "whole-wasm", w, n, c))
            continue
        result[f"{runtime}-prepared"] = (
            lambda w, n, c, b=bytecode_binary(label), r=runtime:
            bytecode(b, f"{r}-prepared", "runtime-registry-execute", w, n, c))
    result["hara-rust-vm-eval"] = (
        lambda w, n, c, b=bytecode_binary("vm"):
        bytecode(b, "hara-rust-vm-eval", "compile-execute", w, n, c))
    return result


def percentile(values, fraction):
    ordered = sorted(values)
    return ordered[min(len(ordered) - 1, math.ceil((len(ordered) - 1) * fraction))]


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
    lisps = [name for name in data["runtime_order"]
             if name.split("-")[0] in LANGUAGE_RUNTIMES]
    hara_tiers = [name for name in data["runtime_order"] if name not in lisps]
    lines = ["# Lisp vs Hara (Rust native) benchmark", "",
             f"Generated: `{data['environment']['timestamp']}` on "
             f"`{data['environment']['platform']}`.", "",
             "Values are machine-specific comparison evidence, not regression "
             "thresholds. `-eval` rows include source loading on every call; "
             "`-prepared` rows compile/load once and invoke repeatedly. Rows "
             "from different lanes are not apples-to-apples.", "",
             "## Startup", "", "| Runtime | p50 ms | p95 ms |", "|---|---:|---:|"]
    for name, item in data["startup"].items():
        if item.get("status") == "unsupported":
            lines.append(f"| {name} | — | — |")
        else:
            lines.append(f"| {name} | {item['p50_ns']/1e6:.2f} | {item['p95_ns']/1e6:.2f} |")
    lines += ["", "## Warm evaluation", "",
              "| Runtime / workload | First ms | Steady ms | ns/iteration | calls/s | Converged window |",
              "|---|---:|---:|---:|---:|---:|"]
    for row in data["measurements"]:
        if row.get("status") == "unsupported":
            lines.append(f"| {row['runtime']} / {row['workload']} | — | — | — | — | — |")
            continue
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
    lines += ["", "## Feature coverage", "", "| Runtime / workload | Status | Detail |",
              "|---|---|---|"]
    for row in data["measurements"]:
        status = row.get("status", "ok")
        detail = row.get("reason", "checksum verified").replace("|", "\\|")
        lines.append(f"| {row['runtime']} / {row['workload']} | {status} | {detail} |")
    lines += ["", "## Head-to-head (steady state, lisp / hara tier)", "",
              "| Workload | Lisp | Hara tier | Lisp steady ms | Hara steady ms | Ratio |",
              "|---|---|---|---:|---:|---:|"]
    index = {(m["runtime"], m["workload"]): m for m in data["measurements"]}
    for workload in data["workload_ids"]:
        for lisp in lisps:
            base = index.get((lisp, workload))
            for tier in hara_tiers:
                hara = index.get((tier, workload))
                if (base and hara and base.get("status") == "ok"
                        and hara.get("status") == "ok"):
                    lisp_ns = base["analysis"]["steady_ns"]
                    hara_ns = hara["analysis"]["steady_ns"]
                    lines.append(f"| {workload} | {lisp} | {tier} | {lisp_ns/1e6:.3f} "
                                 f"| {hara_ns/1e6:.3f} | {lisp_ns/hara_ns:.4f} |")
    lines += ["", "Ratio < 1 means the comparison runtime is faster. Compare "
              "only rows carrying the same `-eval` or `-prepared` suffix. "
              "Convergence is the first "
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
        if "hara-rust-native-eval" in selected:
            run(["cargo", "build", "--manifest-path", "rust/Cargo.toml", "--release",
                 "--bin", "hara-runtime-benchmark"], timeout=600)
        build_bytecode(selected)
    if "hara-rust-native-eval" in selected and not HARA_BENCH.is_file():
        parser.error(f"missing {HARA_BENCH} (build it or drop --no-build)")
    for runtime, (_, label) in BYTECODE_VARIANTS.items():
        if (f"{runtime}-prepared" in selected or
                (runtime == "hara-rust-vm" and
                 "hara-rust-vm-eval" in selected)) and not bytecode_binary(label).is_file():
            parser.error(f"missing {bytecode_binary(label)} (build it or drop --no-build)")
    for name, spec in LANGUAGE_RUNTIMES.items():
        if any(runtime.startswith(f"{name}-") for runtime in selected) and not shutil.which(spec["binary"]):
            parser.error(f"{spec['binary']} not found on PATH "
                         f"(brew install {'chezscheme' if name == 'chez' else name})")

    corpus_path = args.corpus if args.corpus.is_absolute() else ROOT / args.corpus
    corpus = json.loads(corpus_path.read_text())["workloads"]

    measurements = []
    startup = {}
    for name in selected:
        adapter = runtime_adapters[name]
        elapsed = []
        startup_workload = None
        for candidate in corpus:
            try:
                wall, _ = timed(adapter(candidate, 0, 1))
            except subprocess.CalledProcessError:
                continue
            startup_workload = candidate
            elapsed.append(wall)
            break
        if startup_workload is None:
            startup[name] = {"status": "unsupported",
                             "reason": "no corpus workload is supported"}
        else:
            for _ in range(1, profile["startup_samples"]):
                wall, _ = timed(adapter(startup_workload, 0, 1))
                elapsed.append(wall)
            startup[name] = {
                "status": "ok", "workload": startup_workload["id"],
                "samples_ns": elapsed,
                "p50_ns": int(statistics.median(elapsed)),
                "p95_ns": percentile(elapsed, 0.95)}
        for workload in corpus:
            try:
                _, result = timed(adapter(workload, profile["windows"], profile["calls"]))
            except subprocess.CalledProcessError as error:
                message = (error.stderr or error.stdout or str(error)).strip().splitlines()
                result = {"runtime": name, "workload": workload["id"],
                          "status": "unsupported",
                          "reason": message[-1] if message else str(error)}
                measurements.append(result)
                print(f"{name:30} {workload['id']:30} unsupported: {result['reason']}")
                continue
            result["runtime"] = name
            result["analysis"] = analyse(result["samples_ns"])
            result["status"] = "ok"
            operations = workload.get("operations", workload.get("iterations"))
            if operations:
                result["analysis"]["ns_per_iteration"] = (
                    result["analysis"]["steady_ns"] / operations)
            measurements.append(result)
            print(f"{name:18} {workload['id']:18} "
                  f"{result['analysis']['steady_ns']/1e6:9.3f} ms")

    data = {"schema_version": 2, "profile": args.profile,
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
                         "luajit": version(["luajit", "-v"]),
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

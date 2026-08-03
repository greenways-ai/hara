#!/usr/bin/env python3
"""Reproducible cross-runtime startup and warm-up benchmark coordinator."""

from __future__ import annotations

import argparse
import base64
import datetime as dt
import hashlib
import json
import os
import platform
import shutil
import statistics
import subprocess
import sys
import tempfile
import time
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
DEFAULT_CORPUS = ROOT / "lib/bench/lisp-hara/general-workloads.json"
RESULTS = ROOT / "lib/bench/results/reference.json"
REPORT = ROOT / "website/docs/reference/runtime-benchmarks.md"
DEFAULT_BASELINE = ROOT / "lib/bench/runtime/regression-baselines.json"
BYTECODE_VARIANTS = {
    "hara-rust-vm": ("bytecode-vm", "vm"),
    "hara-rust-full": ("whole-wasm", "whole-wasm"),
    "hara-rust-trace-checked": ("tracing-jit", "trace-checked"),
    "hara-rust-trace-native": ("native-jit", "trace-native"),
}
PROFILES = {
    "smoke": {"startup_samples": 2, "windows": 3, "calls": 1},
    "standard": {"startup_samples": 30, "windows": 60, "calls": 10},
}


def workload_for_runtime(workload, runtime):
    """Resolve an explicitly equivalent source for a runtime.

    Workloads without overrides remain shared. A workload with ``sources`` is
    only valid for the named runtimes (or its optional ``default`` entry), so
    an adapter can never silently benchmark different collection semantics.
    """
    if "source" not in workload and "hara_source" in workload:
        workload = {**workload, "source": workload["hara_source"]}
    sources = workload.get("sources")
    if not sources:
        return workload
    source_aliases = {
        "hara-rust-vm": "hara-rust-bytecode",
        "hara-rust-full": "hara-rust-bytecode",
    }
    source = sources.get(runtime, sources.get(source_aliases.get(runtime), sources.get("default")))
    if source is None:
        return None
    return {**workload, "source": source}


def run(command, *, env=None, timeout=120, check=True):
    return subprocess.run(command, cwd=ROOT, env=env, text=True,
                          capture_output=True, timeout=timeout, check=check)


def version(command):
    try:
        result = run(command, check=False, timeout=20)
        text = (result.stdout or result.stderr).strip().splitlines()
        return text[0] if text else "unknown"
    except (OSError, subprocess.SubprocessError):
        return "unavailable"


def java_executable():
    configured = os.environ.get("JAVA_HOME")
    candidates = ([Path(configured) / "bin/java"] if configured else []) + [
        Path("/opt/homebrew/opt/openjdk/bin/java"), Path(shutil.which("java") or "")]
    return str(next((candidate for candidate in candidates if candidate.is_file()), Path("java")))


def classpaths():
    local = Path.home() / ".m2/repository/org/clojure"
    clojure = local / "clojure/1.12.5/clojure-1.12.5.jar"
    spec = local / "spec.alpha/0.5.238/spec.alpha-0.5.238.jar"
    core_spec = local / "core.specs.alpha/0.4.74/core.specs.alpha-0.4.74.jar"
    cp_file = ROOT / "java/target/hara-runtime-classpath.txt"
    truffle = os.pathsep.join((
        str(ROOT / "java/target/classes"),
        str(ROOT / "java/target/truffle-dependencies/*"),
    ))
    if cp_file.exists():
        truffle += os.pathsep + cp_file.read_text().strip()
    return os.pathsep.join(map(str, (clojure, spec, core_spec))), truffle


def encoded(source):
    return base64.urlsafe_b64encode(source.encode()).decode().rstrip("=")


def adapters():
    clj_cp, truffle_cp = classpaths()
    clj_script = str(ROOT / "lib/bench/runtime/clojure_runner.clj")
    node_script = str(ROOT / "lib/bench/runtime/node_runner.mjs")
    glue = ROOT / "target/wasm-bindgen/hara_wasm.js"

    def common(command, runtime, workload, windows, calls, source_encoding="base64"):
        source = workload["source"]
        payload = source.encode().hex() if source_encoding == "hex" else encoded(source)
        return command + [runtime, workload["id"], payload, workload["expected"],
                          str(windows), str(calls)]

    def java(command, runtime, representation, workload, windows, calls):
        source = workload["source"]
        if representation == "vm":
            artifact_dir = ROOT / "target/runtime-benchmark/hbc"
            artifact_dir.mkdir(parents=True, exist_ok=True)
            digest = hashlib.sha256(source.encode()).hexdigest()
            artifact = artifact_dir / f"{digest}.hbc"
            if not artifact.is_file():
                run([str(bytecode_artifact_binary()), source.encode().hex(), str(artifact)])
            payload = base64.urlsafe_b64encode(artifact.read_bytes()).decode().rstrip("=")
        else:
            payload = encoded(source)
        return command + [runtime, representation, workload["id"], payload,
                          workload["expected"], str(windows), str(calls)]

    def bytecode(binary, runtime, mode, workload, windows, calls):
        source = workload["source"].encode().hex()
        return [str(binary), mode, workload["id"], source, workload["expected"],
                str(windows), str(calls), runtime]

    return {
        "clojure": lambda w, n, c: common(
            [java_executable(), "-cp", clj_cp, "clojure.main", clj_script], "clojure", w, n, c),
        "bb": lambda w, n, c: common(["bb", clj_script], "bb", w, n, c),
        "hara-jvm-vm": lambda w, n, c: java(
            [java_executable(), "-cp", truffle_cp, "hara.truffle.Main", "benchmark"],
            "hara-jvm-vm", "vm", w, n, c),
        "hara-jvm-full": lambda w, n, c: java(
            [java_executable(), "-cp", truffle_cp, "hara.truffle.Main", "benchmark"],
            "hara-jvm-full", "full", w, n, c),
        "hara-truffle-vm": lambda w, n, c: java(
            [str(ROOT / "target/hara-truffle-vm"), "benchmark"],
            "hara-truffle-vm", "vm", w, n, c),
        "hara-truffle-full": lambda w, n, c: java(
            [str(ROOT / "target/hara-truffle-full"), "benchmark"],
            "hara-truffle-full", "full", w, n, c),
        "hara-rust-vm": lambda w, n, c: bytecode(
            bytecode_binary("vm"), "hara-rust-vm", "runtime-registry-execute", w, n, c),
        "hara-rust-full": lambda w, n, c: bytecode(
            bytecode_binary("whole-wasm"), "hara-rust-full", "whole-wasm", w, n, c),
        "hara-rust-trace-checked": lambda w, n, c: bytecode(
            bytecode_binary("trace-checked"), "hara-rust-trace-checked", "runtime-registry-execute", w, n, c),
        "hara-rust-trace-native": lambda w, n, c: bytecode(
            bytecode_binary("trace-native"), "hara-rust-trace-native", "runtime-registry-execute", w, n, c),
    }, glue


def bytecode_binary(label):
    return ROOT / "target/runtime-benchmark" / label / "release/hara-bytecode-benchmark"


def bytecode_artifact_binary():
    return ROOT / "target/runtime-benchmark/vm/release/hara-bytecode-artifact"


def build(include_native, selected):
    native_variants = {
        "hara-truffle-vm": ("target/hara-truffle-vm", "false"),
        "hara-truffle-full": ("target/hara-truffle-full", "false"),
    }
    for runtime, (output, fallback) in native_variants.items():
        if include_native or runtime in selected:
            env = os.environ.copy()
            env["HARA_NATIVE_OUTPUT"] = str(ROOT / output)
            env["HARA_NATIVE_USE_FALLBACK_RUNTIME"] = fallback
            run([str(ROOT / "scripts/build-truffle-native")], env=env, timeout=1200)
    if any(runtime.startswith("hara-jvm-") for runtime in selected):
        run(["mvn", "-q", "-f", "java/pom.xml", "-Ptruffle", "-DskipTests", "compile",
             "dependency:build-classpath", "-Dmdep.outputFile=java/target/hara-runtime-classpath.txt"],
            timeout=300)
    for runtime, (features, label) in BYTECODE_VARIANTS.items():
        if runtime not in selected:
            continue
        env = os.environ.copy()
        env["CARGO_TARGET_DIR"] = str(ROOT / "target/runtime-benchmark" / label)
        run(["cargo", "build", "--manifest-path", "rust/Cargo.toml", "--release",
             "--features", features, "--bin", "hara-bytecode-benchmark"],
            env=env, timeout=600)
    if any(runtime in selected for runtime in ("hara-jvm-vm", "hara-truffle-vm")):
        env = os.environ.copy()
        env["CARGO_TARGET_DIR"] = str(ROOT / "target/runtime-benchmark/vm")
        run(["cargo", "build", "--manifest-path", "rust/Cargo.toml", "--release",
             "--features", "bytecode-vm", "--bin", "hara-bytecode-artifact"],
            env=env, timeout=600)


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
    return {"steady_ns": int(reference), "throughput_per_sec": 1e9 / reference,
            "converged_window": converged, "converged": converged is not None}


def timed(command, env):
    rss = None
    actual = command
    rss_file = None
    # GNU time supports the machine-readable `-f`/`-o` pair. macOS ships BSD
    # time at the same path with a different command-line interface.
    if platform.system() == "Linux" and Path("/usr/bin/time").exists():
        handle = tempfile.NamedTemporaryFile(delete=False)
        handle.close()
        rss_file = Path(handle.name)
        actual = ["/usr/bin/time", "-f", "%M", "-o", str(rss_file)] + command
    started = time.perf_counter_ns()
    # Standard profiles deliberately repeat slower interpreter workloads for
    # long enough to establish convergence. Ackermann on the JVM/native-image
    # VM tiers can exceed fifteen minutes at 60 windows × 10 calls.
    result = run(actual, env=env, timeout=1800)
    elapsed = time.perf_counter_ns() - started
    if rss_file:
        try:
            rss = int(rss_file.read_text().strip())
        finally:
            rss_file.unlink(missing_ok=True)
    line = next(line for line in reversed(result.stdout.splitlines()) if line.startswith("{"))
    return elapsed, rss, json.loads(line)


def payload_sizes(glue):
    clojure_files = [
        Path.home() / ".m2/repository/org/clojure/clojure/1.12.5/clojure-1.12.5.jar",
        Path.home() / ".m2/repository/org/clojure/spec.alpha/0.5.238/spec.alpha-0.5.238.jar",
        Path.home() / ".m2/repository/org/clojure/core.specs.alpha/0.4.74/core.specs.alpha-0.4.74.jar"]
    cp_file = ROOT / "java/target/hara-runtime-classpath.txt"
    truffle_files = [ROOT / "java/target/classes"]
    if cp_file.exists():
        truffle_files += [Path(value) for value in cp_file.read_text().strip().split(os.pathsep)]
    paths = {
        "clojure": clojure_files,
        "bb": [Path(shutil.which("bb") or "")],
        "hara-jvm-vm": truffle_files,
        "hara-jvm-full": truffle_files,
        "hara-truffle-vm": [ROOT / "target/hara-truffle-vm"],
        "hara-truffle-full": [ROOT / "target/hara-truffle-full"],
        "hara-rust-vm": [bytecode_binary("vm")],
        "hara-rust-full": [bytecode_binary("whole-wasm")],
        "hara-rust-trace-checked": [bytecode_binary("trace-checked")],
        "hara-rust-trace-native": [bytecode_binary("trace-native")],
    }
    def size(path):
        if path.is_file(): return path.stat().st_size
        if path.is_dir(): return sum(item.stat().st_size for item in path.rglob("*") if item.is_file())
        return 0
    return {name: sum(size(path) for path in files)
            for name, files in paths.items()}


def measurement_index(measurements):
    return {(item["runtime"], item["workload"]): item for item in measurements}


def check_regressions(data, baseline):
    index = measurement_index(data["measurements"])
    failures = []
    for rule in baseline.get("rules", []):
        candidate_key = (rule["runtime"], rule["workload"])
        reference_key = (rule["relative_to"], rule["workload"])
        candidate = index.get(candidate_key)
        reference = index.get(reference_key)
        if candidate is None or reference is None:
            failures.append(f"missing measurement for {candidate_key} or {reference_key}")
            continue
        ratio = candidate["analysis"]["steady_ns"] / reference["analysis"]["steady_ns"]
        if ratio > rule["max_ratio"]:
            failures.append(
                f"{rule['runtime']} / {rule['workload']} ratio {ratio:.3f} "
                f"exceeds {rule['max_ratio']:.3f} vs {rule['relative_to']}")
    return failures


def markdown(data):
    lines = ["# Runtime benchmark reference", "",
             f"Generated: `{data['environment']['timestamp']}` on `{data['environment']['platform']}`.", "",
             "Values are machine-specific evidence, not regression thresholds.",
             "The Truffle/JVM row used the fallback interpreter because this Temurin JVM has no JVMCI compiler.", "",
             "## Startup", "", "| Runtime | p50 ms | p95 ms | Peak RSS MiB | Payload MiB |", "|---|---:|---:|---:|---:|"]
    sizes = data["payload_bytes"]
    for name, item in data["startup"].items():
        rss = "—" if item["peak_rss_kib"] is None else f"{item['peak_rss_kib']/1024:.1f}"
        size = f"{sizes.get(name, 0)/1048576:.1f}" if name in sizes else "—"
        lines.append(f"| {name} | {item['p50_ns']/1e6:.2f} | {item['p95_ns']/1e6:.2f} | {rss} | {size} |")
    lines += ["", "## Warm evaluation", "", "| Runtime / workload | First ms | Steady ms | ns/iteration | calls/s | Converged window |", "|---|---:|---:|---:|---:|---:|"]
    for row in data["measurements"]:
        convergence = row["analysis"]["converged_window"]
        per_iteration = row["analysis"].get("ns_per_iteration")
        per_iteration_text = "—" if per_iteration is None else f"{per_iteration:.2f}"
        lines.append(f"| {row['runtime']} / {row['workload']} | {row['first_ns']/1e6:.3f} | {row['analysis']['steady_ns']/1e6:.3f} | {per_iteration_text} | {row['analysis']['throughput_per_sec']:.1f} | {convergence if convergence is not None else '—'} |")
    lines += ["", "Warm values above are per-call milliseconds (the raw samples are stored as nanoseconds). Lower is better. Adapters receive the same source except where the corpus declares runtime-specific, semantically equivalent APIs (for example Hara mutable collections versus Clojure transients); every adapter checks the same displayed result. Execute-only VM tiers compile once before measurement; their first value is the first execution, not compilation. Convergence is the first five-window run within ±5% of the final ten-window median with CV ≤10%.", ""]
    return "\n".join(lines)


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--profile", choices=PROFILES, default="smoke")
    parser.add_argument("--runtime", action="append")
    parser.add_argument("--corpus", type=Path, default=DEFAULT_CORPUS,
                        help="workload JSON (default: %(default)s)")
    parser.add_argument("--output", type=Path,
                        help="result JSON path; Markdown uses the same stem")
    parser.add_argument("--no-build", action="store_true")
    parser.add_argument("--reference", action="store_true")
    parser.add_argument("--check-regressions", nargs="?", const=DEFAULT_BASELINE, type=Path,
                        help="check relative performance rules (default: %(const)s)")
    args = parser.parse_args()
    profile = PROFILES[args.profile]
    runtime_adapters, glue = adapters()
    selected = args.runtime or list(runtime_adapters)
    unknown = sorted(set(selected) - set(runtime_adapters))
    if unknown:
        parser.error("unknown runtime(s): " + ", ".join(unknown))
    if not args.no_build:
        build(include_native=False, selected=selected)
    missing = []
    corpus_path = args.corpus if args.corpus.is_absolute() else ROOT / args.corpus
    corpus = [({**workload, "source": workload["hara_source"]}
               if "source" not in workload and "hara_source" in workload else workload)
              for workload in json.loads(corpus_path.read_text())["workloads"]]
    env = os.environ.copy()
    env["HARA_WASM_GLUE"] = str(glue)
    measurements = []
    startup = {}
    for name in selected:
        adapter = runtime_adapters[name]
        elapsed = []
        rss_values = []
        for _ in range(profile["startup_samples"]):
            wall, rss, _ = timed(adapter(corpus[0], 0, 1), env)
            elapsed.append(wall)
            if rss is not None: rss_values.append(rss)
        startup[name] = {"samples_ns": elapsed, "p50_ns": int(statistics.median(elapsed)),
                         "p95_ns": percentile(elapsed, 0.95),
                         "peak_rss_kib": max(rss_values) if rss_values else None}
        for workload in corpus:
            resolved = workload_for_runtime(workload, name)
            if resolved is None:
                continue
            _, _, result = timed(adapter(resolved, profile["windows"], profile["calls"]), env)
            result["analysis"] = analyse(result["samples_ns"])
            if workload.get("iterations"):
                result["analysis"]["ns_per_iteration"] = (
                    result["analysis"]["steady_ns"] / workload["iterations"]
                )
            measurements.append(result)
            print(f"{name:18} {workload['id']:18} {result['analysis']['steady_ns']/1e6:9.3f} ms")
    try:
        corpus_label = str(corpus_path.relative_to(ROOT))
    except ValueError:
        corpus_label = str(corpus_path)
    data = {"schema_version": 1, "profile": args.profile,
            "corpus": corpus_label,
            "environment": {"timestamp": dt.datetime.now(dt.timezone.utc).isoformat(),
                            "platform": platform.platform(), "machine": platform.machine(),
                            "python": platform.python_version(),
                            "git_revision": version(["git", "rev-parse", "HEAD"]),
                            "git_dirty": bool(run(["git", "status", "--porcelain"]).stdout)},
            "versions": {"java": version([java_executable(), "-version"]),
                         "clojure": "Clojure 1.12.5", "bb": version(["bb", "--version"]),
                         "rust": version(["rustc", "--version"]), "node": version(["node", "--version"]),
                         "native_image": version(["native-image", "--version"])},
            "missing": missing, "startup": startup, "measurements": measurements,
            "payload_bytes": payload_sizes(glue)}
    if args.output:
        output = args.output if args.output.is_absolute() else ROOT / args.output
    else:
        output = RESULTS if args.reference else ROOT / "target/runtime-benchmark.json"
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_text(json.dumps(data, indent=2) + "\n")
    report = markdown(data)
    if args.output:
        report_path = output.with_suffix(".md")
    else:
        report_path = REPORT if args.reference else ROOT / "target/runtime-benchmark.md"
    report_path.parent.mkdir(parents=True, exist_ok=True)
    report_path.write_text(report)
    failures = []
    if args.check_regressions:
        baseline_path = (args.check_regressions if args.check_regressions.is_absolute()
                         else ROOT / args.check_regressions)
        failures = check_regressions(data, json.loads(baseline_path.read_text()))
        for failure in failures:
            print("REGRESSION: " + failure, file=sys.stderr)
    if missing:
        print("Unavailable: " + ", ".join(missing), file=sys.stderr)
    print(f"wrote {output} and {report_path}")
    if failures:
        raise SystemExit(1)


if __name__ == "__main__":
    main()

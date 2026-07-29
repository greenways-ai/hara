#!/usr/bin/env python3
"""Reproducible cross-runtime startup and warm-up benchmark coordinator."""

from __future__ import annotations

import argparse
import base64
import datetime as dt
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
CORPUS = ROOT / "lib/bench/runtime/workloads.json"
RESULTS = ROOT / "lib/bench/results/reference.json"
REPORT = ROOT / "website/docs/reference/runtime-benchmarks.md"
PROFILES = {
    "smoke": {"startup_samples": 2, "windows": 3, "calls": 1},
    "standard": {"startup_samples": 30, "windows": 60, "calls": 10},
}


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


def classpaths():
    local = Path.home() / ".m2/repository/org/clojure"
    clojure = local / "clojure/1.12.5/clojure-1.12.5.jar"
    spec = local / "spec.alpha/0.5.238/spec.alpha-0.5.238.jar"
    core_spec = local / "core.specs.alpha/0.4.74/core.specs.alpha-0.4.74.jar"
    cp_file = ROOT / "java/target/hara-runtime-classpath.txt"
    truffle = str(ROOT / "java/target/classes")
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

    return {
        "clojure": lambda w, n, c: common(
            ["java", "-cp", clj_cp, "clojure.main", clj_script], "clojure", w, n, c),
        "bb": lambda w, n, c: common(["bb", clj_script], "bb", w, n, c),
        "hara-truffle": lambda w, n, c: common(
            ["java", "-cp", truffle_cp, "hara.truffle.Main", "benchmark"],
            "hara-truffle", w, n, c),
        "hara-native-image": lambda w, n, c: common(
            [str(ROOT / "target/hara-truffle"), "benchmark"],
            "hara-native-image", w, n, c),
        "hara-rust-native": lambda w, n, c: common(
            [str(ROOT / "rust/target/release/hara-runtime-benchmark")],
            "hara-rust-native", w, n, c, "hex"),
        "hara-wasm-node": lambda w, n, c: common(
            ["node", node_script], "hara-wasm-node", w, n, c),
    }, glue


def build(include_native):
    if include_native:
        run([str(ROOT / "scripts/build-truffle-native")], timeout=1200)
    run(["mvn", "-q", "-f", "java/pom.xml", "-Ptruffle", "-DskipTests", "compile",
         "dependency:build-classpath", "-Dmdep.outputFile=java/target/hara-runtime-classpath.txt"],
        timeout=300)
    run(["cargo", "build", "--manifest-path", "rust/Cargo.toml", "--release",
         "--bin", "hara-runtime-benchmark"], timeout=600)
    if shutil.which("wasm-bindgen"):
        run(["cargo", "build", "--manifest-path", "rust/Cargo.toml", "--release",
             "--target", "wasm32-unknown-unknown", "--lib"], timeout=600)
        (ROOT / "target/wasm-bindgen").mkdir(parents=True, exist_ok=True)
        run(["wasm-bindgen", "--target", "nodejs", "--out-dir", "target/wasm-bindgen",
             "rust/target/wasm32-unknown-unknown/release/hara_wasm.wasm"], timeout=300)


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
    if Path("/usr/bin/time").exists():
        handle = tempfile.NamedTemporaryFile(delete=False)
        handle.close()
        rss_file = Path(handle.name)
        actual = ["/usr/bin/time", "-f", "%M", "-o", str(rss_file)] + command
    started = time.perf_counter_ns()
    result = run(actual, env=env, timeout=180)
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
        "hara-truffle": truffle_files,
        "hara-native-image": [ROOT / "target/hara-truffle"],
        "hara-rust-native": [ROOT / "rust/target/release/hara-runtime-benchmark"],
        "hara-wasm-node": [ROOT / "rust/target/wasm32-unknown-unknown/release/hara_wasm.wasm", glue],
    }
    def size(path):
        if path.is_file(): return path.stat().st_size
        if path.is_dir(): return sum(item.stat().st_size for item in path.rglob("*") if item.is_file())
        return 0
    return {name: sum(size(path) for path in files)
            for name, files in paths.items()}


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
    lines += ["", "## Warm evaluation", "", "| Runtime / workload | First ms | Steady ms | calls/s | Converged window |", "|---|---:|---:|---:|---:|"]
    for row in data["measurements"]:
        convergence = row["analysis"]["converged_window"]
        lines.append(f"| {row['runtime']} / {row['workload']} | {row['first_ns']/1e6:.3f} | {row['analysis']['steady_ns']/1e6:.3f} | {row['analysis']['throughput_per_sec']:.1f} | {convergence if convergence is not None else '—'} |")
    lines += ["", "Warm values above are per-call milliseconds (the raw samples are stored as nanoseconds). Lower is better. Each adapter re-reads, evaluates, and checks the same source form, but this is an implementation snapshot rather than a source-normalized language shootout. Convergence is the first five-window run within ±5% of the final ten-window median with CV ≤10%.", ""]
    return "\n".join(lines)


def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--profile", choices=PROFILES, default="smoke")
    parser.add_argument("--runtime", action="append")
    parser.add_argument("--no-build", action="store_true")
    parser.add_argument("--reference", action="store_true")
    args = parser.parse_args()
    profile = PROFILES[args.profile]
    if not args.no_build:
        build(include_native=args.reference or (args.runtime and "hara-native-image" in args.runtime))
    runtime_adapters, glue = adapters()
    selected = args.runtime or list(runtime_adapters)
    missing = []
    if "hara-wasm-node" in selected and not glue.is_file():
        missing.append("hara-wasm-node (install version-matched wasm-bindgen-cli and rebuild)")
        selected.remove("hara-wasm-node")
    corpus = json.loads(CORPUS.read_text())["workloads"]
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
            _, _, result = timed(adapter(workload, profile["windows"], profile["calls"]), env)
            result["analysis"] = analyse(result["samples_ns"])
            measurements.append(result)
            print(f"{name:18} {workload['id']:18} {result['analysis']['steady_ns']/1e6:9.3f} ms")
    data = {"schema_version": 1, "profile": args.profile,
            "environment": {"timestamp": dt.datetime.now(dt.timezone.utc).isoformat(),
                            "platform": platform.platform(), "machine": platform.machine(),
                            "python": platform.python_version(),
                            "git_revision": version(["git", "rev-parse", "HEAD"]),
                            "git_dirty": bool(run(["git", "status", "--porcelain"]).stdout)},
            "versions": {"java": version(["java", "-version"]),
                         "clojure": "Clojure 1.12.5", "bb": version(["bb", "--version"]),
                         "rust": version(["rustc", "--version"]), "node": version(["node", "--version"]),
                         "native_image": version(["native-image", "--version"])},
            "missing": missing, "startup": startup, "measurements": measurements,
            "payload_bytes": payload_sizes(glue)}
    output = RESULTS if args.reference else ROOT / "target/runtime-benchmark.json"
    output.parent.mkdir(parents=True, exist_ok=True)
    output.write_text(json.dumps(data, indent=2) + "\n")
    report = markdown(data)
    report_path = REPORT if args.reference else ROOT / "target/runtime-benchmark.md"
    report_path.parent.mkdir(parents=True, exist_ok=True)
    report_path.write_text(report)
    if missing:
        print("Unavailable: " + ", ".join(missing), file=sys.stderr)
    print(f"wrote {output} and {report_path}")


if __name__ == "__main__":
    main()

#!/usr/bin/env python3
"""Render the Hara runtime and HTTP benchmark dashboard from JSON evidence."""
from __future__ import annotations
import argparse, json, statistics
from pathlib import Path

ROOT = Path(__file__).resolve().parents[2]
RUNTIMES = {
    "Rust": ["hara-rust-vm", "hara-rust-full"],
    "WebAssembly": ["hara-wasm-vm", "hara-wasm-full"],
    "Truffle": ["hara-truffle-jvm", "hara-truffle-native-vm", "hara-truffle-native-full"],
}
RUNTIME_SPECS = {
    "hara-rust-vm": ("Native", "Rust bytecode VM", "bytecode", "hara eval --engine vm"),
    "hara-rust-full": ("Native", "Rust whole-function compiler", "compiled Wasm", "hara eval --engine whole-wasm"),
    "hara-wasm-vm": ("Browser", "Rust bytecode VM in Wasm", "bytecode", "@hara-lang/browser/vm"),
    "hara-wasm-full": ("Browser", "Whole-function Wasm compiler", "compiled Wasm", "@hara-lang/browser/full"),
    "hara-truffle-jvm": ("JVM", "Graal/Truffle interpreter", "optimizing VM", "java … hara.truffle.Main"),
    "hara-truffle-native-vm": ("Native Image", "Truffle native fallback", "VM", "target/hara-truffle-native-vm"),
    "hara-truffle-native-full": ("Native Image", "Truffle native compiled tier", "compiled", "target/hara-truffle-native-full"),
}

def read(path): return json.loads(path.read_text()) if path and path.is_file() else {"measurements": []}

def rows(document):
    result = []
    for row in document.get("measurements", []):
        item = dict(row); item["runtime"] = item.get("runtime", "").removesuffix("-prepared")
        if "analysis" in item: item["steady_ns"] = item["analysis"].get("steady_ns")
        result.append(item)
    return result

def card(runtime, measurements):
    available = [row for row in measurements if row.get("runtime") == runtime and row.get("status", "ok") == "ok"]
    host, engine, tier, invocation = RUNTIME_SPECS[runtime]
    if runtime == "hara-wasm-vm": size = "3.04 MB ESM · 876 KB gzip"
    elif runtime == "hara-wasm-full": size = "3.30 MB ESM · 954 KB gzip"
    else: size = "Recorded by a standard publication run"
    state = "is-unavailable" if not available else ""
    headline = "NOT MEASURED" if not available else f'{statistics.median(row["steady_ns"] for row in available)/1e6:.3f} ms'
    body = [f'<article class="hara-benchmark-card {state}"><h3>{runtime}</h3><strong>{headline}</strong>',
            f'<dl><dt>Host</dt><dd>{host}</dd><dt>Engine</dt><dd>{engine}</dd><dt>Tier</dt><dd>{tier}</dd><dt>Artifact</dt><dd><code>{invocation}</code></dd><dt>Size</dt><dd>{size}</dd></dl>']
    if available:
        body += ['<table><thead><tr><th>Workload</th><th>Prepared</th><th>First</th></tr></thead><tbody>']
        for row in sorted(available, key=lambda item: item.get("workload", "")):
            steady = row.get("steady_ns")
            first = row.get("first_ns")
            body.append(f'<tr><td>{row.get("workload", "—")}</td><td>{steady/1e6:.4f} ms</td><td>{"—" if first is None else f"{first/1e6:.4f} ms"}</td></tr>')
        body.append('</tbody></table>')
    else:
        body.append('<p>This target is fully specified but was not built on the evidence host. A standard publication runner must supply its measurement artifact.</p>')
    body.append('</article>')
    return "".join(body)

def render(runtime_docs, browser_doc, http_doc):
    measurements = [row for document in runtime_docs for row in rows(document)] + rows(browser_doc)
    runtime_doc = runtime_docs[0] if runtime_docs else {}
    profile = runtime_doc.get("profile", browser_doc.get("profile", "unknown"))
    lines = ['<div class="hara-benchmark-page">', '<h1>Hara runtime benchmarks</h1>',
             '<p class="hara-benchmark-lede">VM and compiled tiers measured with checksum-verified workloads. Lower prepared-call time is better.</p>',
             f'<p><strong>Evidence profile:</strong> {profile}. Standard-profile runs are required before publishing reference thresholds.</p>',
             '<div class="hara-benchmark-hosts">']
    for host, runtimes in RUNTIMES.items():
        lines += [f'<section class="hara-benchmark-host"><h2>{host}</h2>']
        lines += [card(runtime, measurements) for runtime in runtimes]
        lines.append('</section>')
    lines.append('</div>')
    concurrency = http_doc.get("configuration", {}).get("concurrency", "recorded per run")
    lines += ['<h2>HTTP frameworks</h2>', f'<p>Single worker/event loop, loopback HTTP/1.1, keep-alive, concurrency {concurrency}.</p>',
              '<table><thead><tr><th>Route</th><th>Server</th><th>Requests/sec</th><th>p50 ms</th><th>p95 ms</th><th>p99 ms</th></tr></thead><tbody>']
    for row in http_doc.get("measurements", []):
        if row.get("status") != "ok": values = ('not applicable', '—', '—', '—')
        else: values = (f"{row['requests_per_second']:.1f}", f"{row['p50_ms']:.3f}", f"{row['p95_ms']:.3f}", f"{row['p99_ms']:.3f}")
        lines.append(f"<tr><td>{row['route']}</td><td>{row['server']}</td>{''.join(f'<td>{value}</td>' for value in values)}</tr>")
    lines += ['</tbody></table>', '<h2>Methodology</h2>', '<p>Every adapter checks the same expected result. Compilation, first execution, and prepared execution are separate boundaries. Browser windows use adaptive batching; HTTP values are medians across trials. Every target remains fully specified even when a particular evidence host cannot build it.</p>', '</div>']
    return "\n".join(lines) + "\n"

def main():
    parser = argparse.ArgumentParser()
    parser.add_argument("--runtime", type=Path, action="append",
                        default=[ROOT / "target/hara-rust-vm-vs-full-smoke.json"])
    parser.add_argument("--browser", type=Path, default=ROOT / "target/browser-tier-benchmark.json")
    parser.add_argument("--http", type=Path, default=ROOT / "target/hara-http-frameworks.json")
    parser.add_argument("--output", type=Path, default=ROOT / "docs/docs/reference/runtime-benchmarks.md")
    parser.add_argument("--standalone-output", type=Path,
                        default=ROOT / "benchmarks/docs/index.md")
    args = parser.parse_args()
    runtime_paths = list(args.runtime)
    truffle = ROOT / "target/hara-truffle-jvm-smoke.json"
    if truffle.is_file() and truffle not in runtime_paths:
        runtime_paths.append(truffle)
    report = render([read(path) for path in runtime_paths], read(args.browser), read(args.http))
    args.output.write_text(report)
    args.standalone_output.parent.mkdir(parents=True, exist_ok=True)
    args.standalone_output.write_text(report)
    print(f"wrote {args.output}")
    print(f"wrote {args.standalone_output}")

if __name__ == "__main__": main()

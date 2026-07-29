#!/usr/bin/env python3
"""Generate and validate the canonical Clojure/Hara core symbol grouping."""

from __future__ import annotations

import json
import os
import re
import subprocess
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
SPEC = ROOT / "specs/archive/planning/language/compatibility"
DOC = ROOT / "docs/docs/reference/clojure-core-compatibility.md"
CLJ_VERSION = "1.12.5"
CLJ_SPECIALS = {"def", "if", "do", "let", "quote", "var", "fn", "loop", "recur", "throw", "try", "new", "set!", "monitor-enter", "monitor-exit", "catch", "finally"}
HARA_SPECIALS = {
    "and", "binding", "catch", "cond", "declare", "def", "defmacro",
    "defmethod", "defmulti", "defn", "defn-", "defprotocol", "defstruct",
    "deref", "do", "extend-type", "finally", "fn", "if", "let", "letfn",
    "loop", "new", "ns", "or", "quote", "recur", "set!", "throw", "try",
    "var", "when", "when-not",
}


def execute(command):
    result = subprocess.run(command, cwd=ROOT, text=True, capture_output=True, check=True)
    return result.stdout.strip()


def java_executable():
    java_home = os.environ.get("JAVA_HOME")
    if java_home:
        java = Path(java_home) / "bin/java"
        if java.exists():
            return str(java)
    return "java"


def clojure_classpath():
    base = Path.home() / ".m2/repository/org/clojure"
    files = [base / f"clojure/{CLJ_VERSION}/clojure-{CLJ_VERSION}.jar",
             base / "spec.alpha/0.5.238/spec.alpha-0.5.238.jar",
             base / "core.specs.alpha/0.4.74/core.specs.alpha-0.4.74.jar"]
    return os.pathsep.join(map(str, files))


def clojure_symbols():
    core_jar = Path.home() / f".m2/repository/org/clojure/clojure/{CLJ_VERSION}/clojure-{CLJ_VERSION}.jar"
    if not core_jar.exists():
        inventory = json.loads((SPEC / "clojure-core-symbols.json").read_text())
        if inventory["version"] != CLJ_VERSION:
            raise SystemExit(f"Clojure {CLJ_VERSION} is unavailable and the stored inventory has a different version")
        return set(inventory["symbols"])
    expression = "(doseq [x (sort (map str (keys (ns-publics 'clojure.core))))] (println x))"
    values = set(execute([java_executable(), "-cp", clojure_classpath(), "clojure.main", "-e", expression]).splitlines())
    return values | CLJ_SPECIALS


def hara_symbols():
    runtime = ROOT / "java/target/hara-truffle.jar"
    if not runtime.exists():
        raise SystemExit("build java/target/hara-truffle.jar before generating compatibility data")
    values = set()
    for namespace in ("std.foundation", "hara.lang.intrinsic"):
        expression = (
            "(reduce-kv "
            "(fn [out name value] "
            "(if (get (meta value) :private) out (conj out name))) "
            f"[] (ns-publics '{namespace}))"
        )
        output = execute([
            java_executable(),
            "-Dpolyglot.engine.WarnInterpreterOnly=false",
            "-jar",
            str(runtime),
            "eval",
            expression,
        ])
        values |= set(re.findall(r'[^\s\[\]]+', output))
    values.add("IFind/has?")
    return values | HARA_SPECIALS


def rust_symbols(canonical):
    execute(["cargo", "build", "--quiet", "--manifest-path", "rust/Cargo.toml", "--bin", "hara"])
    runtime = ROOT / "rust/target/debug/hara"
    fiber = (ROOT / "rust/src/fiber.rs").read_text()
    completion_block = fiber.split("const CORE_SPECIAL_FORMS:", 1)[1].split("];", 1)[0]
    values = set(re.findall(r'"([^"]+)"', completion_block))
    candidates = sorted(canonical - HARA_SPECIALS)
    probes = " ".join(
        f"(try (do {name} '{name}) (catch compatibility-error nil))"
        for name in candidates
    )
    output = execute([str(runtime), "eval", f"[{probes}]"])
    values |= {
        value
        for value in re.findall(r'[^\s\[\]]+', output)
        if value != "nil"
    }
    return values | HARA_SPECIALS


def write_json(path, value):
    path.write_text(json.dumps(value, indent=2, sort_keys=True) + "\n")


def main():
    clojure = clojure_symbols()
    hara = hara_symbols()
    rust = rust_symbols(hara)
    overrides = json.loads((SPEC / "clojure-core-compatibility-overrides.json").read_text())
    used_clojure = set()
    used_hara = set()
    for relation in overrides["renamed"] + overrides["changed"]:
        c, h = relation["clojure"], relation["hara"]
        if c not in clojure: raise SystemExit(f"unknown Clojure symbol in override: {c}")
        if h not in hara: raise SystemExit(f"unknown Hara symbol in override: {h}")
        if c in used_clojure or h in used_hara: raise SystemExit(f"duplicate relationship: {c}/{h}")
        used_clojure.add(c); used_hara.add(h)
    exact = sorted((clojure & hara) - used_clojure - used_hara)
    used_clojure.update(exact); used_hara.update(exact)
    grouping = {
        "schema_version": 1,
        "clojure_version": CLJ_VERSION,
        "hara_surface": "L0 plus eagerly referred std.foundation",
        "groups": {
            "only-clojure": sorted(clojure - used_clojure),
            "only-hara": sorted(hara - used_hara),
            "same-exact": exact,
            "same-renamed": overrides["renamed"],
            "same-changed": overrides["changed"],
        },
        "runtime_drift": {
            "truffle": {"missing": [], "extra": []},
            "rust-native": {"missing": sorted(hara - rust), "extra": sorted(rust - hara)},
            "wasm": {"missing": sorted(hara - rust), "extra": sorted(rust - hara)},
        },
    }
    all_c = set(grouping["groups"]["only-clojure"]) | set(exact) | {x["clojure"] for x in overrides["renamed"] + overrides["changed"]}
    all_h = set(grouping["groups"]["only-hara"]) | set(exact) | {x["hara"] for x in overrides["renamed"] + overrides["changed"]}
    if all_c != clojure or all_h != hara: raise SystemExit("compatibility grouping is not exhaustive")
    write_json(SPEC / "clojure-core-symbols.json", {"version": CLJ_VERSION, "symbols": sorted(clojure)})
    write_json(SPEC / "hal-core-symbols.json", {"surface": grouping["hara_surface"], "symbols": sorted(hara)})
    write_json(SPEC / "clojure-core-compatibility.json", grouping)
    groups = grouping["groups"]
    lines = ["# Clojure core / Hara core compatibility", "", f"Canonical exhaustive grouping for Clojure {CLJ_VERSION} and Hara L0 plus `std.foundation`.", "",
             "| Group | Count |", "|---|---:|"]
    for name in ("only-clojure", "only-hara", "same-exact", "same-renamed", "same-changed"):
        lines.append(f"| `{name}` | {len(groups[name])} |")
    for name in ("same-changed", "same-renamed"):
        lines += ["", f"## {name}", "", "| Clojure | Hara | Contract |", "|---|---|---|"]
        for item in groups[name]: lines.append(f"| `{item['clojure']}` | `{item['hara']}` | {item['summary']} |")
    for name in ("only-clojure", "only-hara", "same-exact"):
        lines += ["", f"## {name}", "", ", ".join(f"`{x}`" for x in groups[name]), ""]
    lines += ["## Runtime drift", "", "| Runtime | Missing canonical | Extra implementation |", "|---|---:|---:|"]
    for name, drift in grouping["runtime_drift"].items(): lines.append(f"| {name} | {len(drift['missing'])} | {len(drift['extra'])} |")
    lines += [
        "",
        "## Parity and transport notes",
        "",
        "The Java/Truffle and Rust runtimes share the same Foundation mapping contract.",
        "Parity coverage includes `odd?`, `update`, and direct/curried/lazy mapping",
        "semantics. If an older packaged runtime disagrees, rebuild it from the current",
        "Foundation source: stale embedded artifacts are the usual cause.",
        "",
        "`HTA1` transports portable values only. It explicitly rejects `Seq` and raw",
        "iterator values; materialize them with `vec` before sending them across an HTA",
        "boundary. Internal `iter-*` helpers remain implementation-level cleanup work,",
        "not the recommended public data-transport surface.",
    ]
    DOC.write_text("\n".join(lines) + "\n")
    print(f"wrote canonical grouping: {len(clojure)} Clojure, {len(hara)} Hara symbols")


if __name__ == "__main__":
    main()

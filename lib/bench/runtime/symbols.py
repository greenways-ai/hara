#!/usr/bin/env python3
"""Generate and validate the canonical Clojure/Hara core symbol grouping."""

from __future__ import annotations

import json
import os
import re
import subprocess
from pathlib import Path

ROOT = Path(__file__).resolve().parents[3]
SPEC = ROOT / "specs/hara/data"
DOC = ROOT / "website/docs/reference/clojure-core-compatibility.md"
CLJ_SPECIALS = {"def", "if", "do", "let", "quote", "var", "fn", "loop", "recur", "throw", "try", "new", "set!", "monitor-enter", "monitor-exit", "catch", "finally"}
HARA_SPECIALS = {"def", "if", "do", "let", "quote", "var", "fn", "loop", "recur", "throw", "try", "catch", "finally", "binding", "defn", "defmacro", "defprotocol", "extend-type", "defstruct", "defmulti", "defmethod", "ns"}


def execute(command):
    result = subprocess.run(command, cwd=ROOT, text=True, capture_output=True, check=True)
    return result.stdout.strip()


def clojure_classpath():
    base = Path.home() / ".m2/repository/org/clojure"
    files = [base / "clojure/1.12.5/clojure-1.12.5.jar",
             base / "spec.alpha/0.5.238/spec.alpha-0.5.238.jar",
             base / "core.specs.alpha/0.4.74/core.specs.alpha-0.4.74.jar"]
    return os.pathsep.join(map(str, files))


def clojure_symbols():
    expression = "(doseq [x (sort (map str (keys (ns-publics 'clojure.core))))] (println x))"
    values = set(execute(["java", "-cp", clojure_classpath(), "clojure.main", "-e", expression]).splitlines())
    return values | CLJ_SPECIALS


def hara_symbols():
    cp = str(ROOT / "java/target/classes")
    cp_file = ROOT / "java/target/hara-runtime-classpath.txt"
    if cp_file.exists(): cp += os.pathsep + cp_file.read_text().strip()
    values = set()
    for namespace in ("std.lib.foundation", "hara.lang.intrinsic"):
        output = execute(["java", "-cp", cp, "hara.truffle.Main", "eval",
                          f"(vec (keys (ns-publics '{namespace})))"])
        values |= set(re.findall(r'[^\s\[\]]+', output))
    values.add("IFind/has?")
    return values | HARA_SPECIALS


def rust_symbols():
    core = (ROOT / "rust/src/core.rs").read_text()
    foundation = (ROOT / "lib/src/std/lib/foundation.hal").read_text()
    names = set(re.findall(r'native_function\("([^"]+)"', core))
    names |= set(re.findall(r'^\(defn?\s+([^\s\[]+)', foundation, re.MULTILINE))
    names.add("IFind/has?")
    return names | HARA_SPECIALS


def write_json(path, value):
    path.write_text(json.dumps(value, indent=2, sort_keys=True) + "\n")


def main():
    clojure = clojure_symbols()
    hara = hara_symbols()
    rust = rust_symbols()
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
        "clojure_version": "1.12.5",
        "hara_surface": "L0 plus eagerly referred std.lib.foundation",
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
    write_json(SPEC / "clojure-core-symbols.json", {"version": "1.12.5", "symbols": sorted(clojure)})
    write_json(SPEC / "hara-core-symbols.json", {"surface": grouping["hara_surface"], "symbols": sorted(hara)})
    write_json(SPEC / "clojure-core-compatibility.json", grouping)
    groups = grouping["groups"]
    lines = ["# Clojure core / Hara core compatibility", "", "Canonical exhaustive grouping for Clojure 1.12.5 and Hara L0 plus `std.lib.foundation`.", "",
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
    DOC.write_text("\n".join(lines) + "\n")
    print(f"wrote canonical grouping: {len(clojure)} Clojure, {len(hara)} Hara symbols")


if __name__ == "__main__":
    main()

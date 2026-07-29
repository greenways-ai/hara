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
HARA_INTERNAL_FUNCTIONS = {
    "comp2",
    "comp3",
    "iter-any?",
    "iter-close",
    "iter-constantly",
    "iter-cycle",
    "iter-drop",
    "iter-drop-while",
    "iter-every?",
    "iter-filter",
    "iter-has?",
    "iter-interleave",
    "iter-interpose",
    "iter-iterate",
    "iter-keep",
    "iter-map",
    "iter-mapcat",
    "iter-next",
    "iter-partition",
    "iter-partition-all",
    "iter-partition-pair",
    "iter-range",
    "iter-repeatedly",
    "iter-take",
    "iter-take-while",
    "iter-zip",
    "load-file",
    "map-transform",
    "module-dependencies",
    "module-revision",
    "partition-all-transform",
    "partition-transform",
    "requiring-resolve",
}
HARA_LIBRARIES = {
    "std.foundation.string": "str",
    "std.foundation.bytes": "bytes",
    "std.foundation.coroutine": "co",
    "std.foundation.promise": "promise",
    "std.foundation.file": "file",
    "std.foundation.json": "json",
    "std.foundation.os": "os",
    "std.foundation.socket": "socket",
    "std.foundation.set": "set",
    "std.pretty": "pretty",
}
HARA_NATIVE_LIBRARY_DECLARATIONS = {
    "json": {"read", "write", "write-pp"},
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


def hara_eval(expression):
    runtime = ROOT / "java/target/hara-truffle.jar"
    if not runtime.exists():
        raise SystemExit("build java/target/hara-truffle.jar before generating compatibility data")
    return execute([
        java_executable(),
        "-Dpolyglot.engine.WarnInterpreterOnly=false",
        "-jar",
        str(runtime),
        "eval",
        expression,
    ])


def hara_namespace_publics(namespace):
    expression = (
        "(reduce-kv "
        "(fn [out name value] "
        "(if (get (meta value) :private) out (conj out name))) "
        f"[] (ns-publics '{namespace}))"
    )
    return set(re.findall(r'[^\s\[\]]+', hara_eval(expression)))


def hal_public_definitions(path):
    source = path.read_text()
    definitions = set()
    for match in re.finditer(r'(?m)^\((defn-?|defmacro|def)\s+', source):
        if match.group(1) == "defn-":
            continue
        tail = source[match.end():].lstrip()
        if tail.startswith("^"):
            opening = tail.find("{")
            depth = 0
            closing = None
            for index, character in enumerate(tail[opening:], opening):
                if character == "{":
                    depth += 1
                elif character == "}":
                    depth -= 1
                    if depth == 0:
                        closing = index
                        break
            if closing is None:
                raise SystemExit(f"unclosed metadata in {path}")
            tail = tail[closing + 1:].lstrip()
        name = re.match(r'([^\s\[\]()]+)', tail)
        if name:
            definitions.add(name.group(1))
    return definitions


def hal_builtin_declarations(path):
    source = path.read_text()
    match = re.search(r':builtins\s*\[(.*?)\]', source, re.DOTALL)
    return set() if match is None else set(re.findall(r'[^\s\[\]]+', match.group(1)))


def hara_inventory():
    foundation = hara_namespace_publics("std.foundation")
    intrinsic = hara_namespace_publics("hara.lang.intrinsic")
    protocols = {name for name in intrinsic if name.startswith("I")}
    functions = (
        foundation
        | (intrinsic - protocols)
    ) - HARA_SPECIALS - HARA_INTERNAL_FUNCTIONS

    current_symbols = set(re.findall(r'"([^"]+)"', hara_eval("(vec (current-symbols))")))
    libraries = {}
    for namespace, alias in HARA_LIBRARIES.items():
        path = ROOT / "lib/src" / Path(*namespace.split(".")).with_suffix(".hal")
        names = set(HARA_NATIVE_LIBRARY_DECLARATIONS.get(alias, set()))
        if path.exists():
            names |= hal_builtin_declarations(path) | hal_public_definitions(path)
        names |= {
            symbol.split("/", 1)[1]
            for symbol in current_symbols
            if symbol.startswith(f"{alias}/")
        }
        libraries[alias] = sorted(names)

    return {
        "functions": functions,
        "special_forms": set(HARA_SPECIALS),
        "protocols": protocols,
        "libraries": libraries,
    }


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
    inventory = hara_inventory()
    hara = inventory["functions"] | inventory["special_forms"]
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
        "schema_version": 2,
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
    write_json(SPEC / "hal-core-symbols.json", {
        "schema_version": 2,
        "surface": grouping["hara_surface"],
        "symbols": sorted(hara),
        "functions": sorted(inventory["functions"]),
        "special_forms": sorted(inventory["special_forms"]),
        "protocols": sorted(inventory["protocols"]),
        "libraries": inventory["libraries"],
    })
    write_json(SPEC / "clojure-core-compatibility.json", grouping)
    groups = grouping["groups"]
    library_count = sum(len(names) for names in inventory["libraries"].values())
    lines = ["# Clojure core / Hara core compatibility", "", f"Canonical exhaustive grouping for Clojure {CLJ_VERSION} and Hara L0 plus `std.foundation`.", "",
             "| Group | Count |", "|---|---:|"]
    for name in ("only-clojure", "only-hara", "same-exact", "same-renamed", "same-changed"):
        lines.append(f"| `{name}` | {len(groups[name])} |")
    for name in ("same-changed", "same-renamed"):
        lines += ["", f"## {name}", "", "| Clojure | Hara | Contract |", "|---|---|---|"]
        for item in groups[name]: lines.append(f"| `{item['clojure']}` | `{item['hara']}` | {item['summary']} |")
    lines += [
        "",
        "## Hara public surface",
        "",
        "| Kind | Count |",
        "|---|---:|",
        f"| Core functions and macros | {len(inventory['functions'])} |",
        f"| Special forms | {len(inventory['special_forms'])} |",
        f"| Protocols | {len(inventory['protocols'])} |",
        f"| Namespaced library functions | {library_count} |",
    ]
    for name in ("only-clojure", "only-hara", "same-exact"):
        title = "only-hara functions" if name == "only-hara" else name
        lines += ["", f"## {title}", "", ", ".join(f"`{x}`" for x in groups[name]), ""]
    lines += [
        "## Hara protocols",
        "",
        ", ".join(f"`{name}`" for name in sorted(inventory["protocols"])),
        "",
        "## Hara namespaced libraries",
        "",
        "| Alias | Public functions |",
        "|---|---|",
    ]
    for alias, names in inventory["libraries"].items():
        lines.append(f"| `{alias}` | {', '.join(f'`{alias}/{name}`' for name in names)} |")
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
    print(
        f"wrote canonical grouping: {len(clojure)} Clojure, "
        f"{len(inventory['functions'])} Hara functions, "
        f"{len(inventory['protocols'])} protocols, "
        f"{library_count} namespaced functions"
    )


if __name__ == "__main__":
    main()

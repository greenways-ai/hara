#!/usr/bin/env python3
import json
import sys
import time


def main():
    if len(sys.argv) != 7:
        raise SystemExit("python_runner expects MODE ID SOURCE_HEX EXPECTED WINDOWS CALLS")
    mode, workload, source_hex, expected, windows_text, calls_text = sys.argv[1:]
    source = bytes.fromhex(source_hex).decode()
    sys.setrecursionlimit(100_000)
    windows, calls = int(windows_text), int(calls_text)
    prepared = None
    if mode == "prepared":
        scope = {}
        exec(compile(source, workload, "exec"), scope)
        prepared = scope["benchmark"]

    def evaluate():
        if prepared is None:
            scope = {}
            exec(compile(source, workload, "exec"), scope)
            value = scope["benchmark"]()
        else:
            value = prepared()
        if str(value) != expected:
            raise SystemExit(f"{workload}: expected {expected}, got {value}")

    started = time.perf_counter_ns()
    evaluate()
    first_ns = time.perf_counter_ns() - started
    samples = []
    for _ in range(windows):
        started = time.perf_counter_ns()
        for _ in range(calls):
            evaluate()
        samples.append((time.perf_counter_ns() - started) // calls)
    print(json.dumps({"runtime": "python", "workload": workload,
                      "first_ns": first_ns, "samples_ns": samples},
                     separators=(",", ":")))


if __name__ == "__main__":
    main()

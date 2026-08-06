#!/usr/bin/env python3
import subprocess
import sys
import tempfile
from pathlib import Path


TEMPLATE = r'''#include <inttypes.h>
#include <stdint.h>
#include <stdio.h>
#include <stdlib.h>
#include <time.h>
volatile int64_t benchmark_seed = 0;
{source}
static uint64_t now_ns(void) {{
  struct timespec value;
  clock_gettime(CLOCK_MONOTONIC_RAW, &value);
  return (uint64_t)value.tv_sec * 1000000000ULL + (uint64_t)value.tv_nsec;
}}
int main(int argc, char **argv) {{
  const char *id = argv[1];
  int64_t expected = strtoll(argv[2], NULL, 10);
  int windows = atoi(argv[3]), calls = atoi(argv[4]);
  uint64_t started = now_ns();
  int64_t value = benchmark();
  uint64_t first = now_ns() - started;
  if (value != expected) {{ fprintf(stderr, "%s: expected %" PRId64 ", got %" PRId64 "\n", id, expected, value); return 1; }}
  printf("{{\"runtime\":\"c\",\"workload\":\"%s\",\"first_ns\":%" PRIu64 ",\"samples_ns\":[", id, first);
  for (int window = 0; window < windows; window++) {{
    started = now_ns();
    for (int call = 0; call < calls; call++) value = benchmark();
    uint64_t sample = (now_ns() - started) / (uint64_t)calls;
    if (value != expected) return 1;
    printf("%s%" PRIu64, window ? "," : "", sample);
  }}
  puts("]}}");
  return 0;
}}
'''


def main():
    if len(sys.argv) != 7:
        raise SystemExit("c_runner expects MODE ID SOURCE_HEX EXPECTED WINDOWS CALLS")
    _, workload, source_hex, expected, windows, calls = sys.argv[1:]
    source = bytes.fromhex(source_hex).decode()
    with tempfile.TemporaryDirectory(prefix="hara-c-bench-") as directory:
        root = Path(directory)
        source_path, binary = root / "benchmark.c", root / "benchmark"
        source_path.write_text(TEMPLATE.format(source=source))
        subprocess.run(["cc", "-O3", "-std=c11", str(source_path), "-o", str(binary)], check=True)
        completed = subprocess.run([str(binary), workload, expected, windows, calls])
        raise SystemExit(completed.returncode)


if __name__ == "__main__":
    main()

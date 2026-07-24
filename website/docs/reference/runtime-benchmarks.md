# Runtime benchmark reference

Generated: `2026-07-24T14:18:51.332929+00:00` on `Linux-6.17.0-40-generic-x86_64-with-glibc2.39`.

Values are machine-specific evidence, not regression thresholds.
The Truffle/JVM row used the fallback interpreter because this Temurin JVM has no JVMCI compiler.

## Startup

| Runtime | p50 ms | p95 ms | Peak RSS MiB | Payload MiB |
|---|---:|---:|---:|---:|
| clojure | 463.67 | 488.21 | 110.1 | 4.6 |
| bb | 12.65 | 19.39 | 36.2 | 65.5 |
| hara-truffle | 310.99 | 328.15 | 95.8 | 33.7 |
| hara-native-image | 11.98 | 15.88 | 38.3 | 59.7 |
| hara-rust-native | 4.73 | 8.97 | 5.0 | 8.7 |
| hara-wasm-node | 37.00 | 39.36 | 62.9 | 1.2 |

## Warm evaluation

| Runtime / workload | First ms | Steady ms | calls/s | Converged window |
|---|---:|---:|---:|---:|
| clojure / noop | 0.181 | 0.022 | 46377.9 | — |
| clojure / arithmetic | 3.712 | 0.892 | 1120.7 | 41 |
| clojure / function-call | 4.243 | 0.813 | 1230.4 | 41 |
| clojure / persistent-vector | 3.320 | 0.507 | 1974.1 | 52 |
| clojure / persistent-map | 1.371 | 0.452 | 2213.5 | 52 |
| clojure / sequence-navigation | 2.505 | 0.450 | 2222.2 | 55 |
| bb / noop | 0.020 | 0.009 | 112555.6 | 40 |
| bb / arithmetic | 1.492 | 0.477 | 2094.7 | 22 |
| bb / function-call | 0.394 | 0.227 | 4409.5 | 25 |
| bb / persistent-vector | 0.171 | 0.119 | 8400.0 | 46 |
| bb / persistent-map | 0.221 | 0.110 | 9076.3 | 34 |
| bb / sequence-navigation | 0.162 | 0.086 | 11674.3 | 41 |
| hara-truffle / noop | 83.823 | 0.014 | 73182.3 | 53 |
| hara-truffle / arithmetic | 95.989 | 4.057 | 246.5 | 13 |
| hara-truffle / function-call | 110.891 | 1.943 | 514.8 | 29 |
| hara-truffle / persistent-vector | 109.479 | 1.438 | 695.3 | 21 |
| hara-truffle / persistent-map | 96.661 | 1.352 | 739.5 | 28 |
| hara-truffle / sequence-navigation | 109.354 | 1.099 | 909.8 | 35 |
| hara-native-image / noop | 6.051 | 0.001 | 1362397.8 | — |
| hara-native-image / arithmetic | 13.188 | 4.566 | 219.0 | 50 |
| hara-native-image / function-call | 11.186 | 2.415 | 414.2 | 22 |
| hara-native-image / persistent-vector | 7.991 | 2.286 | 437.4 | 1 |
| hara-native-image / persistent-map | 7.037 | 1.927 | 519.0 | 10 |
| hara-native-image / sequence-navigation | 7.974 | 1.667 | 599.9 | 33 |
| hara-rust-native / noop | 0.120 | 0.040 | 24794.2 | 10 |
| hara-rust-native / arithmetic | 4.144 | 3.655 | 273.6 | 8 |
| hara-rust-native / function-call | 29.413 | 29.627 | 33.8 | 31 |
| hara-rust-native / persistent-vector | 1.015 | 0.927 | 1078.6 | 3 |
| hara-rust-native / persistent-map | 1.255 | 1.087 | 919.7 | 7 |
| hara-rust-native / sequence-navigation | 0.604 | 0.563 | 1775.8 | 0 |
| hara-wasm-node / noop | 0.329 | 0.042 | 23667.2 | 43 |
| hara-wasm-node / arithmetic | 8.940 | 4.174 | 239.6 | 3 |
| hara-wasm-node / function-call | 27.543 | 19.903 | 50.2 | 0 |
| hara-wasm-node / persistent-vector | 3.568 | 0.962 | 1039.0 | 20 |
| hara-wasm-node / persistent-map | 7.522 | 1.234 | 810.3 | 4 |
| hara-wasm-node / sequence-navigation | 2.924 | 0.617 | 1620.1 | 27 |

Warm samples are per-call nanoseconds. Convergence is the first five-window run within ±5% of the final ten-window median with CV ≤10%.

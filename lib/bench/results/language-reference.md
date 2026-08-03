# Lisp vs Hara (Rust native) benchmark

Generated: `2026-08-03T13:18:27.721193+00:00` on `macOS-26.5.2-arm64-arm-64bit-Mach-O`.

Values are machine-specific comparison evidence, not regression thresholds. Every row prepares its program once and invokes it repeatedly. Hara is represented only by `hara-rust-full`.

## Startup

| Runtime | p50 ms | p95 ms |
|---|---:|---:|
| sbcl-prepared | 20.79 | 38.81 |
| chez-prepared | 31.09 | 37.37 |
| guile-prepared | 27.08 | 36.49 |
| bb-prepared | 19.93 | 63.76 |
| python-prepared | 16.82 | 18.22 |
| c-prepared | 216.02 | 423.47 |
| java-prepared | 254.18 | 281.94 |
| luajit-prepared | 2.27 | 2.95 |
| hara-rust-full | 35.68 | 50.72 |

## Warm evaluation

| Runtime / workload | First ms | Steady ms | ns/iteration | calls/s | Converged window |
|---|---:|---:|---:|---:|---:|
| sbcl-prepared / sieve-array | 0.005 | 0.001 | 2.15 | 909090.9 | 24 |
| sbcl-prepared / towers-recursive | 2.812 | 2.523 | 9.63 | 396.3 | 6 |
| sbcl-prepared / queens-backtracking | 0.470 | 0.435 | 27.67 | 2298.9 | 0 |
| sbcl-prepared / heap-permute | 0.828 | 0.835 | 20.70 | 1197.9 | 0 |
| sbcl-prepared / ackermann-deep | 19.357 | 20.157 | 7.24 | 49.6 | 0 |
| sbcl-prepared / tak-branching | 0.275 | 0.274 | 4.31 | 3647.6 | 0 |
| sbcl-prepared / collatz-range | 4.812 | 4.847 | 5.70 | 206.3 | 0 |
| sbcl-prepared / matrix-multiply | 0.045 | 0.037 | 5.79 | 26990.6 | 4 |
| chez-prepared / sieve-array | 0.027 | 0.001 | 1.56 | 1250000.0 | 8 |
| chez-prepared / towers-recursive | 0.715 | 0.700 | 2.67 | 1429.1 | 0 |
| chez-prepared / queens-backtracking | 0.245 | 0.136 | 8.63 | 7369.2 | 50 |
| chez-prepared / heap-permute | 0.488 | 0.484 | 12.01 | 2065.9 | 0 |
| chez-prepared / ackermann-deep | 2.293 | 2.430 | 0.87 | 411.5 | 2 |
| chez-prepared / tak-branching | 0.079 | 0.057 | 0.90 | 17559.3 | 2 |
| chez-prepared / collatz-range | 4.913 | 5.083 | 5.98 | 196.8 | 0 |
| chez-prepared / matrix-multiply | 0.042 | 0.023 | 3.53 | 44247.8 | 23 |
| guile-prepared / sieve-array | 0.341 | 0.066 | 129.39 | 15094.3 | 33 |
| guile-prepared / towers-recursive | 62.805 | 60.885 | 232.26 | 16.4 | 0 |
| guile-prepared / queens-backtracking | 41.357 | 39.074 | 2485.62 | 25.6 | 0 |
| guile-prepared / heap-permute | 74.761 | 83.366 | 2067.61 | 12.0 | 17 |
| guile-prepared / ackermann-deep | 289.178 | 309.774 | 111.19 | 3.2 | 8 |
| guile-prepared / tak-branching | 8.714 | 7.295 | 114.68 | 137.1 | 9 |
| guile-prepared / collatz-range | 149.597 | 157.271 | 185.10 | 6.4 | 28 |
| guile-prepared / matrix-multiply | 3.398 | 6.723 | 1050.46 | 148.7 | 25 |
| bb-prepared / sieve-array | 0.069 | 0.024 | 47.44 | 41167.5 | 13 |
| bb-prepared / towers-recursive | 43.131 | 25.489 | 97.23 | 39.2 | 16 |
| bb-prepared / queens-backtracking | 6.845 | 6.380 | 405.88 | 156.7 | 2 |
| bb-prepared / heap-permute | 14.051 | 13.948 | 345.94 | 71.7 | 3 |
| bb-prepared / ackermann-deep | 144.522 | 142.650 | 51.20 | 7.0 | 0 |
| bb-prepared / tak-branching | 2.544 | 2.324 | 36.53 | 430.4 | 3 |
| bb-prepared / collatz-range | 41.086 | 40.346 | 47.48 | 24.8 | 0 |
| bb-prepared / matrix-multiply | 1.531 | 1.021 | 159.58 | 979.1 | 0 |
| python-prepared / sieve-array | 0.007 | 0.004 | 7.02 | 278280.2 | 5 |
| python-prepared / towers-recursive | 12.760 | 12.615 | 48.12 | 79.3 | 0 |
| python-prepared / queens-backtracking | 4.732 | 4.879 | 310.37 | 205.0 | 0 |
| python-prepared / heap-permute | 9.821 | 6.466 | 160.37 | 154.7 | 2 |
| python-prepared / ackermann-deep | 92.194 | 91.527 | 32.85 | 10.9 | 1 |
| python-prepared / tak-branching | 1.378 | 1.429 | 22.47 | 699.8 | 0 |
| python-prepared / collatz-range | 36.172 | 34.232 | 40.29 | 29.2 | 0 |
| python-prepared / matrix-multiply | 0.519 | 0.507 | 79.25 | 1971.5 | 0 |
| c-prepared / sieve-array | 0.001 | 0.000 | 0.32 | 6097561.0 | 1 |
| c-prepared / towers-recursive | 0.659 | 0.245 | 0.93 | 4084.3 | 23 |
| c-prepared / queens-backtracking | 0.161 | 0.049 | 3.09 | 20605.4 | 51 |
| c-prepared / heap-permute | 1.332 | 0.095 | 2.36 | 10496.4 | 46 |
| c-prepared / ackermann-deep | 11.389 | 6.861 | 2.46 | 145.8 | 1 |
| c-prepared / tak-branching | 0.147 | 0.066 | 1.04 | 15079.2 | 52 |
| c-prepared / collatz-range | 0.610 | 0.605 | 0.71 | 1653.0 | 10 |
| c-prepared / matrix-multiply | 0.005 | 0.003 | 0.42 | 371333.1 | 18 |
| java-prepared / sieve-array | 0.011 | 0.002 | 3.00 | 651465.8 | 28 |
| java-prepared / towers-recursive | 0.539 | 0.318 | 1.21 | 3142.3 | 0 |
| java-prepared / queens-backtracking | 0.393 | 0.033 | 2.10 | 30347.2 | 4 |
| java-prepared / heap-permute | 0.392 | 0.098 | 2.42 | 10234.2 | 1 |
| java-prepared / ackermann-deep | 4.669 | 4.230 | 1.52 | 236.4 | 26 |
| java-prepared / tak-branching | 0.237 | 0.039 | 0.61 | 25907.1 | 20 |
| java-prepared / collatz-range | 2.401 | 0.894 | 1.05 | 1119.1 | 1 |
| java-prepared / matrix-multiply | 0.081 | 0.001 | 0.18 | 882612.5 | — |
| luajit-prepared / sieve-array | 0.026 | 0.001 | 1.86 | 1052631.6 | — |
| luajit-prepared / towers-recursive | 0.968 | 1.348 | 5.14 | 742.0 | 0 |
| luajit-prepared / queens-backtracking | 0.358 | 0.101 | 6.43 | 9896.1 | 1 |
| luajit-prepared / heap-permute | 0.338 | 0.205 | 5.09 | 4875.7 | 0 |
| luajit-prepared / ackermann-deep | 3.333 | 7.019 | 2.52 | 142.5 | 38 |
| luajit-prepared / tak-branching | 0.589 | 0.200 | 3.14 | 5003.8 | 0 |
| luajit-prepared / collatz-range | 3.459 | 3.025 | 3.56 | 330.6 | 13 |
| luajit-prepared / matrix-multiply | 0.078 | 0.011 | 1.79 | 87336.2 | 37 |
| hara-rust-full / sieve-array | 0.030 | 0.002 | 3.14 | 621118.0 | 0 |
| hara-rust-full / towers-recursive | 0.854 | 0.851 | 3.25 | 1174.5 | 0 |
| hara-rust-full / queens-backtracking | 0.481 | 0.391 | 24.89 | 2555.5 | 1 |
| hara-rust-full / heap-permute | 0.629 | 0.602 | 14.94 | 1660.1 | 4 |
| hara-rust-full / ackermann-deep | 9.205 | 9.314 | 3.34 | 107.4 | 0 |
| hara-rust-full / tak-branching | 0.148 | 0.129 | 2.03 | 7748.5 | 3 |
| hara-rust-full / collatz-range | 3.756 | 3.621 | 4.26 | 276.1 | 0 |
| hara-rust-full / matrix-multiply | 0.074 | 0.039 | 6.05 | 25845.5 | 8 |

## Feature coverage

| Runtime / workload | Status | Detail |
|---|---|---|
| sbcl-prepared / sieve-array | ok | checksum verified |
| sbcl-prepared / towers-recursive | ok | checksum verified |
| sbcl-prepared / queens-backtracking | ok | checksum verified |
| sbcl-prepared / heap-permute | ok | checksum verified |
| sbcl-prepared / ackermann-deep | ok | checksum verified |
| sbcl-prepared / tak-branching | ok | checksum verified |
| sbcl-prepared / collatz-range | ok | checksum verified |
| sbcl-prepared / matrix-multiply | ok | checksum verified |
| chez-prepared / sieve-array | ok | checksum verified |
| chez-prepared / towers-recursive | ok | checksum verified |
| chez-prepared / queens-backtracking | ok | checksum verified |
| chez-prepared / heap-permute | ok | checksum verified |
| chez-prepared / ackermann-deep | ok | checksum verified |
| chez-prepared / tak-branching | ok | checksum verified |
| chez-prepared / collatz-range | ok | checksum verified |
| chez-prepared / matrix-multiply | ok | checksum verified |
| guile-prepared / sieve-array | ok | checksum verified |
| guile-prepared / towers-recursive | ok | checksum verified |
| guile-prepared / queens-backtracking | ok | checksum verified |
| guile-prepared / heap-permute | ok | checksum verified |
| guile-prepared / ackermann-deep | ok | checksum verified |
| guile-prepared / tak-branching | ok | checksum verified |
| guile-prepared / collatz-range | ok | checksum verified |
| guile-prepared / matrix-multiply | ok | checksum verified |
| bb-prepared / sieve-array | ok | checksum verified |
| bb-prepared / towers-recursive | ok | checksum verified |
| bb-prepared / queens-backtracking | ok | checksum verified |
| bb-prepared / heap-permute | ok | checksum verified |
| bb-prepared / ackermann-deep | ok | checksum verified |
| bb-prepared / tak-branching | ok | checksum verified |
| bb-prepared / collatz-range | ok | checksum verified |
| bb-prepared / matrix-multiply | ok | checksum verified |
| python-prepared / sieve-array | ok | checksum verified |
| python-prepared / towers-recursive | ok | checksum verified |
| python-prepared / queens-backtracking | ok | checksum verified |
| python-prepared / heap-permute | ok | checksum verified |
| python-prepared / ackermann-deep | ok | checksum verified |
| python-prepared / tak-branching | ok | checksum verified |
| python-prepared / collatz-range | ok | checksum verified |
| python-prepared / matrix-multiply | ok | checksum verified |
| c-prepared / sieve-array | ok | checksum verified |
| c-prepared / towers-recursive | ok | checksum verified |
| c-prepared / queens-backtracking | ok | checksum verified |
| c-prepared / heap-permute | ok | checksum verified |
| c-prepared / ackermann-deep | ok | checksum verified |
| c-prepared / tak-branching | ok | checksum verified |
| c-prepared / collatz-range | ok | checksum verified |
| c-prepared / matrix-multiply | ok | checksum verified |
| java-prepared / sieve-array | ok | checksum verified |
| java-prepared / towers-recursive | ok | checksum verified |
| java-prepared / queens-backtracking | ok | checksum verified |
| java-prepared / heap-permute | ok | checksum verified |
| java-prepared / ackermann-deep | ok | checksum verified |
| java-prepared / tak-branching | ok | checksum verified |
| java-prepared / collatz-range | ok | checksum verified |
| java-prepared / matrix-multiply | ok | checksum verified |
| luajit-prepared / sieve-array | ok | checksum verified |
| luajit-prepared / towers-recursive | ok | checksum verified |
| luajit-prepared / queens-backtracking | ok | checksum verified |
| luajit-prepared / heap-permute | ok | checksum verified |
| luajit-prepared / ackermann-deep | ok | checksum verified |
| luajit-prepared / tak-branching | ok | checksum verified |
| luajit-prepared / collatz-range | ok | checksum verified |
| luajit-prepared / matrix-multiply | ok | checksum verified |
| hara-rust-full / sieve-array | ok | checksum verified |
| hara-rust-full / towers-recursive | ok | checksum verified |
| hara-rust-full / queens-backtracking | ok | checksum verified |
| hara-rust-full / heap-permute | ok | checksum verified |
| hara-rust-full / ackermann-deep | ok | checksum verified |
| hara-rust-full / tak-branching | ok | checksum verified |
| hara-rust-full / collatz-range | ok | checksum verified |
| hara-rust-full / matrix-multiply | ok | checksum verified |

## Head-to-head (steady state, lisp / hara tier)

| Workload | Lisp | Hara tier | Lisp steady ms | Hara steady ms | Ratio |
|---|---|---|---:|---:|---:|
| sieve-array | sbcl-prepared | hara-rust-full | 0.001 | 0.002 | 0.6832 |
| sieve-array | chez-prepared | hara-rust-full | 0.001 | 0.002 | 0.4969 |
| sieve-array | guile-prepared | hara-rust-full | 0.066 | 0.002 | 41.1491 |
| sieve-array | bb-prepared | hara-rust-full | 0.024 | 0.002 | 15.0876 |
| sieve-array | python-prepared | hara-rust-full | 0.004 | 0.002 | 2.2317 |
| sieve-array | c-prepared | hara-rust-full | 0.000 | 0.002 | 0.1019 |
| sieve-array | java-prepared | hara-rust-full | 0.002 | 0.002 | 0.9534 |
| sieve-array | luajit-prepared | hara-rust-full | 0.001 | 0.002 | 0.5901 |
| towers-recursive | sbcl-prepared | hara-rust-full | 2.523 | 0.851 | 2.9638 |
| towers-recursive | chez-prepared | hara-rust-full | 0.700 | 0.851 | 0.8219 |
| towers-recursive | guile-prepared | hara-rust-full | 60.885 | 0.851 | 71.5111 |
| towers-recursive | bb-prepared | hara-rust-full | 25.489 | 0.851 | 29.9381 |
| towers-recursive | python-prepared | hara-rust-full | 12.615 | 0.851 | 14.8165 |
| towers-recursive | c-prepared | hara-rust-full | 0.245 | 0.851 | 0.2876 |
| towers-recursive | java-prepared | hara-rust-full | 0.318 | 0.851 | 0.3738 |
| towers-recursive | luajit-prepared | hara-rust-full | 1.348 | 0.851 | 1.5829 |
| queens-backtracking | sbcl-prepared | hara-rust-full | 0.435 | 0.391 | 1.1116 |
| queens-backtracking | chez-prepared | hara-rust-full | 0.136 | 0.391 | 0.3468 |
| queens-backtracking | guile-prepared | hara-rust-full | 39.074 | 0.391 | 99.8528 |
| queens-backtracking | bb-prepared | hara-rust-full | 6.380 | 0.391 | 16.3050 |
| queens-backtracking | python-prepared | hara-rust-full | 4.879 | 0.391 | 12.4684 |
| queens-backtracking | c-prepared | hara-rust-full | 0.049 | 0.391 | 0.1240 |
| queens-backtracking | java-prepared | hara-rust-full | 0.033 | 0.391 | 0.0842 |
| queens-backtracking | luajit-prepared | hara-rust-full | 0.101 | 0.391 | 0.2582 |
| heap-permute | sbcl-prepared | hara-rust-full | 0.835 | 0.602 | 1.3858 |
| heap-permute | chez-prepared | hara-rust-full | 0.484 | 0.602 | 0.8036 |
| heap-permute | guile-prepared | hara-rust-full | 83.366 | 0.602 | 138.3953 |
| heap-permute | bb-prepared | hara-rust-full | 13.948 | 0.602 | 23.1558 |
| heap-permute | python-prepared | hara-rust-full | 6.466 | 0.602 | 10.7341 |
| heap-permute | c-prepared | hara-rust-full | 0.095 | 0.602 | 0.1582 |
| heap-permute | java-prepared | hara-rust-full | 0.098 | 0.602 | 0.1622 |
| heap-permute | luajit-prepared | hara-rust-full | 0.205 | 0.602 | 0.3405 |
| ackermann-deep | sbcl-prepared | hara-rust-full | 20.157 | 9.314 | 2.1642 |
| ackermann-deep | chez-prepared | hara-rust-full | 2.430 | 9.314 | 0.2609 |
| ackermann-deep | guile-prepared | hara-rust-full | 309.774 | 9.314 | 33.2590 |
| ackermann-deep | bb-prepared | hara-rust-full | 142.650 | 9.314 | 15.3157 |
| ackermann-deep | python-prepared | hara-rust-full | 91.527 | 9.314 | 9.8268 |
| ackermann-deep | c-prepared | hara-rust-full | 6.861 | 9.314 | 0.7366 |
| ackermann-deep | java-prepared | hara-rust-full | 4.230 | 9.314 | 0.4542 |
| ackermann-deep | luajit-prepared | hara-rust-full | 7.019 | 9.314 | 0.7536 |
| tak-branching | sbcl-prepared | hara-rust-full | 0.274 | 0.129 | 2.1242 |
| tak-branching | chez-prepared | hara-rust-full | 0.057 | 0.129 | 0.4413 |
| tak-branching | guile-prepared | hara-rust-full | 7.295 | 0.129 | 56.5242 |
| tak-branching | bb-prepared | hara-rust-full | 2.324 | 0.129 | 18.0035 |
| tak-branching | python-prepared | hara-rust-full | 1.429 | 0.129 | 11.0726 |
| tak-branching | c-prepared | hara-rust-full | 0.066 | 0.129 | 0.5138 |
| tak-branching | java-prepared | hara-rust-full | 0.039 | 0.129 | 0.2991 |
| tak-branching | luajit-prepared | hara-rust-full | 0.200 | 0.129 | 1.5485 |
| collatz-range | sbcl-prepared | hara-rust-full | 4.847 | 3.621 | 1.3384 |
| collatz-range | chez-prepared | hara-rust-full | 5.083 | 3.621 | 1.4035 |
| collatz-range | guile-prepared | hara-rust-full | 157.271 | 3.621 | 43.4280 |
| collatz-range | bb-prepared | hara-rust-full | 40.346 | 3.621 | 11.1409 |
| collatz-range | python-prepared | hara-rust-full | 34.232 | 3.621 | 9.4526 |
| collatz-range | c-prepared | hara-rust-full | 0.605 | 3.621 | 0.1671 |
| collatz-range | java-prepared | hara-rust-full | 0.894 | 3.621 | 0.2467 |
| collatz-range | luajit-prepared | hara-rust-full | 3.025 | 3.621 | 0.8353 |
| matrix-multiply | sbcl-prepared | hara-rust-full | 0.037 | 0.039 | 0.9576 |
| matrix-multiply | chez-prepared | hara-rust-full | 0.023 | 0.039 | 0.5841 |
| matrix-multiply | guile-prepared | hara-rust-full | 6.723 | 0.039 | 173.7600 |
| matrix-multiply | bb-prepared | hara-rust-full | 1.021 | 0.039 | 26.3972 |
| matrix-multiply | python-prepared | hara-rust-full | 0.507 | 0.039 | 13.1095 |
| matrix-multiply | c-prepared | hara-rust-full | 0.003 | 0.039 | 0.0696 |
| matrix-multiply | java-prepared | hara-rust-full | 0.001 | 0.039 | 0.0293 |
| matrix-multiply | luajit-prepared | hara-rust-full | 0.011 | 0.039 | 0.2959 |

Ratio < 1 means the comparison runtime is faster. Convergence is the first five-window run within ±5% of the final ten-window median with CV ≤10%.

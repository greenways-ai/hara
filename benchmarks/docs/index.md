<div class="hara-benchmark-page">
<h1>Hara runtime benchmarks</h1>
<p class="hara-benchmark-lede">VM and compiled tiers measured with checksum-verified workloads. Lower prepared-call time is better.</p>
<p><strong>Evidence profile:</strong> smoke. Standard-profile runs are required before publishing reference thresholds.</p>
<div class="hara-benchmark-hosts">
<section class="hara-benchmark-host"><h2>Rust</h2>
<article class="hara-benchmark-card "><h3>hara-rust-vm</h3><strong>1.360 ms</strong><dl><dt>Host</dt><dd>Native</dd><dt>Engine</dt><dd>Rust bytecode VM</dd><dt>Tier</dt><dd>bytecode</dd><dt>Artifact</dt><dd><code>hara eval --engine vm</code></dd><dt>Size</dt><dd>Recorded by a standard publication run</dd></dl><table><thead><tr><th>Workload</th><th>Prepared</th><th>First</th></tr></thead><tbody><tr><td>arithmetic-branch-mix</td><td>1.3596 ms</td><td>1.3528 ms</td></tr><tr><td>mutable-array-update</td><td>0.6570 ms</td><td>0.6430 ms</td></tr><tr><td>mutable-object-counters</td><td>0.5965 ms</td><td>0.6118 ms</td></tr><tr><td>persistent-nested-transform</td><td>2.2788 ms</td><td>2.7002 ms</td></tr><tr><td>recursive-tree-eval</td><td>2.2694 ms</td><td>2.2929 ms</td></tr></tbody></table></article>
<article class="hara-benchmark-card "><h3>hara-rust-full</h3><strong>0.019 ms</strong><dl><dt>Host</dt><dd>Native</dd><dt>Engine</dt><dd>Rust whole-function compiler</dd><dt>Tier</dt><dd>compiled Wasm</dd><dt>Artifact</dt><dd><code>hara eval --engine whole-wasm</code></dd><dt>Size</dt><dd>Recorded by a standard publication run</dd></dl><table><thead><tr><th>Workload</th><th>Prepared</th><th>First</th></tr></thead><tbody><tr><td>arithmetic-branch-mix</td><td>0.0207 ms</td><td>0.0450 ms</td></tr><tr><td>mutable-array-update</td><td>0.0060 ms</td><td>0.0328 ms</td></tr><tr><td>mutable-object-counters</td><td>0.0078 ms</td><td>0.0341 ms</td></tr><tr><td>persistent-nested-transform</td><td>0.0512 ms</td><td>0.0974 ms</td></tr><tr><td>recursive-tree-eval</td><td>0.0195 ms</td><td>0.0417 ms</td></tr></tbody></table></article>
</section>
<section class="hara-benchmark-host"><h2>WebAssembly</h2>
<article class="hara-benchmark-card "><h3>hara-wasm-vm</h3><strong>3.362 ms</strong><dl><dt>Host</dt><dd>Browser</dd><dt>Engine</dt><dd>Rust bytecode VM in Wasm</dd><dt>Tier</dt><dd>bytecode</dd><dt>Artifact</dt><dd><code>@hara-lang/browser/vm</code></dd><dt>Size</dt><dd>3.04 MB ESM · 876 KB gzip</dd></dl><table><thead><tr><th>Workload</th><th>Prepared</th><th>First</th></tr></thead><tbody><tr><td>arithmetic-branch-mix</td><td>3.3625 ms</td><td>—</td></tr><tr><td>mutable-array-update</td><td>1.8923 ms</td><td>—</td></tr><tr><td>mutable-object-counters</td><td>1.7857 ms</td><td>—</td></tr><tr><td>persistent-nested-transform</td><td>3.9571 ms</td><td>—</td></tr><tr><td>recursive-tree-eval</td><td>3.6143 ms</td><td>—</td></tr></tbody></table></article>
<article class="hara-benchmark-card "><h3>hara-wasm-full</h3><strong>0.028 ms</strong><dl><dt>Host</dt><dd>Browser</dd><dt>Engine</dt><dd>Whole-function Wasm compiler</dd><dt>Tier</dt><dd>compiled Wasm</dd><dt>Artifact</dt><dd><code>@hara-lang/browser/full</code></dd><dt>Size</dt><dd>3.30 MB ESM · 954 KB gzip</dd></dl><table><thead><tr><th>Workload</th><th>Prepared</th><th>First</th></tr></thead><tbody><tr><td>arithmetic-branch-mix</td><td>0.0222 ms</td><td>—</td></tr><tr><td>mutable-array-update</td><td>0.0054 ms</td><td>—</td></tr><tr><td>mutable-object-counters</td><td>0.0335 ms</td><td>—</td></tr><tr><td>persistent-nested-transform</td><td>0.2940 ms</td><td>—</td></tr><tr><td>recursive-tree-eval</td><td>0.0283 ms</td><td>—</td></tr></tbody></table></article>
</section>
<section class="hara-benchmark-host"><h2>Truffle</h2>
<article class="hara-benchmark-card "><h3>hara-truffle-jvm</h3><strong>2.881 ms</strong><dl><dt>Host</dt><dd>JVM</dd><dt>Engine</dt><dd>Graal/Truffle interpreter</dd><dt>Tier</dt><dd>optimizing VM</dd><dt>Artifact</dt><dd><code>java … hara.truffle.Main</code></dd><dt>Size</dt><dd>Recorded by a standard publication run</dd></dl><table><thead><tr><th>Workload</th><th>Prepared</th><th>First</th></tr></thead><tbody><tr><td>arithmetic</td><td>10.9614 ms</td><td>99.3926 ms</td></tr><tr><td>branchy-loop</td><td>10.9389 ms</td><td>94.1287 ms</td></tr><tr><td>function-call</td><td>5.0374 ms</td><td>90.5256 ms</td></tr><tr><td>noop</td><td>0.0206 ms</td><td>79.2935 ms</td></tr><tr><td>persistent-map</td><td>1.7895 ms</td><td>84.3275 ms</td></tr><tr><td>persistent-vector</td><td>2.2175 ms</td><td>83.7090 ms</td></tr><tr><td>sequence-navigation</td><td>2.8805 ms</td><td>88.8175 ms</td></tr></tbody></table></article>
<article class="hara-benchmark-card is-unavailable"><h3>hara-truffle-native-vm</h3><strong>NOT MEASURED</strong><dl><dt>Host</dt><dd>Native Image</dd><dt>Engine</dt><dd>Truffle native fallback</dd><dt>Tier</dt><dd>VM</dd><dt>Artifact</dt><dd><code>target/hara-truffle-native-vm</code></dd><dt>Size</dt><dd>Recorded by a standard publication run</dd></dl><p>This target is fully specified but was not built on the evidence host. A standard publication runner must supply its measurement artifact.</p></article>
<article class="hara-benchmark-card is-unavailable"><h3>hara-truffle-native-full</h3><strong>NOT MEASURED</strong><dl><dt>Host</dt><dd>Native Image</dd><dt>Engine</dt><dd>Truffle native compiled tier</dd><dt>Tier</dt><dd>compiled</dd><dt>Artifact</dt><dd><code>target/hara-truffle-native-full</code></dd><dt>Size</dt><dd>Recorded by a standard publication run</dd></dl><p>This target is fully specified but was not built on the evidence host. A standard publication runner must supply its measurement artifact.</p></article>
</section>
</div>
<h2>HTTP frameworks</h2>
<p>Single worker/event loop, loopback HTTP/1.1, keep-alive, concurrency 8.</p>
<table><thead><tr><th>Route</th><th>Server</th><th>Requests/sec</th><th>p50 ms</th><th>p95 ms</th><th>p99 ms</th></tr></thead><tbody>
<tr><td>/hello</td><td>hoplite</td><td>48461.3</td><td>0.000</td><td>0.000</td><td>0.000</td></tr>
<tr><td>/json</td><td>hoplite</td><td>48285.8</td><td>0.000</td><td>0.000</td><td>0.000</td></tr>
<tr><td>/delay</td><td>hoplite</td><td>281.2</td><td>27.000</td><td>28.000</td><td>28.000</td></tr>
<tr><td>/hello</td><td>openresty</td><td>49975.0</td><td>0.000</td><td>0.000</td><td>0.000</td></tr>
<tr><td>/json</td><td>openresty</td><td>56513.1</td><td>0.000</td><td>0.000</td><td>0.000</td></tr>
<tr><td>/delay</td><td>openresty</td><td>285.7</td><td>27.000</td><td>27.000</td><td>28.000</td></tr>
<tr><td>/hello</td><td>nginx</td><td>58530.9</td><td>0.000</td><td>0.000</td><td>0.000</td></tr>
<tr><td>/json</td><td>nginx</td><td>63816.2</td><td>0.000</td><td>0.000</td><td>0.000</td></tr>
<tr><td>/hello</td><td>fastify</td><td>31220.7</td><td>0.000</td><td>0.000</td><td>1.000</td></tr>
<tr><td>/json</td><td>fastify</td><td>31520.9</td><td>0.000</td><td>0.000</td><td>1.000</td></tr>
<tr><td>/delay</td><td>fastify</td><td>274.2</td><td>28.000</td><td>29.000</td><td>30.000</td></tr>
<tr><td>/hello</td><td>axum</td><td>52151.2</td><td>0.000</td><td>0.000</td><td>0.000</td></tr>
<tr><td>/json</td><td>axum</td><td>45903.1</td><td>0.000</td><td>0.000</td><td>0.000</td></tr>
<tr><td>/delay</td><td>axum</td><td>276.9</td><td>28.000</td><td>29.000</td><td>29.000</td></tr>
<tr><td>/delay</td><td>nginx</td><td>not applicable</td><td>—</td><td>—</td><td>—</td></tr>
</tbody></table>
<h2>Methodology</h2>
<p>Every adapter checks the same expected result. Compilation, first execution, and prepared execution are separate boundaries. Browser windows use adaptive batching; HTTP values are medians across trials. Every target remains fully specified even when a particular evidence host cannot build it.</p>
</div>

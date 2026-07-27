# chrome-hara Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Build a Chrome MV3 extension (`chrome-hara/`) that embeds the hara raw HTA wasm runtime in a DevTools panel REPL, exposes the Chrome API to hara as `(require [chrome.api :as api])`, supports a user-picked home directory for loading `.hal` scripts, and (stretch) exposes a RESP eval endpoint via a local bridge.

**Architecture:** The raw HTA runtime (`wasm/raw/` + `wasm/web/hta.js` + `hta-worker.js`) runs in a Web Worker inside the extension's DevTools panel page. Hara `(host/call ...)` round-trips to a JS `hostCalls` proxy in the panel, which forwards over a `chrome.runtime` Port to the service worker, where the actual `chrome.*` calls (incl. `chrome.debugger`/CDP) execute. `(require [ns :as alias])` support is added to core eval with a thread-local namespace-source provider; JS registers `.hal` sources by name via a new `register-resource` HTA target. The RESP surface is a small node bridge (RESP TCP ⟷ WebSocket) that the panel dials outbound.

**Tech Stack:** Rust (wasm32-unknown-unknown, no new deps), vanilla JS ESM (no bundler), Chrome MV3, `@playwright/test` for browser tests, `node --test` for pure-JS unit tests, `ws` (bridge only).

**Approved design:** see session plan (brainstorming output). Key constraint from exploration: the wasm-bindgen `Runtime` (`wasm/src/lib.rs`) does NOT wire `host/call` and is NOT used here; only the raw HTA build is.

## Global Constraints

- No new Rust crate dependencies. No bundler (vite/webpack) in `chrome-hara/` — plain ESM + copied vendor files.
- Raw runtime build: `bash scripts/build-hara-wasm-raw` → `wasm/raw/target/wasm32-unknown-unknown/release/hara_wasm_raw.wasm`.
- Existing tests must stay green: `cargo test --manifest-path wasm/Cargo.toml`, `cargo test --manifest-path wasm/raw/Cargo.toml`, `cd wasm/web && npm run test:hta`.
- Browser tests run under `xvfb-run -a` (Playwright persistent context with extensions requires a headed browser on this version).
- Hara syntax available in the raw runtime is CORE ONLY (no `std.lib.foundation` bootstrap): `defn` (multi-arity + variadic), `fn`, `let`, `if`, `do`, `loop/recur`, `get`, `assoc`, `str`, `deref`, `host/call`, iter-*, etc. Do not use foundation fns (`first`, `rest`, `map`, `get-in`…) in `.hal` files shipped here.
- Naming: top-level `chrome-hara/` mirrors `vscode-hara/`, `emacs-hara/`.
- Git commits are included per task; confirm with the user before the first commit.

---

### Task 1: Rust — `require` with namespace-source provider in core eval + raw runtime resources

The raw runtime's `ns` special form (`wasm/src/core.rs:4912-4924`) currently accepts only `(ns name)` and silently ignores `:require` clauses; there is no `require` form at all. Add: (a) a thread-local namespace-source provider mirroring `HOST_CALL_HANDLER`, (b) `require` handling in core eval (`ns` clauses + standalone `require` form), (c) a `resources` map + `register-resource` target in the raw HTA ABI.

**Files:**
- Modify: `wasm/src/core.rs` (thread-local near line 1086; `with_namespace_source` near line 1417; ns/require eval near line 4912; helpers near `eval_namespace_operation` line 4352)
- Modify: `wasm/raw/src/lib.rs` (Runtime struct line 49; `start_fiber` line 124 / `resume_fiber` line 134; `hta_start` dispatch line 250; tests module line 437)
- Test: `wasm/raw/src/lib.rs` tests module (same file)

**Interfaces:**
- Consumes: existing `NamespaceRegistry` API — `find(&str) -> Option<Namespace<V>>`, `current() -> Namespace<V>`, `Namespace::alias(impl AsRef<str>, Namespace<V>)` (`wasm/src/kernel/namespace.rs:109,178,191`); `select_namespace_environment(registry, env, name)` and `refresh_namespace_environment(registry, env)` (`wasm/src/core.rs:1173,1194`); `kernel::parse_forms(&str) -> Result<Vec<Form>, String>`.
- Produces:
  - `pub fn with_namespace_source<R>(provider: Rc<dyn Fn(&str) -> Option<String>>, action: impl FnOnce() -> R) -> R` in `wasm/src/core.rs`
  - Hara forms: `(ns name (:require [a.b :as b]))` and `(require [a.b :as b] [c.d])` working in the raw runtime.
  - New HTA target `"register-resource"` with args `[name: String, source: String]`, completing with `Value::Bool(true)`.

- [ ] **Step 1: Write the failing tests**

Add to the `#[cfg(test)] mod tests` in `wasm/raw/src/lib.rs` (adjust imports at the top of the module: `use super::{eval_error_code, evaluate, Runtime};` already exists; add `use crate::hta;` if not reachable via `super::hta` — use `super::hta`):

```rust
    fn completion_value(runtime: &mut Runtime, task: u64) -> crate::core::Value {
        let frame = runtime
            .events
            .borrow_mut()
            .pop_front()
            .expect("completion event");
        match super::hta::decode(&frame).unwrap() {
            crate::core::Value::Vector(values) => {
                assert_eq!(values[0], crate::core::Value::Number(0), "eval failed");
                assert_eq!(values[1], crate::core::Value::Number(task as i64));
                values[2].clone()
            }
            other => panic!("unexpected event: {other:?}"),
        }
    }

    #[test]
    fn require_loads_registered_resource_and_binds_alias() {
        let mut runtime = Runtime::new();
        runtime.resources.borrow_mut().insert(
            "chrome.api".to_string(),
            "(ns chrome.api) (defn answer [] 42)".to_string(),
        );
        runtime
            .start_fiber(1, "(require [chrome.api :as api]) (api/answer)")
            .unwrap();
        assert_eq!(completion_value(&mut runtime, 1), crate::core::Value::Number(42));
    }

    #[test]
    fn require_supports_ns_form_clauses_and_qualified_access() {
        let mut runtime = Runtime::new();
        runtime.resources.borrow_mut().insert(
            "acme.tools".to_string(),
            "(ns acme.tools) (defn seven [] 7)".to_string(),
        );
        runtime
            .start_fiber(2, "(ns demo (:require [acme.tools :as tools])) (tools/seven)")
            .unwrap();
        assert_eq!(completion_value(&mut runtime, 2), crate::core::Value::Number(7));
        runtime
            .start_fiber(3, "(acme.tools/seven)")
            .unwrap();
        assert_eq!(completion_value(&mut runtime, 3), crate::core::Value::Number(7));
    }

    #[test]
    fn require_missing_namespace_is_a_clean_error() {
        let mut runtime = Runtime::new();
        runtime.start_fiber(4, "(require [no.such.ns])").unwrap();
        let frame = runtime
            .events
            .borrow_mut()
            .pop_front()
            .expect("error event");
        match super::hta::decode(&frame).unwrap() {
            crate::core::Value::Vector(values) => {
                assert_eq!(values[0], crate::core::Value::Number(1), "expected failure");
            }
            other => panic!("unexpected event: {other:?}"),
        }
    }
```

- [ ] **Step 2: Run tests to verify they fail**

Run: `cargo test --manifest-path wasm/raw/Cargo.toml`
Expected: FAIL to compile (`runtime.resources` field does not exist), or if made to compile, `require` forms fail with "unbound symbol"/ns errors.

- [ ] **Step 3: Implement core eval support in `wasm/src/core.rs`**

Near `HOST_CALL_HANDLER` (line 1086), add:

```rust
    static NAMESPACE_SOURCE_PROVIDER: RefCell<Option<Rc<dyn Fn(&str) -> Option<String>>>> = const { RefCell::new(None) };
```

Near `with_host_calls` (line 1417), add:

```rust
/// Runs an evaluation with a source provider used to satisfy `require` loads.
pub fn with_namespace_source<R>(
    provider: Rc<dyn Fn(&str) -> Option<String>>,
    action: impl FnOnce() -> R,
) -> R {
    NAMESPACE_SOURCE_PROVIDER.with(|active| {
        let previous = active.borrow_mut().replace(provider);
        let result = action();
        *active.borrow_mut() = previous;
        result
    })
}
```

Near `eval_namespace_operation` (line 4352), add the require machinery:

```rust
fn ensure_namespace(
    registry: &NamespaceRegistry<Value>,
    env: &mut HashMap<String, Value>,
    name: &str,
) -> Result<(), String> {
    if registry.find(name).is_some() {
        return Ok(());
    }
    let source = NAMESPACE_SOURCE_PROVIDER
        .with(|active| active.borrow().as_ref().and_then(|provider| provider(name)))
        .ok_or_else(|| format!("Cannot require missing namespace: {name}"))?;
    let requiring = registry.current().name().as_str().to_owned();
    for form in crate::kernel::parse_forms(&source)? {
        eval(&form, env)?;
    }
    select_namespace_environment(registry, env, &requiring);
    Ok(())
}

fn eval_require_spec(
    registry: &NamespaceRegistry<Value>,
    env: &mut HashMap<String, Value>,
    form: &Form,
) -> Result<(), String> {
    let spec = match form {
        Form::Vector(items) => items,
        _ => return Err("require expects vectors such as [chrome.api :as api]".into()),
    };
    let target = match spec.first() {
        Some(Form::Symbol(target)) => target.clone(),
        _ => return Err("require namespace must be a symbol".into()),
    };
    ensure_namespace(registry, env, &target)?;
    if (spec.len() - 1) % 2 != 0 {
        return Err(format!("Malformed require options for {target}"));
    }
    for option in spec[1..].chunks(2) {
        let name = match &option[0] {
            Form::Keyword(keyword) => keyword.as_str(),
            _ => return Err("Malformed require options".into()),
        };
        match name {
            "as" => {
                let alias = match &option[1] {
                    Form::Symbol(alias) if !alias.contains('/') => alias.clone(),
                    _ => return Err("require :as expects an unqualified symbol".into()),
                };
                let namespace = registry
                    .find(&target)
                    .ok_or_else(|| format!("Cannot require missing namespace: {target}"))?;
                registry.current().alias(alias, namespace);
            }
            other => return Err(format!("Unsupported require option: :{other}")),
        }
    }
    Ok(())
}

fn eval_require_specs(
    registry: &NamespaceRegistry<Value>,
    env: &mut HashMap<String, Value>,
    specs: &[Form],
) -> Result<(), String> {
    for spec in specs {
        eval_require_spec(registry, env, spec)?;
    }
    refresh_namespace_environment(registry, env);
    Ok(())
}
```

Replace the existing `ns` branch (lines 4912-4924) and add a `require` branch directly after it:

```rust
            Form::Symbol(n) if n == "ns" => {
                if fs.len() < 2 {
                    return Err("ns expects a namespace symbol".into());
                }
                let name = match &fs[1] {
                    Form::Symbol(name) if !name.contains('/') => name.clone(),
                    _ => return Err("ns expects a namespace symbol".into()),
                };
                let registry = namespace_registry()?;
                select_namespace_environment(&registry, env, &name);
                for clause in &fs[2..] {
                    match clause {
                        Form::List(clause_forms)
                            if matches!(clause_forms.first(), Some(Form::Keyword(k)) if k == "require") =>
                        {
                            eval_require_specs(&registry, env, &clause_forms[1..])?;
                        }
                        _ => return Err("unsupported ns clause (only :require is supported)".into()),
                    }
                }
                Ok(Value::Nil)
            }
            Form::Symbol(n) if n == "require" => {
                let registry = namespace_registry()?;
                eval_require_specs(&registry, env, &fs[1..])?;
                Ok(Value::Nil)
            }
```

Notes for the implementer:
- `Form`, `NamespaceRegistry`, `Value`, `select_namespace_environment`, `refresh_namespace_environment`, `namespace_registry`, `eval` are all already in scope in `core.rs`; `HashMap` is imported at the top of the file (verify; add `use std::collections::HashMap;` only if missing).
- `crate::kernel::parse_forms` resolves in both crates: the main `wasm` crate (`mod kernel` in `wasm/src/lib.rs`) and the raw crate (`#[path] mod kernel` in `wasm/raw/src/lib.rs`). Verify `parse_forms` is `pub` — it is used as `kernel::parse_forms` from `wasm/src/lib.rs:141` and `wasm/raw/src/lib.rs:402`.
- The wasm-bindgen runtime (`wasm/src/lib.rs` `eval_text_mode`) intercepts `ns` forms before `core::eval` and never reaches this branch, so its behavior is unchanged.

- [ ] **Step 4: Implement raw runtime resources in `wasm/raw/src/lib.rs`**

In `struct Runtime` (line 49), add a field and initialize it in `Runtime::new` (line 61):

```rust
    resources: Rc<RefCell<HashMap<String, String>>>,
```
```rust
            resources: Rc::new(RefCell::new(HashMap::new())),
```

In `start_fiber` (line 124), wrap fiber start with the provider (same for `resume_fiber`, line 134):

```rust
    fn start_fiber(&mut self, task: u64, source: &str) -> Result<(), String> {
        let (handler, pending, next) = self.host_handler(task);
        let namespaces = self.namespaces.clone();
        let resources = self.resources.clone();
        let provider = Rc::new(move |name: &str| resources.borrow().get(name).cloned());
        let fiber = core::with_namespace_registry(&namespaces, || {
            core::with_namespace_source(provider, || {
                core::with_host_calls(handler, || EvalFiber::start(source, self.env.clone()))
            })
        })?;
        self.collect_calls(task, pending, next);
        self.drive(task, fiber);
        Ok(())
    }
```

In `resume_fiber`, wrap `fiber.resume(state)` the same way (provider + host calls inside `with_namespace_registry`).

In `hta_start`'s dispatch (line 250), add a new arm BEFORE the `hta/target-unknown` fallthrough:

```rust
            Ok((target, args)) if target == "register-resource" => match args.as_slice() {
                [Value::String(name), Value::String(source)] => {
                    runtime
                        .resources
                        .borrow_mut()
                        .insert(name.clone(), source.clone());
                    runtime.event(event(0, task, Value::Bool(true)));
                    Ok(())
                }
                _ => Err("hta register-resource expects name and source strings".into()),
            },
```

- [ ] **Step 5: Run tests to verify they pass**

Run: `cargo test --manifest-path wasm/raw/Cargo.toml`
Expected: PASS, including the 3 new tests.

Run: `cargo test --manifest-path wasm/Cargo.toml`
Expected: PASS (no regressions in the bindgen/native crate).

Rebuild the browser artifact for later tasks:
Run: `bash scripts/build-hara-wasm-raw`
Expected: `Built .../wasm/raw/target/wasm32-unknown-unknown/release/hara_wasm_raw.wasm`

- [ ] **Step 6: Commit**

```bash
git add wasm/src/core.rs wasm/raw/src/lib.rs
git commit -m "feat(wasm): require with namespace-source provider in raw runtime"
```

---

### Task 2: Extension skeleton — manifest, DevTools panel REPL, vendor sync, Playwright round-trip

**Files:**
- Create: `chrome-hara/manifest.json`
- Create: `chrome-hara/src/devtools.html`, `chrome-hara/src/devtools.js`
- Create: `chrome-hara/src/panel.html`, `chrome-hara/src/panel.css`, `chrome-hara/src/panel.js`
- Create: `chrome-hara/src/background.js` (port plumbing + echo only; real chrome calls in Task 3)
- Create: `chrome-hara/src/host-bridge.js`
- Create: `chrome-hara/scripts/sync-runtime.mjs`
- Create: `chrome-hara/package.json`, `chrome-hara/playwright.config.js`
- Create: `chrome-hara/test/extension.js`, `chrome-hara/test/host-bridge.test.js`, `chrome-hara/test/repl.spec.js`
- Create: `chrome-hara/README.md`

**Interfaces:**
- Consumes: `HtaContext` from `wasm/web/hta.js` (`new HtaContext({worker, moduleBytes, hostCalls})`, `context.call(target, args) -> Promise<value>`); raw wasm at `wasm/raw/target/wasm32-unknown-unknown/release/hara_wasm_raw.wasm`; HTA targets `"eval"` and (from Task 1) `"register-resource"`.
- Produces:
  - `window.hara = { context, evalSource(source) -> Promise<value>, tabId }` on the panel page (test hook).
  - `createHostCalls(port) -> Proxy` in `src/host-bridge.js`; `toPlain(value)`, `fromPlain(value)` exported for unit tests and Task 6's renderer.
  - Port protocol (panel → service worker): request `{id, service, method, args}`; response `{id, ok, value}` or `{id, ok:false, error}`.
  - Vendor layout: `chrome-hara/vendor/hta.js`, `chrome-hara/vendor/hta-worker.js`, `chrome-hara/vendor/hara.wasm`.

- [ ] **Step 1: Scaffold + sync script**

`chrome-hara/package.json`:

```json
{
  "name": "chrome-hara",
  "private": true,
  "type": "module",
  "scripts": {
    "build": "bash ../scripts/build-hara-wasm-raw && node scripts/sync-runtime.mjs",
    "sync": "node scripts/sync-runtime.mjs",
    "test": "node --test test/*.test.js",
    "test:browser": "xvfb-run -a npx playwright test"
  },
  "devDependencies": {
    "@playwright/test": "^1.54.1"
  },
  "dependencies": {
    "ws": "^8.18.0"
  }
}
```

`chrome-hara/scripts/sync-runtime.mjs`:

```js
import { copyFileSync, existsSync, mkdirSync } from "node:fs";
import path from "node:path";
import { fileURLToPath } from "node:url";

const root = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
const repo = path.resolve(root, "..");
const vendor = path.join(root, "vendor");
mkdirSync(vendor, { recursive: true });

const files = [
  [path.join(repo, "wasm/web/hta.js"), path.join(vendor, "hta.js")],
  [path.join(repo, "wasm/web/hta-worker.js"), path.join(vendor, "hta-worker.js")],
  [
    path.join(repo, "wasm/raw/target/wasm32-unknown-unknown/release/hara_wasm_raw.wasm"),
    path.join(vendor, "hara.wasm"),
  ],
];
for (const [from, to] of files) {
  if (!existsSync(from)) {
    console.error(`missing ${from} — run: bash scripts/build-hara-wasm-raw`);
    process.exit(1);
  }
  copyFileSync(from, to);
  console.log(`synced ${path.basename(to)}`);
}
```

Note: `hta-worker.js` imports `./hta.js` — both land flat in `vendor/`, so the import resolves unchanged.

Run: `cd chrome-hara && npm install && npm run build`
Expected: `Built .../hara_wasm_raw.wasm`, then three `synced` lines.

- [ ] **Step 2: Write the failing unit test for the host bridge**

`chrome-hara/test/host-bridge.test.js`:

```js
import { test } from "node:test";
import assert from "node:assert/strict";
import { toPlain, fromPlain, createHostCalls } from "../src/host-bridge.js";
import { HtaKeyword, HtaSymbol } from "../vendor/hta.js";

test("toPlain converts HTA values to JSON-safe values", () => {
  const input = new Map([
    [new HtaKeyword("url"), "https://example.com"],
    [new HtaKeyword("nested"), new Map([[new HtaKeyword("n"), 42]])],
    [new HtaKeyword("list"), [1, new HtaKeyword("two")]],
    [new HtaKeyword("sym"), new HtaSymbol("foo")],
  ]);
  assert.deepEqual(toPlain(input), {
    url: "https://example.com",
    nested: { n: 42 },
    list: [1, ":two"],
    sym: "foo",
  });
});

test("fromPlain converts JSON objects to keyword-keyed maps", () => {
  const out = fromPlain({ result: { value: 42 }, ok: true, nothing: null });
  assert.ok(out instanceof Map);
  const result = [...out].find(([k]) => k.name === "result")[1];
  assert.equal([...result][0][0].name, "value");
  assert.equal([...result][0][1], 42);
});

test("createHostCalls routes service/method over the port and decodes replies", async () => {
  const listeners = [];
  const sent = [];
  const port = {
    onMessage: { addListener: (fn) => listeners.push(fn) },
    postMessage: (msg) => {
      sent.push(msg);
      queueMicrotask(() =>
        listeners.forEach((fn) => fn({ id: msg.id, ok: true, value: { echoed: msg.args } })),
      );
    },
  };
  const hostCalls = createHostCalls(port);
  const value = await hostCalls["chrome.debugger/sendCommand"](1, "Page.navigate", { url: "x" });
  assert.equal(sent[0].service, "chrome.debugger");
  assert.equal(sent[0].method, "sendCommand");
  assert.ok(value instanceof Map);
  assert.equal([...value][0][0].name, "echoed");
});
```

Run: `cd chrome-hara && node --test test/host-bridge.test.js`
Expected: FAIL — `../src/host-bridge.js` does not exist.

- [ ] **Step 3: Implement `src/host-bridge.js`**

```js
import { HtaKeyword, HtaSymbol, HtaHandle } from "../vendor/hta.js";

/** HTA value -> JSON-safe value for chrome.runtime Port messaging. */
export function toPlain(value) {
  if (value === null || value === undefined) return null;
  if (value instanceof HtaKeyword) return `:${value.name}`;
  if (value instanceof HtaSymbol) return value.name;
  if (value instanceof HtaHandle) return String(value);
  if (value instanceof Map) {
    const out = {};
    for (const [key, item] of value) {
      const plainKey = key instanceof HtaKeyword ? key.name : String(toPlain(key));
      out[plainKey] = toPlain(item);
    }
    return out;
  }
  if (value instanceof Set) return [...value].map(toPlain);
  if (Array.isArray(value)) return value.map(toPlain);
  if (value instanceof Uint8Array) return [...value];
  if (typeof value === "bigint") return Number(value);
  return value;
}

/** JSON value -> HTA-compatible value (objects become keyword-keyed Maps). */
export function fromPlain(value) {
  if (Array.isArray(value)) return value.map(fromPlain);
  if (value !== null && typeof value === "object") {
    return new Map(
      Object.entries(value).map(([key, item]) => [new HtaKeyword(key), fromPlain(item)]),
    );
  }
  return value;
}

/**
 * Dynamic hostCalls map: any "service/method" key becomes a function that
 * forwards the call over the extension Port and resolves with the reply.
 */
export function createHostCalls(port) {
  const pending = new Map();
  let next = 1;
  port.onMessage.addListener(({ id, ok, value, error }) => {
    const entry = pending.get(id);
    if (!entry) return;
    pending.delete(id);
    if (ok) entry.resolve(fromPlain(value));
    else entry.reject(new Error(error ?? "host call failed"));
  });
  return new Proxy({}, {
    get: (_target, key) => {
      const text = String(key);
      const split = text.lastIndexOf("/");
      const service = text.slice(0, split);
      const method = text.slice(split + 1);
      return (...args) =>
        new Promise((resolve, reject) => {
          const id = next++;
          pending.set(id, { resolve, reject });
          port.postMessage({ id, service, method, args: args.map(toPlain) });
        });
    },
  });
}
```

Run: `cd chrome-hara && node --test test/host-bridge.test.js`
Expected: PASS (3 tests).

- [ ] **Step 4: Write the extension files**

`chrome-hara/manifest.json`:

```json
{
  "manifest_version": 3,
  "name": "hara",
  "version": "0.1.0",
  "description": "Hara runtime in DevTools with full Chrome API automation",
  "devtools_page": "src/devtools.html",
  "background": { "service_worker": "src/background.js", "type": "module" },
  "permissions": ["debugger", "tabs", "storage"],
  "content_security_policy": {
    "extension_pages": "script-src 'self' 'wasm-unsafe-eval'; object-src 'self'"
  }
}
```

`chrome-hara/src/devtools.html`:

```html
<!doctype html><meta charset="utf-8"><script src="./devtools.js"></script>
```

`chrome-hara/src/devtools.js`:

```js
chrome.devtools.panels.create("hara", "", "src/panel.html");
```

`chrome-hara/src/panel.html`:

```html
<!doctype html>
<meta charset="utf-8">
<title>hara</title>
<link rel="stylesheet" href="./panel.css">
<body>
  <div id="toolbar">
    <button id="home-button" type="button">choose home</button>
    <span id="home-label">no home</span>
    <button id="run-file-button" type="button">run .hal file</button>
  </div>
  <div id="output"></div>
  <textarea id="input" rows="3" placeholder="(+ 1 2) — ctrl+enter to eval"></textarea>
  <script type="module" src="./panel.js"></script>
</body>
```

`chrome-hara/src/panel.css`:

```css
body { margin: 0; display: flex; flex-direction: column; height: 100vh; font: 12px monospace; }
#toolbar { display: flex; gap: 8px; align-items: center; padding: 4px; border-bottom: 1px solid #444; }
#output { flex: 1; overflow: auto; padding: 4px; white-space: pre-wrap; }
#input { border: none; border-top: 1px solid #444; padding: 4px; font: inherit; resize: vertical; }
```

`chrome-hara/src/background.js` (echo only for now; Task 3 replaces `dispatch`):

```js
chrome.runtime.onConnect.addListener((port) => {
  if (port.name !== "hara-host") return;
  port.onMessage.addListener(async ({ id, service, method, args }) => {
    try {
      const value = await dispatch(service, method, args ?? []);
      port.postMessage({ id, ok: true, value });
    } catch (error) {
      port.postMessage({ id, ok: false, error: String(error?.message ?? error) });
    }
  });
});

async function dispatch(service, method, args) {
  if (service === "hara" && method === "echo") return args[0] ?? null;
  throw new Error(`host-call-denied: ${service}/${method}`);
}
```

`chrome-hara/src/panel.js`:

```js
import { HtaContext } from "../vendor/hta.js";
import { createHostCalls } from "./host-bridge.js";

const params = new URLSearchParams(location.search);
const tabId = params.has("tabId")
  ? Number(params.get("tabId"))
  : globalThis.chrome?.devtools?.inspectedWindow?.tabId;

const worker = new Worker(chrome.runtime.getURL("vendor/hta-worker.js"), { type: "module" });
const moduleBytes = new Uint8Array(
  await (await fetch(chrome.runtime.getURL("vendor/hara.wasm"))).arrayBuffer(),
);
const port = chrome.runtime.connect({ name: "hara-host" });
const context = new HtaContext({ worker, moduleBytes, hostCalls: createHostCalls(port) });

function evalSource(source) {
  return context.call("eval", [source]);
}

window.hara = { context, evalSource, tabId };

const input = document.getElementById("input");
const output = document.getElementById("output");
function print(text) {
  output.textContent += `${text}\n`;
  output.scrollTop = output.scrollHeight;
}
input.addEventListener("keydown", async (event) => {
  if (event.key !== "Enter" || !event.ctrlKey) return;
  event.preventDefault();
  const source = input.value;
  input.value = "";
  print(`hara=> ${source}`);
  try {
    print(String(await evalSource(source)));
  } catch (error) {
    print(`error: ${error?.message ?? error}`);
  }
});
```

Run: `cd chrome-hara && npm run sync`
Expected: three `synced` lines.

- [ ] **Step 5: Write the failing Playwright round-trip test**

`chrome-hara/playwright.config.js`:

```js
import { defineConfig } from "@playwright/test";
export default defineConfig({
  testDir: "test",
  testMatch: "*.spec.js",
  timeout: 30000,
});
```

`chrome-hara/test/extension.js`:

```js
import { chromium } from "@playwright/test";
import path from "node:path";
import { fileURLToPath } from "node:url";

export async function launchWithExtension() {
  const extensionPath = path.resolve(path.dirname(fileURLToPath(import.meta.url)), "..");
  const context = await chromium.launchPersistentContext("", {
    headless: false,
    args: [
      `--disable-extensions-except=${extensionPath}`,
      `--load-extension=${extensionPath}`,
    ],
  });
  const existing = context.serviceWorkers()[0];
  const serviceWorker = existing ?? (await context.waitForEvent("serviceworker"));
  const extensionId = new URL(serviceWorker.url()).host;
  return { context, serviceWorker, extensionId };
}

export async function activeTabId(serviceWorker) {
  return serviceWorker.evaluate(
    async () => (await chrome.tabs.query({ active: true, currentWindow: true }))[0].id,
  );
}
```

`chrome-hara/test/repl.spec.js`:

```js
import { test, expect } from "@playwright/test";
import { launchWithExtension, activeTabId } from "./extension.js";

test("panel evals hara through the raw wasm runtime", async () => {
  const { context, extensionId } = await launchWithExtension();
  const page = await context.newPage();
  await page.goto(`chrome-extension://${extensionId}/src/panel.html?tabId=0`);
  await page.waitForFunction(() => globalThis.hara !== undefined);
  const value = await page.evaluate(() => globalThis.hara.evalSource("(+ 1 2)"));
  expect(Number(value)).toBe(3);
  await context.close();
});

test("host/call round-trips panel -> service worker -> panel", async () => {
  const { context, extensionId, serviceWorker } = await launchWithExtension();
  const target = await context.newPage();
  await target.goto("about:blank");
  const tabId = await activeTabId(serviceWorker);
  const page = await context.newPage();
  await page.goto(`chrome-extension://${extensionId}/src/panel.html?tabId=${tabId}`);
  await page.waitForFunction(() => globalThis.hara !== undefined);
  const value = await page.evaluate(() =>
    globalThis.hara.evalSource('(deref (host/call "hara" "echo" 42))'),
  );
  expect(Number(value)).toBe(42);
  await context.close();
});
```

Run: `cd chrome-hara && npm run test:browser`
Expected: FAIL — test 1 may pass already; test 2 passes too if the echo works (that is fine — TDD red came from Step 2). The real gate: both tests PASS.

- [ ] **Step 6: Run and verify**

Run: `cd chrome-hara && npm run test:browser`
Expected: 2 passed. If `xvfb-run` is unavailable locally, run `npx playwright test` with a display; document in README.

- [ ] **Step 7: README + commit**

`chrome-hara/README.md`:

```markdown
# chrome-hara

Chrome (MV3) extension embedding the hara wasm runtime in a DevTools panel.

## Build

    npm install
    npm run build   # builds wasm/raw + copies vendor files

## Load

`chrome://extensions` -> developer mode -> "Load unpacked" -> select this directory.
Open DevTools -> the "hara" panel.

## Test

    npm test               # node unit tests
    npm run test:browser   # playwright (uses xvfb-run)
```

```bash
git add chrome-hara
git commit -m "feat(chrome-hara): extension skeleton with devtools panel repl"
```

---

### Task 3: Service worker — generic chrome proxy + first-class chrome.debugger

**Files:**
- Modify: `chrome-hara/src/background.js` (replace `dispatch`)
- Test: `chrome-hara/test/debugger.spec.js`

**Interfaces:**
- Consumes: port protocol from Task 2.
- Produces:
  - Generic proxy: service `"chrome.<path>"` (e.g. `chrome.tabs`), any method → promisified `chrome.<path>.<method>(...args)`. Non-`chrome.` services are rejected.
  - First-class `chrome.debugger` methods with hara-friendly shapes:
    - `attach(tabId) -> nil`, `detach(tabId) -> nil`
    - `sendCommand(tabId, command, params?) -> result object`
    - `next-event(tabId) -> {method, params}` (long-poll; resolves when the next debugger event for that tab arrives)

- [ ] **Step 1: Write the failing Playwright test**

`chrome-hara/test/debugger.spec.js`:

```js
import { test, expect } from "@playwright/test";
import { launchWithExtension, activeTabId } from "./extension.js";

test("hara attaches to a tab and evaluates JS via CDP", async () => {
  const { context, extensionId, serviceWorker } = await launchWithExtension();
  const target = await context.newPage();
  await target.goto("about:blank");
  const tabId = await activeTabId(serviceWorker);
  const page = await context.newPage();
  await page.goto(`chrome-extension://${extensionId}/src/panel.html?tabId=${tabId}`);
  await page.waitForFunction(() => globalThis.hara !== undefined);
  const value = await page.evaluate(async (tab) => {
    const src = `
      (do
        (deref (host/call "chrome.debugger" "attach" ${tab}))
        (deref (host/call "chrome.debugger" "sendCommand" ${tab} "Runtime.evaluate"
                           {:expression "21 + 21" :returnByValue true})))`;
    const result = await globalThis.hara.evalSource(src);
    const get = (map, key) => [...map].find(([k]) => k.name === key)?.[1];
    return Number(get(get(result, "result"), "value"));
  }, tabId);
  expect(value).toBe(42);
  await context.close();
});

test("generic proxy exposes chrome.tabs", async () => {
  const { context, extensionId, serviceWorker } = await launchWithExtension();
  const target = await context.newPage();
  await target.goto("about:blank");
  await activeTabId(serviceWorker);
  const page = await context.newPage();
  await page.goto(`chrome-extension://${extensionId}/src/panel.html?tabId=0`);
  await page.waitForFunction(() => globalThis.hara !== undefined);
  const count = await page.evaluate(async () => {
    const result = await globalThis.hara.evalSource(
      '(count (deref (host/call "chrome.tabs" "query" {})))',
    );
    return Number(result);
  });
  expect(count).toBeGreaterThanOrEqual(2);
  await context.close();
});
```

Run: `cd chrome-hara && npm run test:browser -- --grep "chrome.tabs"`
Expected: FAIL — `host-call-denied: chrome.tabs/query`.

- [ ] **Step 2: Implement dispatch in `src/background.js`**

Replace the whole file with:

```js
const debuggerEvents = new Map(); // tabId -> { queue: [{method, params}], waiters: [resolve] }

chrome.runtime.onConnect.addListener((port) => {
  if (port.name !== "hara-host") return;
  port.onMessage.addListener(async ({ id, service, method, args }) => {
    try {
      const value = await dispatch(service, method, args ?? []);
      port.postMessage({ id, ok: true, value });
    } catch (error) {
      port.postMessage({ id, ok: false, error: String(error?.message ?? error) });
    }
  });
});

chrome.debugger.onEvent.addListener((source, method, params) => {
  const entry = debuggerEvents.get(source.tabId) ?? { queue: [], waiters: [] };
  const waiter = entry.waiters.shift();
  if (waiter) waiter({ method, params });
  else entry.queue.push({ method, params });
  debuggerEvents.set(source.tabId, entry);
});

async function dispatch(service, method, args) {
  if (service === "hara" && method === "echo") return args[0] ?? null;
  if (service === "chrome.debugger") return debuggerCall(method, args);
  if (!service.startsWith("chrome.")) {
    throw new Error(`host-call-denied: ${service}`);
  }
  const owner = service
    .slice("chrome.".length)
    .split(".")
    .reduce((value, key) => value?.[key], chrome);
  const fn = owner?.[method];
  if (typeof fn !== "function") {
    throw new Error(`unknown chrome api: ${service}/${method}`);
  }
  return (await fn.apply(owner, args)) ?? null;
}

async function debuggerCall(method, args) {
  const [tabId, ...rest] = args;
  switch (method) {
    case "attach":
      await chrome.debugger.attach({ tabId }, "1.3");
      return null;
    case "detach":
      await chrome.debugger.detach({ tabId });
      return null;
    case "sendCommand": {
      const [command, params] = rest;
      return (await chrome.debugger.sendCommand({ tabId }, command, params ?? {})) ?? null;
    }
    case "next-event": {
      const entry = debuggerEvents.get(tabId) ?? { queue: [], waiters: [] };
      const queued = entry.queue.shift();
      if (queued) return queued;
      return new Promise((resolve) => {
        entry.waiters.push(resolve);
        debuggerEvents.set(tabId, entry);
      });
    }
    default:
      throw new Error(`unknown chrome.debugger method: ${method}`);
  }
}
```

- [ ] **Step 3: Run tests to verify they pass**

Run: `cd chrome-hara && npm run test:browser`
Expected: 4 passed (2 from Task 2 + 2 new).

- [ ] **Step 4: Commit**

```bash
git add chrome-hara/src/background.js chrome-hara/test/debugger.spec.js
git commit -m "feat(chrome-hara): chrome proxy and debugger host calls"
```

---

### Task 4: The `chrome.api` hara namespace

**Files:**
- Create: `chrome-hara/src/hara/api.hal`
- Modify: `chrome-hara/src/panel.js` (register the resource at startup)
- Test: `chrome-hara/test/api.spec.js`

**Interfaces:**
- Consumes: HTA target `"register-resource"` (Task 1), debugger/proxy host calls (Task 3).
- Produces — hara namespace `chrome.api` with:
  - `(call service method)` / `(call service method a)` / `(call service method a b)` / `(call service method a b c)` → dereffed `host/call`
  - `(attach tab-id)`, `(detach tab-id)`, `(send-command tab-id method)` / `(send-command tab-id method params)`, `(next-event tab-id)`
  - `(tabs-query query-map)`, `(navigate tab-id url)`, `(eval-js tab-id expression)`

- [ ] **Step 1: Write the failing test**

`chrome-hara/test/api.spec.js`:

```js
import { test, expect } from "@playwright/test";
import { launchWithExtension, activeTabId } from "./extension.js";

test("(require [chrome.api :as api]) drives a tab", async () => {
  const { context, extensionId, serviceWorker } = await launchWithExtension();
  const target = await context.newPage();
  await target.goto("about:blank");
  const tabId = await activeTabId(serviceWorker);
  const page = await context.newPage();
  await page.goto(`chrome-extension://${extensionId}/src/panel.html?tabId=${tabId}`);
  await page.waitForFunction(() => globalThis.hara !== undefined);
  const value = await page.evaluate(async (tab) => {
    await globalThis.hara.evalSource("(require [chrome.api :as api])");
    await globalThis.hara.evalSource(`(api/attach ${tab})`);
    await globalThis.hara.evalSource(`(api/navigate ${tab} "data:text/html,<title>hara</title>")`);
    const result = await globalThis.hara.evalSource(
      `(api/eval-js ${tab} "document.title")`,
    );
    const get = (map, key) => [...map].find(([k]) => k.name === key)?.[1];
    return get(get(result, "result"), "value");
  }, tabId);
  expect(value).toBe("hara");
  await context.close();
});
```

Run: `cd chrome-hara && npm run test:browser -- --grep "chrome.api"`
Expected: FAIL — `Cannot require missing namespace: chrome.api`.

- [ ] **Step 2: Write `src/hara/api.hal` and register it**

`chrome-hara/src/hara/api.hal` (core-only syntax; multi-arity `defn` is core — see `foundation.hal:11-13`):

```clojure
; Chrome API bindings for hara. Loaded as a registered resource; use via
; (require [chrome.api :as api])
(ns chrome.api)

(defn call
  ([service method] (deref (host/call service method)))
  ([service method a] (deref (host/call service method a)))
  ([service method a b] (deref (host/call service method a b)))
  ([service method a b c] (deref (host/call service method a b c))))

(defn attach [tab-id]
  (call "chrome.debugger" "attach" tab-id))

(defn detach [tab-id]
  (call "chrome.debugger" "detach" tab-id))

(defn send-command
  ([tab-id method] (call "chrome.debugger" "sendCommand" tab-id method))
  ([tab-id method params] (call "chrome.debugger" "sendCommand" tab-id method params)))

(defn next-event [tab-id]
  (call "chrome.debugger" "next-event" tab-id))

(defn tabs-query [query]
  (call "chrome.tabs" "query" query))

(defn navigate [tab-id url]
  (do
    (send-command tab-id "Page.enable")
    (send-command tab-id "Page.navigate" {:url url})))

(defn eval-js [tab-id expression]
  (send-command tab-id "Runtime.evaluate" {:expression expression :returnByValue true}))
```

In `chrome-hara/src/panel.js`, after the `context` creation, add:

```js
const apiSource = await (
  await fetch(chrome.runtime.getURL("src/hara/api.hal"))
).text();
await context.call("register-resource", ["chrome.api", apiSource]);
```

Place it before `window.hara = {...}` so tests never race it.

- [ ] **Step 3: Run tests to verify they pass**

Run: `cd chrome-hara && npm run test:browser`
Expected: 5 passed.

- [ ] **Step 4: Commit**

```bash
git add chrome-hara/src/hara/api.hal chrome-hara/src/panel.js chrome-hara/test/api.spec.js
git commit -m "feat(chrome-hara): chrome.api namespace with debugger wrappers"
```

---

### Task 5: Home directory + `.hal` loading (File System Access API)

**Files:**
- Create: `chrome-hara/src/home.js`
- Modify: `chrome-hara/src/panel.js` (home button, run-file button, preloader hook in eval)
- Test: `chrome-hara/test/home.test.js` (node, fake dir handle), `chrome-hara/test/home.spec.js` (playwright, fake handle injected)

**Interfaces:**
- Consumes: `context.call("register-resource", [name, source])`, `evalSource`.
- Produces:
  - `requireSpecs(source) -> [{ns, alias}]` (pure)
  - `nsToPath(ns) -> string` (`chrome.api` → `chrome/api.hal`; `-` → `_`)
  - `parseSourcePaths(projectHalText) -> string[]`
  - `readNsSource(dirHandle, sourcePaths, ns) -> Promise<string|null>` (injectable handle)
  - `preloadRequires(source, {dir, sourcePaths, register, loaded}) -> Promise<void>` (recursive)
  - `chooseHome()`, `restoreHome()`, `readHomeFile(fileHandle)` for the panel.

- [ ] **Step 1: Write the failing node test**

`chrome-hara/test/home.test.js`:

```js
import { test } from "node:test";
import assert from "node:assert/strict";
import {
  requireSpecs,
  nsToPath,
  parseSourcePaths,
  readNsSource,
  preloadRequires,
} from "../src/home.js";

test("requireSpecs finds ns-form and standalone requires", () => {
  const source = `
    (ns demo (:require [acme.tools :as tools] [acme.more]))
    (require [extra.lib :as x])
    (tools/seven)`;
  assert.deepEqual(requireSpecs(source), [
    { ns: "acme.tools", alias: "tools" },
    { ns: "acme.more", alias: null },
    { ns: "extra.lib", alias: "x" },
  ]);
});

test("nsToPath follows the hara project convention", () => {
  assert.equal(nsToPath("chrome.api"), "chrome/api.hal");
  assert.equal(nsToPath("my-app.deep.lib-name"), "my_app/deep/lib_name.hal");
});

test("parseSourcePaths reads project.hal or defaults", () => {
  assert.deepEqual(parseSourcePaths('(defproject demo {:source-paths ["src" "lib"]})'), ["src", "lib"]);
  assert.deepEqual(parseSourcePaths("(defproject demo {})"), ["."]);
});

function fakeDir(files) {
  return {
    async getFileHandle(name) {
      if (!(name in files)) throw new Error(`NotFound: ${name}`);
      return { async getFile() { return { async text() { return files[name]; } }; } };
    },
    async getDirectoryHandle(name) {
      const prefix = `${name}/`;
      const sub = Object.fromEntries(
        Object.entries(files)
          .filter(([p]) => p.startsWith(prefix))
          .map(([p, v]) => [p.slice(prefix.length), v]),
      );
      if (Object.keys(sub).length === 0) throw new Error(`NotFound: ${name}`);
      return fakeDir(sub);
    },
  };
}

test("readNsSource resolves through source paths", async () => {
  const dir = fakeDir({ "src/acme/tools.hal": "(ns acme.tools)" });
  assert.equal(await readNsSource(dir, ["src"], "acme.tools"), "(ns acme.tools)");
  assert.equal(await readNsSource(dir, ["src"], "no.such.ns"), null);
});

test("preloadRequires registers transitively, std and loaded skipped", async () => {
  const dir = fakeDir({
    "acme/tools.hal": "(ns acme.tools (:require [acme.base])) (defn seven [] 7)",
    "acme/base.hal": "(ns acme.base) (defn three [] 3)",
  });
  const registered = [];
  const loaded = new Set(["chrome.api"]);
  await preloadRequires("(require [acme.tools :as t] [chrome.api :as api] [std.lib.foundation])", {
    dir,
    sourcePaths: ["."],
    register: async (ns, src) => registered.push([ns, src]),
    loaded,
  });
  assert.deepEqual(
    registered.map(([ns]) => ns).sort(),
    ["acme.base", "acme.tools"],
  );
  assert.ok(loaded.has("acme.tools") && loaded.has("acme.base"));
});
```

Run: `cd chrome-hara && node --test test/home.test.js`
Expected: FAIL — `../src/home.js` does not exist.

- [ ] **Step 2: Implement `src/home.js`**

```js
const SPEC_PATTERN = /\[([a-z][a-z0-9-]*(?:\.[a-z0-9-]+)+)(?:\s+:as\s+([a-z][a-z0-9-]*))?[^\]]*\]/g;

/** Extract require specs ([ns :as alias] vectors) from ns and require forms.
 *  Naive by design: scans regions after `require` tokens; may over-match in
 *  strings/comments, which is harmless (extra files get registered). */
export function requireSpecs(source) {
  const specs = [];
  const seen = new Set();
  const requireToken = /require/g;
  let match;
  while ((match = requireToken.exec(source)) !== null) {
    SPEC_PATTERN.lastIndex = match.index;
    let spec;
    while ((spec = SPEC_PATTERN.exec(source)) !== null) {
      if (spec.index > match.index + 400) break; // specs live right after the token
      const ns = spec[1];
      if (!seen.has(ns)) {
        seen.add(ns);
        specs.push({ ns, alias: spec[2] ?? null });
      }
      if (source.indexOf(")", match.index) < spec.index) break;
    }
  }
  return specs;
}

export function nsToPath(ns) {
  return `${ns.replaceAll(".", "/").replaceAll("-", "_")}.hal`;
}

export function parseSourcePaths(projectHal) {
  const match = /:source-paths\s*\[([^\]]*)\]/.exec(projectHal);
  if (!match) return ["."];
  const paths = [...match[1].matchAll(/"([^"]+)"/g)].map((m) => m[1]);
  return paths.length > 0 ? paths : ["."];
}

export async function readNsSource(dir, sourcePaths, ns) {
  const rel = nsToPath(ns);
  for (const base of sourcePaths) {
    try {
      const parts = base === "." ? [] : base.split("/");
      let current = dir;
      for (const part of parts) current = await current.getDirectoryHandle(part);
      const segments = rel.split("/");
      for (const segment of segments.slice(0, -1)) {
        current = await current.getDirectoryHandle(segment);
      }
      const fileHandle = await current.getFileHandle(segments.at(-1));
      return await (await fileHandle.getFile()).text();
    } catch {
      continue;
    }
  }
  return null;
}

/** Recursively register every required namespace found in `source`. */
export async function preloadRequires(source, { dir, sourcePaths, register, loaded }) {
  for (const { ns } of requireSpecs(source)) {
    if (loaded.has(ns) || ns.startsWith("std.lib.")) continue;
    const text = await readNsSource(dir, sourcePaths, ns);
    if (text === null) throw new Error(`Cannot require missing namespace: ${ns}`);
    loaded.add(ns);
    await preloadRequires(text, { dir, sourcePaths, register, loaded });
    await register(ns, text);
  }
}

const HOME_KEY = "chrome-hara-home";

function idb() {
  return new Promise((resolve, reject) => {
    const request = indexedDB.open("chrome-hara", 1);
    request.onupgradeneeded = () => request.result.createObjectStore("kv");
    request.onsuccess = () => resolve(request.result);
    request.onerror = () => reject(request.error);
  });
}

export async function chooseHome() {
  const dir = await showDirectoryPicker({ mode: "read" });
  const db = await idb();
  await new Promise((resolve, reject) => {
    const tx = db.transaction("kv", "readwrite");
    tx.objectStore("kv").put(dir, HOME_KEY);
    tx.oncomplete = resolve;
    tx.onerror = () => reject(tx.error);
  });
  return dir;
}

export async function restoreHome() {
  const db = await idb();
  const dir = await new Promise((resolve, reject) => {
    const request = db.transaction("kv").objectStore("kv").get(HOME_KEY);
    request.onsuccess = () => resolve(request.result ?? null);
    request.onerror = () => reject(request.error);
  });
  if (dir && (await dir.queryPermission({ mode: "read" })) !== "granted") {
    if ((await dir.requestPermission({ mode: "read" })) !== "granted") return null;
  }
  return dir;
}
```

Run: `cd chrome-hara && node --test test/home.test.js`
Expected: PASS (5 tests). Note `chooseHome`/`restoreHome` are browser-only and untested in node — covered by the spec below via injected fakes.

- [ ] **Step 3: Wire into `panel.js` + failing browser test**

Add to `chrome-hara/src/panel.js` (imports at top):

```js
import { preloadRequires, parseSourcePaths, chooseHome, restoreHome } from "./home.js";
```

Replace the plain `evalSource` with a preloading one, add home state, and wire the buttons (insert after `window.hara` assignment):

```js
let homeDir = null;
let homeSourcePaths = ["."];
const loadedResources = new Set(["chrome.api"]);
const register = (ns, text) => context.call("register-resource", [ns, text]);

async function preload(source) {
  if (!homeDir) return;
  await preloadRequires(source, {
    dir: homeDir,
    sourcePaths: homeSourcePaths,
    register,
    loaded: loadedResources,
  });
}

const homeLabel = document.getElementById("home-label");
async function setHome(dir) {
  homeDir = dir;
  homeLabel.textContent = dir ? `home: ${dir.name}` : "no home";
  homeSourcePaths = ["."];
  if (dir) {
    try {
      const projectHal = await (
        await (await dir.getFileHandle("project.hal")).getFile()
      ).text();
      homeSourcePaths = parseSourcePaths(projectHal);
    } catch { /* no project.hal — default paths */ }
  }
}

document.getElementById("home-button").addEventListener("click", async () => {
  try { setHome(await chooseHome()); } catch { /* picker cancelled */ }
});
document.getElementById("run-file-button").addEventListener("click", async () => {
  try {
    const [fileHandle] = await showOpenFilePicker({
      types: [{ description: "hara", accept: { "text/plain": [".hal"] } }],
    });
    const source = await (await fileHandle.getFile()).text();
    await preload(source);
    print(`hara=> ${fileHandle.name}`);
    print(String(await evalSource(source)));
  } catch (error) {
    if (error?.name !== "AbortError") print(`error: ${error?.message ?? error}`);
  }
});
setHome(await restoreHome());
```

And change the REPL keydown handler to `await preload(source);` before `evalSource(source)`. Expose test hooks by extending `window.hara`:

```js
window.hara = { context, evalSource, preload, setHome, tabId };
```

`chrome-hara/test/home.spec.js`:

```js
import { test, expect } from "@playwright/test";
import { launchWithExtension, activeTabId } from "./extension.js";

const FAKE_HOME = `
  ({
    name: "fake-home",
    async getFileHandle(name) {
      const files = {
        "project.hal": '(defproject demo {:source-paths ["src"]})',
      };
      if (name in files) return { async getFile() { return { async text() { return files[name]; } }; } };
      throw new Error("NotFound: " + name);
    },
    async getDirectoryHandle(name) {
      if (name !== "src") throw new Error("NotFound: " + name);
      const files = {
        "acme/tools.hal": "(ns acme.tools (:require [acme.base])) (defn total [] (+ (acme.base/three) 4))",
        "acme/base.hal": "(ns acme.base) (defn three [] 3)",
      };
      return {
        async getDirectoryHandle(sub) {
          if (sub !== "acme") throw new Error("NotFound: " + sub);
          return {
            async getFileHandle(file) {
              const key = "acme/" + file;
              if (key in files) return { async getFile() { return { async text() { return files[key]; } }; } };
              throw new Error("NotFound: " + file);
            },
          };
        },
      };
    },
  })
`;

test("requires resolve from the home directory transitively", async () => {
  const { context, extensionId } = await launchWithExtension();
  const page = await context.newPage();
  await page.goto(`chrome-extension://${extensionId}/src/panel.html?tabId=0`);
  await page.waitForFunction(() => globalThis.hara !== undefined);
  const value = await page.evaluate(async (fakeHomeSource) => {
    const dir = eval(fakeHomeSource);
    await globalThis.hara.setHome(dir);
    const source = "(require [acme.tools :as t]) (t/total)";
    await globalThis.hara.preload(source);
    return globalThis.hara.evalSource(source);
  }, FAKE_HOME);
  expect(Number(value)).toBe(7);
  await context.close();
});
```

Run: `cd chrome-hara && npm run test:browser -- --grep "home directory"`
Expected: FAIL before wiring (no `setHome`/`preload` on `window.hara`), PASS after.

- [ ] **Step 4: Run full suites**

Run: `cd chrome-hara && npm test && npm run test:browser`
Expected: node tests pass; 6 playwright tests pass.

- [ ] **Step 5: Commit**

```bash
git add chrome-hara/src/home.js chrome-hara/src/panel.js chrome-hara/test/home.test.js chrome-hara/test/home.spec.js chrome-hara/src/panel.html
git commit -m "feat(chrome-hara): home directory with .hal require preloading"
```

---

### Task 6 (stretch): RESP eval endpoint via local bridge

MV3 extensions cannot listen on TCP. The bridge is a node process: RESP2 TCP server ⟷ WebSocket server; the panel dials the WS outbound. Deferrable without affecting Tasks 1-5.

**Files:**
- Create: `chrome-hara/bridge/resp-bridge.mjs`
- Create: `chrome-hara/src/resp-client.js`
- Modify: `chrome-hara/src/panel.js`, `chrome-hara/src/panel.html` (RESP connect toggle)
- Test: `chrome-hara/test/resp-bridge.test.js` (node, fake extension client), `chrome-hara/test/resp.spec.js` (full stack)

**Interfaces:**
- Consumes: `window.hara.evalSource`, `ws` package.
- Produces:
  - `startBridge({respPort, wsPort, evalSource?}) -> {close()}` in `bridge/resp-bridge.mjs` (importable for tests; CLI: `node bridge/resp-bridge.mjs [respPort] [wsPort]`, defaults 7355/7356).
  - RESP commands: `PING` → `+PONG`, `EVAL <source>` → bulk string of the rendered result or `-ERR ...`, `INFO` → bulk string, `QUIT` → `+OK` + close. `HELLO` → `+OK` (subset; no v4 streaming, no sessions — deliberate).
  - Panel: `connectResp(url)` in `src/resp-client.js`; auto-connect when panel URL has `?resp=ws://...`.

- [ ] **Step 1: Write the failing node test**

`chrome-hara/test/resp-bridge.test.js`:

```js
import { test } from "node:test";
import assert from "node:assert/strict";
import net from "node:net";
import WebSocket from "ws";
import { startBridge } from "../bridge/resp-bridge.mjs";

function respClient(port, chunks) {
  return new Promise((resolve, reject) => {
    const socket = net.connect(port, "127.0.0.1");
    let data = "";
    socket.on("data", (chunk) => (data += chunk));
    socket.on("connect", () => socket.end(chunks));
    socket.on("end", () => resolve(data));
    socket.on("error", reject);
  });
}

const bulk = (s) => `$${Buffer.byteLength(s)}\r\n${s}\r\n`;

test("EVAL forwards to the extension and replies bulk", async () => {
  const bridge = await startBridge({ respPort: 17355, wsPort: 17356 });
  const extension = new WebSocket("ws://127.0.0.1:17356");
  extension.on("message", (raw) => {
    const { id, source } = JSON.parse(raw);
    extension.send(JSON.stringify({ id, ok: true, value: `echoed:${source}` }));
  });
  await new Promise((resolve) => extension.on("open", resolve));
  const reply = await respClient(17355, `*2\r\n${bulk("EVAL")}${bulk("(+ 1 2)")}*1\r\n${bulk("QUIT")}`);
  assert.ok(reply.includes("$13\r\nechoed:(+ 1 2)\r\n"), reply);
  assert.ok(reply.endsWith("+OK\r\n"), reply);
  await bridge.close();
});

test("EVAL without a connected extension is an error", async () => {
  const bridge = await startBridge({ respPort: 17357, wsPort: 17358 });
  const reply = await respClient(17357, `*2\r\n${bulk("EVAL")}${bulk("1")}*1\r\n${bulk("QUIT")}`);
  assert.ok(reply.startsWith("-ERR hara extension not connected"), reply);
  await bridge.close();
});
```

Run: `cd chrome-hara && node --test test/resp-bridge.test.js`
Expected: FAIL — `../bridge/resp-bridge.mjs` does not exist.

- [ ] **Step 2: Implement the bridge**

`chrome-hara/bridge/resp-bridge.mjs`:

```js
import net from "node:net";
import { WebSocketServer } from "ws";

/** Minimal RESP2 reader: arrays of bulk/simple strings only. */
function createRespParser(onCommand) {
  let buffer = Buffer.alloc(0);
  return (chunk) => {
    buffer = Buffer.concat([buffer, chunk]);
    for (;;) {
      const command = readCommand(buffer);
      if (!command) return;
      buffer = buffer.subarray(command.consumed);
      onCommand(command.args);
    }
  };
}

function readCommand(buffer) {
  if (buffer.length === 0 || buffer[0] !== 0x2a /* * */) return null;
  const lineEnd = buffer.indexOf("\r\n");
  if (lineEnd === -1) return null;
  const count = Number(buffer.subarray(1, lineEnd).toString());
  const args = [];
  let cursor = lineEnd + 2;
  for (let i = 0; i < count; i++) {
    if (buffer[cursor] !== 0x24 /* $ */) return null;
    const sizeEnd = buffer.indexOf("\r\n", cursor);
    if (sizeEnd === -1) return null;
    const size = Number(buffer.subarray(cursor + 1, sizeEnd).toString());
    if (buffer.length < sizeEnd + 2 + size + 2) return null;
    args.push(buffer.subarray(sizeEnd + 2, sizeEnd + 2 + size).toString());
    cursor = sizeEnd + 2 + size + 2;
  }
  return { args, consumed: cursor };
}

export async function startBridge({ respPort = 7355, wsPort = 7356 }) {
  let extension = null;
  const pending = new Map();
  let next = 1;

  const wss = new WebSocketServer({ port: wsPort, host: "127.0.0.1" });
  wss.on("connection", (socket) => {
    extension = socket;
    socket.on("message", (raw) => {
      const { id, ok, value, error } = JSON.parse(raw);
      const entry = pending.get(id);
      if (!entry) return;
      pending.delete(id);
      ok ? entry.resolve(value) : entry.reject(new Error(error ?? "eval failed"));
    });
    socket.on("close", () => {
      if (extension === socket) extension = null;
    });
  });

  const evalInExtension = (source) =>
    new Promise((resolve, reject) => {
      if (!extension) {
        reject(new Error("hara extension not connected"));
        return;
      }
      const id = next++;
      pending.set(id, { resolve, reject });
      extension.send(JSON.stringify({ id, source }));
    });

  const server = net.createServer((socket) => {
    const write = {
      simple: (s) => socket.write(`+${s}\r\n`),
      error: (s) => socket.write(`-ERR ${s}\r\n`),
      bulk: (s) => socket.write(`$${Buffer.byteLength(s)}\r\n${s}\r\n`),
    };
    socket.on(
      "data",
      createRespParser(async (args) => {
        const command = (args[0] ?? "").toUpperCase();
        try {
          switch (command) {
            case "PING": write.simple("PONG"); break;
            case "HELLO": write.simple("OK"); break;
            case "INFO": write.bulk("chrome-hara resp bridge (subset: PING HELLO EVAL INFO QUIT)"); break;
            case "EVAL": write.bulk(String(await evalInExtension(args[1] ?? ""))); break;
            case "QUIT": write.simple("OK"); socket.end(); break;
            default: write.error(`unknown command: ${command}`);
          }
        } catch (error) {
          write.error(String(error?.message ?? error));
        }
      }),
    );
  });

  await Promise.all([
    new Promise((resolve) => server.listen(respPort, "127.0.0.1", resolve)),
    new Promise((resolve) => wss.on("listening", resolve)),
  ]);
  return {
    close: () =>
      Promise.all([
        new Promise((resolve) => server.close(resolve)),
        new Promise((resolve) => wss.close(resolve)),
      ]),
  };
}

if (process.argv[1] && import.meta.url === new URL(`file://${process.argv[1]}`).href) {
  const respPort = Number(process.argv[2] ?? 7355);
  const wsPort = Number(process.argv[3] ?? 7356);
  await startBridge({ respPort, wsPort });
  console.log(`chrome-hara resp bridge: resp=127.0.0.1:${respPort} ws=127.0.0.1:${wsPort}`);
}
```

Run: `cd chrome-hara && node --test test/resp-bridge.test.js`
Expected: PASS (2 tests).

- [ ] **Step 3: Panel RESP client + full-stack test**

`chrome-hara/src/resp-client.js`:

```js
import { HtaKeyword } from "../vendor/hta.js";

/** Render an HTA value as a display string for RESP replies. */
export function renderHta(value) {
  if (value === null || value === undefined) return "nil";
  if (value instanceof HtaKeyword) return `:${value.name}`;
  if (value instanceof Map) {
    return `{${[...value].map(([k, v]) => `${renderHta(k)} ${renderHta(v)}`).join(", ")}}`;
  }
  if (value instanceof Set) return `#{${[...value].map(renderHta).join(" ")}}`;
  if (Array.isArray(value)) return `[${value.map(renderHta).join(" ")}]`;
  if (typeof value === "string") return value;
  return String(value);
}

export function connectResp(url, evalSource) {
  const socket = new WebSocket(url);
  socket.onmessage = async (event) => {
    const { id, source } = JSON.parse(event.data);
    try {
      const value = await evalSource(source);
      socket.send(JSON.stringify({ id, ok: true, value: renderHta(value) }));
    } catch (error) {
      socket.send(JSON.stringify({ id, ok: false, error: String(error?.message ?? error) }));
    }
  };
  return socket;
}
```

In `chrome-hara/src/panel.js`, after `window.hara` assignment:

```js
import { connectResp } from "./resp-client.js"; // (move to top imports)
// ...
if (params.has("resp")) connectResp(params.get("resp"), evalSource);
```

`chrome-hara/test/resp.spec.js`:

```js
import { test, expect } from "@playwright/test";
import { spawn } from "node:child_process";
import net from "node:net";
import path from "node:path";
import { fileURLToPath } from "node:url";
import { launchWithExtension } from "./extension.js";

const bridgePath = path.resolve(
  path.dirname(fileURLToPath(import.meta.url)),
  "../bridge/resp-bridge.mjs",
);

function respEval(port, source) {
  const bulk = (s) => `$${Buffer.byteLength(s)}\r\n${s}\r\n`;
  return new Promise((resolve, reject) => {
    const socket = net.connect(port, "127.0.0.1");
    let data = "";
    socket.on("data", (chunk) => (data += chunk));
    socket.on("connect", () => socket.end(`*2\r\n${bulk("EVAL")}${bulk(source)}*1\r\n${bulk("QUIT")}`));
    socket.on("end", () => resolve(data));
    socket.on("error", reject);
  });
}

test("RESP EVAL reaches the hara runtime in the extension", async () => {
  const bridge = spawn("node", [bridgePath, "27355", "27356"], { stdio: "ignore" });
  await new Promise((resolve) => setTimeout(resolve, 500));
  try {
    const { context, extensionId } = await launchWithExtension();
    const page = await context.newPage();
    await page.goto(
      `chrome-extension://${extensionId}/src/panel.html?tabId=0&resp=${encodeURIComponent("ws://127.0.0.1:27356")}`,
    );
    await page.waitForFunction(() => globalThis.hara !== undefined);
    await expect
      .poll(async () => respEval(27355, "(+ 40 2)"), { timeout: 10000 })
      .toContain("$2\r\n42\r\n");
    await context.close();
  } finally {
    bridge.kill();
  }
});
```

Run: `cd chrome-hara && npm run test:browser -- --grep RESP`
Expected: PASS.

- [ ] **Step 4: Full suites + README note + commit**

Run: `cd chrome-hara && npm test && npm run test:browser`
Expected: all node tests pass; 7 playwright tests pass.

Append to `chrome-hara/README.md`:

```markdown
## RESP bridge

    node bridge/resp-bridge.mjs        # resp=7355, ws=7356

Open the panel with `?resp=ws://127.0.0.1:7356` appended (or call
`connectResp(url, hara.evalSource)` from the console), then:

    redis-cli -p 7355 EVAL "(+ 1 2)"
```

```bash
git add chrome-hara/bridge chrome-hara/src/resp-client.js chrome-hara/src/panel.js chrome-hara/test/resp-bridge.test.js chrome-hara/test/resp.spec.js chrome-hara/README.md
git commit -m "feat(chrome-hara): resp eval bridge over websocket"
```

---

## Verification (all tasks done)

- `cargo test --manifest-path wasm/Cargo.toml` — green
- `cargo test --manifest-path wasm/raw/Cargo.toml` — green (3 new tests)
- `cd wasm/web && npm run test:hta` — green
- `cd chrome-hara && npm test` — green (host-bridge, home, resp-bridge unit tests)
- `cd chrome-hara && npm run test:browser` — green (7 tests: repl×2, debugger×2, api, home, resp)
- Manual smoke: load unpacked, open DevTools → hara panel, `(require [chrome.api :as api])`, `(api/attach <tab>)`, `(api/eval-js <tab> "document.title")`

## Known limitations (documented, not bugs)

- Required namespaces must be load-time pure: top-level `deref` of a pending `host/call` during a `require` load is unsupported.
- `requireSpecs` is a scanner, not a reader — it can over-match inside strings/comments (harmless: extra registrations).
- RESP bridge is a subset (no v4 streaming, no sessions) and requires an open hara panel.
- Chrome shows the "debugging this browser" infobar while `chrome.debugger` is attached.

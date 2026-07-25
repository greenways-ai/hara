// Page host — the JS side of hara's `host/call` special form.
//
// console.js hands over the booted Runtime via the `hara:runtime-ready`
// event; this script then:
//   1. fetches the .hal namespaces under hal/ and registers them as runtime
//      resources, so `(:require [host.browser.dom :as dom])` resolves;
//   2. installs `dispatch` as the host handler behind `host/call`.
//
// `dispatch` is the host policy: a whitelist of services. Anything else
// throws, which surfaces in hara as an ordinary eval error.
(() => {
  const RESOURCES = {
    'host.local': 'hal/host/local.hal',
    'host.browser.dom': 'hal/host/browser/dom.hal',
    'host.browser.canvas': 'hal/host/browser/canvas.hal',
    'host.browser.audio': 'hal/host/browser/audio.hal',
    'host.browser.wasm': 'hal/host/browser/wasm.hal',
  };

  const one = (selector) => {
    const el = document.querySelector(selector);
    if (!el) throw new Error(`browser.dom/no-match: ${selector}`);
    return el;
  };

  const dom = {
    count: (selector) => document.querySelectorAll(selector).length,
    text: (selector) => one(selector).textContent,
    'set-text': (selector, text) => {
      one(selector).textContent = String(text);
      return null;
    },
    'set-html': (selector, html) => {
      one(selector).innerHTML = String(html);
      return null;
    },
    'set-style': (selector, k, v) => one(selector).style.setProperty(String(k), String(v)),
    'set-attr': (selector, k, v) => one(selector).setAttribute(String(k), String(v)),
    'add-class': (selector, k) => one(selector).classList.add(String(k)),
    'remove-class': (selector, k) => one(selector).classList.remove(String(k)),
  };

  // 2d contexts are configured once per canvas: backing store matched to the
  // device pixel ratio, so hara draws in CSS pixels.
  const prepared = new WeakSet();
  const ctx2d = (selector) => {
    const el = one(selector);
    if (!(el instanceof HTMLCanvasElement)) {
      throw new Error(`browser.canvas/not-canvas: ${selector}`);
    }
    const ctx = el.getContext('2d');
    if (!prepared.has(el)) {
      const dpr = window.devicePixelRatio || 1;
      if (el.clientWidth && el.clientHeight && dpr !== 1) {
        el.width = Math.round(el.clientWidth * dpr);
        el.height = Math.round(el.clientHeight * dpr);
        ctx.scale(dpr, dpr);
      }
      prepared.add(el);
    }
    return ctx;
  };

  const canvas = {
    clear: (selector) => {
      const el = one(selector);
      ctx2d(selector).clearRect(0, 0, el.width, el.height);
      return null;
    },
    rect: (selector, x, y, w, h, color) => {
      const ctx = ctx2d(selector);
      ctx.fillStyle = String(color);
      ctx.fillRect(x, y, w, h);
      return null;
    },
    circle: (selector, x, y, r, color) => {
      const ctx = ctx2d(selector);
      ctx.fillStyle = String(color);
      ctx.beginPath();
      ctx.arc(x, y, r, 0, Math.PI * 2);
      ctx.fill();
      return null;
    },
    line: (selector, x1, y1, x2, y2, color) => {
      const ctx = ctx2d(selector);
      ctx.strokeStyle = String(color);
      ctx.beginPath();
      ctx.moveTo(x1, y1);
      ctx.lineTo(x2, y2);
      ctx.stroke();
      return null;
    },
  };

  let audioContext = null;
  const live = new Set();
  const audioCtx = () => {
    if (!audioContext) {
      audioContext = new (window.AudioContext || window.webkitAudioContext)();
    }
    if (audioContext.state === 'suspended') audioContext.resume();
    return audioContext;
  };

  const audio = {
    note: (freq, secs, gain) => {
      const ac = audioCtx();
      const osc = ac.createOscillator();
      const amp = ac.createGain();
      osc.frequency.value = freq;
      amp.gain.value = gain;
      osc.connect(amp).connect(ac.destination);
      osc.start();
      osc.stop(ac.currentTime + secs);
      live.add(osc);
      osc.onended = () => live.delete(osc);
      return null;
    },
    stop: () => {
      for (const osc of live) {
        try {
          osc.stop();
        } catch (_) {
          // already stopped
        }
      }
      live.clear();
      return null;
    },
  };

  // The host is the wasm engine: only JS can call WebAssembly.instantiate.
  // Instantiation is async but the bridge is synchronous, so `load` returns
  // an opaque id immediately and starts loading in the background; `call`,
  // `mem-read-*` and `unload` throw `browser.wasm/not-ready` until the
  // instance arrives. Same import rule as core.v1: modules with imports are
  // rejected.
  let nextId = 1;
  const instances = new Map();

  const startLoad = (url) => {
    const entry = { id: nextId++, url, instance: null, error: null };
    instances.set(entry.id, entry);
    (async () => {
      try {
        const response = await fetch(new URL(url, document.baseURI));
        if (!response.ok) throw new Error(`HTTP ${response.status}`);
        const module = await WebAssembly.compile(await response.arrayBuffer());
        if (WebAssembly.Module.imports(module).length > 0) {
          throw new Error('browser.wasm/imports-unsupported');
        }
        entry.instance = new WebAssembly.Instance(module, {});
      } catch (err) {
        entry.error = err;
      }
    })();
    return entry.id;
  };

  const ready = (id) => {
    const entry = instances.get(id);
    if (!entry) throw new Error(`browser.wasm/unknown-id: ${id}`);
    if (entry.error) throw new Error(`browser.wasm/load-failed: ${entry.url}: ${entry.error.message}`);
    if (!entry.instance) throw new Error(`browser.wasm/not-ready: ${entry.url}`);
    return entry.instance;
  };

  const wasm = {
    load: (url) => startLoad(String(url)),
    call: (id, name, ...args) => {
      const fn = ready(id).exports[String(name)];
      if (typeof fn !== 'function') {
        throw new Error(`browser.wasm/no-export: ${name}`);
      }
      // i64 params arrive from hara as numbers, but the wasm JS API needs
      // BigInt for them and export signatures aren't introspectable — so on
      // a BigInt-conversion TypeError, lift the next integer arg and retry.
      const lifted = args.map(() => false);
      let result;
      for (let attempt = 0; ; attempt++) {
        try {
          result = fn(...args);
          break;
        } catch (err) {
          if (!(err instanceof TypeError) || !/BigInt/.test(err.message)) throw err;
          const next = args.findIndex(
            (v, i) => !lifted[i] && typeof v === 'number' && Number.isInteger(v),
          );
          if (next === -1 || attempt >= args.length) throw err;
          lifted[next] = true;
          args[next] = BigInt(args[next]);
        }
      }
      if (typeof result === 'bigint') {
        if (result >= -Number.MAX_SAFE_INTEGER && result <= Number.MAX_SAFE_INTEGER) {
          return Number(result);
        }
        throw new Error(`browser.wasm/i64-result: ${name}`);
      }
      return result === undefined ? null : result;
    },
    'mem-read-f32': (id, ptr, len) => {
      const memory = ready(id).exports.memory;
      if (!memory) throw new Error('browser.wasm/no-memory-export');
      return Array.from(new Float32Array(memory.buffer, ptr, len));
    },
    'mem-read-u8': (id, ptr, len) => {
      const memory = ready(id).exports.memory;
      if (!memory) throw new Error('browser.wasm/no-memory-export');
      return new Uint8Array(memory.buffer, ptr, len);
    },
    unload: (id) => {
      if (!instances.delete(id)) throw new Error(`browser.wasm/unknown-id: ${id}`);
      return null;
    },
  };

  const services = {
    'browser.dom': dom,
    'browser.canvas': canvas,
    'browser.audio': audio,
    'browser.wasm': wasm,
  };

  const dispatch = (service, method, args) => {
    const table = services[service];
    if (!table) throw new Error(`host/unknown-service: ${service}`);
    const fn = table[method];
    if (!fn) throw new Error(`host/unknown-method: ${service}/${method}`);
    return fn(...args);
  };

  const install = async (runtime) => {
    await Promise.all(
      Object.entries(RESOURCES).map(async ([name, path]) => {
        const response = await fetch(new URL(path, document.baseURI));
        if (!response.ok) {
          throw new Error(`host/resource ${path}: HTTP ${response.status}`);
        }
        runtime.register_resource(name, await response.text());
      }),
    );
    runtime.install_host_handler(dispatch);
    // Warm the site's own demo module so the console's first
    // wasm/load -> wasm/call sequence is immediate.
    startLoad('assets/wasm/demo-synth.wasm');
    window.haraPageHostReady = true;
    document.dispatchEvent(new CustomEvent('hara:page-host-ready'));
  };

  document.addEventListener('hara:runtime-ready', (event) => {
    install(event.detail.runtime).catch((err) => console.error('[hara page-host]', err));
  });
})();

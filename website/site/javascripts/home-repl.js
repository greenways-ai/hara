// Home REPL panel — boots a standalone Hara WASM runtime in the parent page
// and wires it to the bottom slide-up REPL panel.
(() => {
  const RESOURCES = {
    'host.local': 'hal/host/local.hal',
    'host.browser.dom': 'hal/host/browser/dom.hal',
    'host.browser.canvas': 'hal/host/browser/canvas.hal',
    'host.browser.audio': 'hal/host/browser/audio.hal',
    'host.browser.wasm': 'hal/host/browser/wasm.hal',
  };

  const escapeHtml = (str) => str
    .replace(/&/g, '&amp;')
    .replace(/</g, '&lt;')
    .replace(/>/g, '&gt;');

  const logEl = () => document.getElementById('hara-repl-log');
  const inputEl = () => document.getElementById('hara-repl-input');
  const panelEl = () => document.getElementById('hara-repl-panel');

  const appendLine = (html) => {
    const el = logEl();
    if (!el) return;
    const line = document.createElement('div');
    line.className = 'hara-repl-line';
    line.innerHTML = html;
    el.appendChild(line);
    el.scrollTop = el.scrollHeight;
  };

  const log = (msg) => {
    appendLine(`<span class="hara-repl-msg">${escapeHtml(String(msg))}</span>`);
  };

  // The parent page has no canvas/audio DOM for the REPL. Reject every
  // host/call so pure forms still evaluate, but browser services fail cleanly.
  const dispatch = (service, method) => {
    throw new Error(`host/unsupported-in-repl: ${service}/${method}`);
  };

  const registerResources = async (runtime) => {
    await Promise.all(
      Object.entries(RESOURCES).map(async ([name, path]) => {
        const response = await fetch(new URL(path, document.baseURI));
        if (!response.ok) {
          throw new Error(`host/resource ${path}: HTTP ${response.status}`);
        }
        runtime.register_resource(name, await response.text());
      }),
    );
  };

  let runtime = null;
  let bootError = null;

  const bootPromise = (async () => {
    try {
      const wasmUrl = new URL('rust/pkg/hara_wasm.js', location.href).href;
      const mod = await import(wasmUrl);
      await mod.default();
      runtime = mod.Runtime.core();
      runtime.install_memory_file_provider('/browser');
      runtime.install_loopback_socket_provider();
      await registerResources(runtime);
      runtime.install_host_handler(dispatch);
      const version = mod.version().replace('hara-wasm/', '');
      log(`;; hara.wasm ${version} ready`);
      return runtime;
    } catch (err) {
      bootError = err;
      console.error('[hara home-repl] boot error', err);
      appendLine(`<span class="hara-repl-error">REPL boot error: ${escapeHtml(String(err))}</span>`);
      throw err;
    }
  })();

  const evalForm = (form) => {
    if (bootError) {
      throw new Error(`REPL not ready: ${bootError}`);
    }
    if (!runtime) {
      throw new Error('REPL runtime is still booting');
    }
    try {
      return runtime.eval(form);
    } catch (err) {
      throw new Error(String(err));
    }
  };

  window.haraRepl = {
    eval: evalForm,
    log,
    clear: () => {
      const el = logEl();
      if (el) el.innerHTML = '';
    },
    ready: bootPromise,
  };

  const focusInput = () => {
    const input = inputEl();
    if (input) input.focus();
  };

  const wireInput = () => {
    const input = inputEl();
    if (!input) return;
    input.addEventListener('keydown', async (event) => {
      if (event.key !== 'Enter') return;
      const source = input.value.trim();
      if (!source) return;
      input.value = '';
      appendLine(`<span class="hara-repl-prompt">› ${escapeHtml(source)}</span>`);
      try {
        await bootPromise;
        const result = window.haraRepl.eval(source);
        appendLine(`<span class="hara-repl-result">${escapeHtml(String(result))}</span>`);
      } catch (err) {
        appendLine(`<span class="hara-repl-error">${escapeHtml(String(err))}</span>`);
      }
    });
  };

  const wireShortcuts = () => {
    const panel = panelEl();
    if (!panel) return;
    panel.addEventListener('keydown', (event) => {
      if ((event.ctrlKey || event.metaKey) && event.key.toLowerCase() === 'k') {
        event.preventDefault();
        window.haraRepl.clear();
      }
    });
  };

  const hookPanelOpen = () => {
    const patch = () => {
      const chrome = window.haraHomeChrome;
      if (!chrome || chrome.__haraReplPatched) return;
      chrome.__haraReplPatched = true;
      const originalOpen = chrome.openRepl;
      chrome.openRepl = function openRepl(...args) {
        const result = originalOpen.apply(this, args);
        focusInput();
        return result;
      };
    };

    if (window.haraHomeChrome) {
      patch();
      return;
    }

    const observer = new MutationObserver(() => {
      if (window.haraHomeChrome) {
        patch();
        observer.disconnect();
      }
    });
    observer.observe(document.documentElement, { childList: true, subtree: true });
    window.setTimeout(() => {
      if (window.haraHomeChrome) patch();
    }, 10000);
  };

  const init = () => {
    wireInput();
    wireShortcuts();
    hookPanelOpen();
  };

  if (document.readyState === 'loading') {
    document.addEventListener('DOMContentLoaded', init);
  } else {
    init();
  }
})();

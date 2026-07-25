// Live hara console on the home page — the REPL mock made real.
//
// Loads the wasm-bindgen runtime from rust/pkg/ (built locally by
// scripts/build-hara-wasm-web; on the deployed site pages.yml places the same
// files under /rust/pkg/). Eval is synchronous over a single Runtime instance.
(() => {
  const root = document.querySelector('[data-hara-component="console"]');
  if (!root) return;

  const log = root.querySelector('[data-console-log]');
  const input = root.querySelector('[data-console-input]');
  const commands = root.querySelectorAll('[data-console-command]');
  const status = (key, value) => {
    const slot = root.querySelector(`[data-status="${key}"]`);
    if (slot) slot.textContent = value;
  };

  const print = (spans, error) => {
    const div = document.createElement('div');
    div.className = 'hara-console-line' + (error ? ' hara-console-line--error' : '');
    for (const [cls, text] of spans) {
      const span = document.createElement('span');
      if (cls) span.className = cls;
      span.textContent = text;
      div.append(span);
    }
    log.append(div);
    log.scrollTop = log.scrollHeight;
  };

  let runtime = null;
  const ready = (async () => {
    try {
      const url = new URL('rust/pkg/hara_wasm.js', document.baseURI).href;
      const mod = await import(url);
      await mod.default();
      runtime = new mod.Runtime();
      runtime.install_memory_file_provider('/browser');
      runtime.install_loopback_socket_provider();
      status('runtime', `WASM · ${mod.version().replace('hara-wasm/', '')}`.toUpperCase());
      status('file', runtime.file_supported() ? 'MEMORY' : '—');
      status('socket', runtime.socket_supported() ? 'LOOPBACK' : '—');
      status('state', 'READY');
    } catch (err) {
      console.error('[hara console]', err);
      status('runtime', 'WASM · ERROR');
      status('state', 'ERROR');
      throw err;
    }
  })();

  const evaluate = async (source) => {
    if (!source) return;
    print([['hara-tty-p', 'hara › '], [null, source]]);
    try {
      await ready;
      const result = runtime.eval(source);
      print([['hara-tty-v', `=> ${result}`]]);
    } catch (err) {
      print([['hara-console-err', `error: ${err}`]], true);
    }
  };

  input.addEventListener('keydown', async (event) => {
    if (event.key !== 'Enter') return;
    const source = input.value.trim();
    input.value = '';
    await evaluate(source);
  });

  commands.forEach((button) => {
    button.addEventListener('click', () => evaluate(button.dataset.consoleCommand));
  });
})();

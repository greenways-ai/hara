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
  const start = document.querySelector('[data-hara-start]');
  const panel = root.querySelector('[data-console-panel]');
  const toggle = root.querySelector('[data-console-toggle]');
  const close = root.querySelector('[data-console-close]');
  const count = root.querySelector('[data-console-count]');
  const led = root.querySelector('[data-runtime-led]');
  let entries = 0;

  const status = (key, value) => {
    const slot = root.querySelector(`[data-status="${key}"]`);
    if (slot) slot.textContent = value;
  };

  const setOpen = (open, focusInput = false) => {
    root.classList.toggle('is-console-open', open);
    if (panel) panel.setAttribute('aria-hidden', String(!open));
    if (toggle) toggle.setAttribute('aria-expanded', String(open));
    if (open && input && focusInput) input.focus();
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
    entries++;
    if (count) count.textContent = String(entries);
    if (toggle) toggle.classList.add('has-activity');
    window.setTimeout(() => toggle && toggle.classList.remove('has-activity'), 650);
  };

  let runtime = null;
  const ready = (async () => {
    try {
      const url = new URL('rust/pkg/hara_wasm.js', document.baseURI).href;
      const mod = await import(url);
      await mod.default();
      runtime = mod.Runtime.core();
      runtime.install_memory_file_provider('/browser');
      runtime.install_loopback_socket_provider();
      status('runtime', `WASM · ${mod.version().replace('hara-wasm/', '')}`.toUpperCase());
      status('file', runtime.file_supported() ? 'MEMORY' : '—');
      status('socket', runtime.socket_supported() ? 'LOOPBACK' : '—');
      status('state', 'READY');
      if (led) led.classList.add('is-ready');
      print([['hara-tty-o', `;; hara.wasm ${mod.version().replace('hara-wasm/', '')} ready`]]);
    } catch (err) {
      console.error('[hara console]', err);
      status('runtime', 'WASM · ERROR');
      status('state', 'ERROR');
      if (led) led.classList.add('is-error');
      print([['hara-console-err', `boot error: ${err}`]], true);
      throw err;
    }
  })();

  const evaluate = async (source) => {
    if (!source) return;
    setOpen(true);
    print([['hara-tty-p', 'hara › '], [null, source]]);
    status('state', 'EVAL');
    try {
      await ready;
      const result = runtime.eval(source);
      print([['hara-tty-v', `=> ${result}`]]);
      status('state', 'READY');
    } catch (err) {
      print([['hara-console-err', `error: ${err}`]], true);
      status('state', 'ERROR');
    }
  };

  input.addEventListener('keydown', async (event) => {
    if (event.key !== 'Enter' || !input) return;
    const source = input.value.trim();
    input.value = '';
    await evaluate(source);
  });

  commands.forEach((button) => {
    button.addEventListener('click', () => evaluate(button.dataset.consoleCommand));
  });

  if (toggle) {
    toggle.addEventListener('click', () => {
      setOpen(!root.classList.contains('is-console-open'), true);
    });
  }

  if (close) {
    close.addEventListener('click', () => {
      setOpen(false);
      if (toggle) toggle.focus();
    });
  }

  if (toggle || panel) {
    document.addEventListener('keydown', (event) => {
      if (event.key === 'Escape' && root.classList.contains('is-console-open')) {
        setOpen(false);
        if (toggle) toggle.focus();
      }
    });
  }

  if (start) {
    start.addEventListener('click', (event) => {
      event.preventDefault();
      evaluate(start.dataset.consoleCommand);
    });
  }
})();

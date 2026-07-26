// Homepage chrome controller for the HARA scene gallery.
// Wires the dots, controls, and slide-in panels to window.haraSceneManager.

function waitForManager(timeout = 10000) {
  const start = performance.now();
  return new Promise((resolve, reject) => {
    if (window.haraSceneManager) {
      resolve(window.haraSceneManager);
      return;
    }
    const check = () => {
      if (window.haraSceneManager) {
        resolve(window.haraSceneManager);
        return;
      }
      if (performance.now() - start > timeout) {
        reject(new Error('haraSceneManager not available'));
        return;
      }
      requestAnimationFrame(check);
    };
    check();
  });
}

function renderDots(manager) {
  const nav = document.getElementById('hara-scene-dots');
  if (!nav || !manager.manifest?.scenes) return;

  nav.innerHTML = '';
  for (const scene of manager.manifest.scenes) {
    const dot = document.createElement('button');
    dot.type = 'button';
    dot.className = 'hara-scene-dot';
    dot.setAttribute('aria-label', scene.title);
    dot.title = scene.title;
    dot.style.setProperty('--hara-dot-color', scene.color || 'var(--hara-cyan)');
    dot.addEventListener('click', () => manager.load(scene.id));
    nav.appendChild(dot);
  }
}

function updateActiveDot(manager) {
  const nav = document.getElementById('hara-scene-dots');
  if (!nav) return;
  const scenes = manager.manifest?.scenes || [];
  const dots = nav.querySelectorAll('.hara-scene-dot');
  dots.forEach((dot, index) => {
    const scene = scenes[index];
    dot.classList.toggle('is-active', scene && manager.currentSceneId === scene.id);
  });
}

function wireControls(manager) {
  document.getElementById('hara-zoom-out')?.addEventListener('click', () => manager.zoomBy(0.8));
  document.getElementById('hara-zoom-in')?.addEventListener('click', () => manager.zoomBy(1.25));
  document.getElementById('hara-pan-up')?.addEventListener('click', () => manager.pan(0, 40));
  document.getElementById('hara-pan-down')?.addEventListener('click', () => manager.pan(0, -40));
  document.getElementById('hara-pan-left')?.addEventListener('click', () => manager.pan(40, 0));
  document.getElementById('hara-pan-right')?.addEventListener('click', () => manager.pan(-40, 0));
}

function wireStatusLed() {
  const led = document.getElementById('hara-wasm-led');
  if (!led) return;
  document.addEventListener('hara:scene-status', (event) => {
    const status = event.detail?.state || 'booting';
    led.className = '';
    led.classList.add(status);
  });
}

function wirePanels() {
  const editorPanel = document.getElementById('hara-editor-panel');
  const replPanel = document.getElementById('hara-repl-panel');

  const openEditor = () => editorPanel?.classList.add('open');
  const closeEditor = () => editorPanel?.classList.remove('open');
  const openRepl = () => replPanel?.classList.add('open');
  const closeRepl = () => replPanel?.classList.remove('open');

  document.getElementById('hara-toggle-editor')?.addEventListener('click', openEditor);
  document.getElementById('hara-editor-close')?.addEventListener('click', closeEditor);
  document.getElementById('hara-toggle-repl')?.addEventListener('click', openRepl);
  document.getElementById('hara-repl-close')?.addEventListener('click', closeRepl);

  return { openEditor, closeEditor, openRepl, closeRepl };
}

async function init() {
  const manager = await waitForManager();
  renderDots(manager);
  wireControls(manager);
  wireStatusLed();

  const panelHelpers = wirePanels();
  window.haraHomeChrome = {
    ...panelHelpers,
    sceneManager: manager,
  };

  const originalLoad = manager.load.bind(manager);
  manager.load = function (sceneId) {
    const result = originalLoad(sceneId);
    updateActiveDot(manager);
    return result;
  };
  updateActiveDot(manager);
}

init().catch((err) => console.error('[hara home] failed to initialize chrome:', err));

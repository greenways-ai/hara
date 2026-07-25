// Scene runner — boots an isolated Hara wasm runtime inside a homepage iframe,
// fetches a scene .hal file, and drives its setup/tick loop.
(() => {
  const params = new URLSearchParams(location.search);
  const sceneId = params.get('scene') || 'fire';
  const sceneUrl = `scenes/${encodeURIComponent(sceneId)}.hal`;

  const status = (state, detail) => {
    parent.postMessage(
      { type: 'hara:scene-status', scene: sceneId, state, detail: detail || '' },
      '*'
    );
  };

  const runtimeReady = (async () => {
    status('booting');
    try {
      const wasmUrl = new URL('rust/pkg/hara_wasm.js', location.href).href;
      const mod = await import(wasmUrl);
      await mod.default();
      const runtime = mod.Runtime.core();
      runtime.install_memory_file_provider('/browser');
      runtime.install_loopback_socket_provider();
      document.dispatchEvent(new CustomEvent('hara:runtime-ready', { detail: { runtime } }));
      status('ready', mod.version().replace('hara-wasm/', ''));
      return runtime;
    } catch (err) {
      status('error', String(err));
      console.error('[scene-runner] boot error', err);
      throw err;
    }
  })();

  const waitForHost = () => new Promise((resolve) => {
    if (window.haraPageHostReady) {
      resolve();
      return;
    }
    document.addEventListener('hara:page-host-ready', () => resolve(), { once: true });
  });

  let camera = { zoom: 1, panX: 0, panY: 0 };
  let sourceOverride = null;
  let sceneStarted = false;

  window.addEventListener('message', (event) => {
    if (event.data && event.data.type === 'hara:camera') {
      camera = event.data.camera;
      return;
    }
    if (event.data && event.data.type === 'hara:scene-source') {
      if (sceneStarted) return;
      if (typeof event.data.source === 'string') {
        sourceOverride = event.data.source;
      }
    }
  });

  (async () => {
    const runtime = await runtimeReady;
    await waitForHost();
    try {
      let source;
      if (sourceOverride !== null) {
        source = sourceOverride;
      } else {
        const response = await fetch(sceneUrl);
        if (!response.ok) throw new Error(`fetch ${sceneUrl}: HTTP ${response.status}`);
        source = await response.text();
      }
      sceneStarted = true;
      runtime.eval(source);
      runtime.eval(`(scenes.${sceneId}/setup)`);

      let last = performance.now();
      const frame = (now) => {
        const dt = Math.min(50, now - last);
        last = now;
        try {
          runtime.eval(`(scenes.${sceneId}/tick ${dt} ${camera.zoom} ${camera.panX} ${camera.panY})`);
        } catch (err) {
          status('error', String(err));
          console.error('[scene-runner] tick error', err);
          return;
        }
        requestAnimationFrame(frame);
      };
      requestAnimationFrame(frame);
    } catch (err) {
      status('error', String(err));
      console.error('[scene-runner] load error', err);
    }
  })();
})();

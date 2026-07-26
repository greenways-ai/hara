// Parent-page scene manager for the HARA homepage scene gallery.
// Loads `scenes/manifest.json`, owns the `#hara-scene-mount` container,
// and swaps same-origin iframes running `scene-runner.html`.

const MANIFEST_URL = 'scenes/manifest.json';
const RUNNER_URL = 'scene-runner.html';

function createMount() {
  let mount = document.getElementById('hara-scene-mount');
  if (!mount) {
    mount = document.createElement('div');
    mount.id = 'hara-scene-mount';
    mount.style.width = '100%';
    mount.style.height = '100%';
    mount.style.position = 'relative';
    document.body.appendChild(mount);
  }
  return mount;
}

function postCamera(iframe, camera) {
  if (iframe && iframe.contentWindow) {
    iframe.contentWindow.postMessage({ type: 'hara:camera', camera }, '*');
  }
}

async function initManager() {
  const mount = createMount();

  let manifest = null;
  try {
    const response = await fetch(new URL(MANIFEST_URL, document.baseURI));
    if (!response.ok) throw new Error(`HTTP ${response.status}`);
    manifest = await response.json();
  } catch (err) {
    console.error('[hara scene-manager] failed to load manifest:', err);
  }

  let iframe = null;
  const camera = { zoom: 1, panX: 0, panY: 0 };

  const manager = {
    manifest,
    camera,
    currentSceneId: null,

    load(sceneId) {
      if (iframe) {
        iframe.remove();
      }
      iframe = document.createElement('iframe');
      iframe.style.width = '100%';
      iframe.style.height = '100%';
      iframe.style.border = 'none';
      iframe.style.display = 'block';
      iframe.src = new URL(
        `${RUNNER_URL}?scene=${encodeURIComponent(sceneId)}`,
        document.baseURI,
      ).href;
      iframe.addEventListener('load', () => {
        postCamera(iframe, camera);
      });
      mount.appendChild(iframe);
      this.currentSceneId = sceneId;
      return this;
    },

    setZoom(z) {
      camera.zoom = Math.min(4, Math.max(0.25, z));
      postCamera(iframe, camera);
      return this;
    },

    pan(dx, dy) {
      camera.panX += dx;
      camera.panY += dy;
      postCamera(iframe, camera);
      return this;
    },

    zoomBy(factor) {
      return this.setZoom(camera.zoom * factor);
    },
  };

  window.addEventListener('message', (event) => {
    if (!event.data || event.data.type !== 'hara:scene-status') return;
    if (event.source !== iframe?.contentWindow) return;
    document.dispatchEvent(new CustomEvent('hara:scene-status', { detail: event.data }));
  });

  window.haraSceneManager = manager;

  if (manifest && manifest.scenes && manifest.scenes.length > 0) {
    manager.load(manifest.scenes[0].id);
  }

  return manager;
}

export default initManager();

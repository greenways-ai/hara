const query = (selector, root = document) => root.querySelector(selector);
const queryAll = (selector, root = document) => [...root.querySelectorAll(selector)];

const stylesheet = document.createElement("link");
stylesheet.rel = "stylesheet";
stylesheet.href = "./story.css?v=kernel-greenways-1";
document.head.append(stylesheet);

const story = document.createElement("div");
story.className = "kernel-story";
story.dataset.kernelStory = "";
story.hidden = true;
story.setAttribute("aria-hidden", "true");
story.innerHTML = `
  <section class="story-screen" data-story-screen="1" aria-labelledby="kernel-story-title">
    <article class="story-copy">
      <p class="story-step">02 // THE HARA KERNEL</p>
      <h2 id="kernel-story-title">One program.<br>Every medium.</h2>
      <p>
        The Hara kernel sits inside an isolated browser worker. HAL owns the program,
        state and transformations; browser adapters provide audio, video, graphics,
        input and recording. HTA messages cross the boundary as ordinary values.
      </p>
      <div class="story-principle">
        A unified representation means the same content value can be inspected,
        transformed, versioned, generated with an agent, played live and published.
      </div>
      <div class="story-capabilities" aria-label="Browser media examples">
        <div class="story-capability"><strong>AUDIO</strong><span>Web Audio graphs, synthesis, MIDI and FFT analysis.</span></div>
        <div class="story-capability"><strong>VIDEO</strong><span>Clips, camera streams, timing and frame-driven effects.</span></div>
        <div class="story-capability"><strong>VISUAL</strong><span>Canvas 2D, WebGL shaders, 3D entities and rigs.</span></div>
        <div class="story-capability"><strong>OUTPUT</strong><span>MediaRecorder capture, workspace bundles and publishing.</span></div>
      </div>
      <pre class="story-code" aria-label="Unified creative scene example">{:creative/version 1
 :background "#020408"
 :entities [{:id "mesh/hero"
             :mesh {:primitive :box}
             :material {:color "#41f5e4"}}]
 :audio {:tempo 120 :midi true :voices []}
 :video {:src "/media/clip.webm" :muted true}}</pre>
      <div class="story-actions">
        <button type="button" class="story-primary" data-story-code>OPEN THE LIVE .HAL</button>
        <a href="https://docs.hara-lang.org/" target="_blank" rel="noopener noreferrer">READ THE MODEL</a>
      </div>
    </article>
  </section>

  <section class="story-screen" data-story-screen="2" aria-labelledby="greenways-story-title">
    <article class="story-copy">
      <p class="story-step">03 // GREENWAYS OS</p>
      <h2 id="greenways-story-title">Create where the work lives.</h2>
      <p>
        Greenways OS is the Chrome layer around Hara: open a creative workspace over
        the current page, connect to its kernel when the page supports Hara, inspect
        the live system, and publish the resulting project with identity and provenance.
      </p>
      <div class="story-principle">
        The extension is not another website. It turns the browser tab itself into a
        kernel-aware creative and debugging surface.
      </div>
      <div class="story-features">
        <div class="story-feature"><strong>CREATE IN CONTEXT</strong><span>Use the page, its media and its data as the working material.</span></div>
        <div class="story-feature"><strong>DEBUG LIVE KERNELS</strong><span>Inspect sessions, capabilities, HTA messages, frames and errors.</span></div>
        <div class="story-feature"><strong>PUBLISH A WORKSPACE</strong><span>Bundle source, manifests, assets and runtime intent as one project.</span></div>
        <div class="story-feature"><strong>KEEP PROVENANCE</strong><span>Attach versions, authorship and Greenways identity to the published work.</span></div>
      </div>
      <div class="story-flow" aria-label="Greenways OS workflow">
        <strong>CHROME EXTENSION</strong><span>→</span>
        <strong>PAGE KERNEL</strong><span>→</span>
        <strong>HARA WORKSPACE</strong><span>→</span>
        <strong>GREENWAYS PUBLISH</strong>
      </div>
      <div class="story-actions">
        <button type="button" class="story-primary" data-story-debug>DEBUG THIS PAGE</button>
        <button type="button" data-story-create>CREATE A WORKSPACE</button>
        <button type="button" data-story-code>OPEN THE LIVE .HAL</button>
      </div>
    </article>
  </section>`;

document.body.append(story);

const start = query("[data-start]");
const previous = query("[data-workspace-prev]");
const next = query("[data-workspace-next]");
const backgroundSelect = query("[data-background-source]");
let activeScreen = 0;
let returnBackground = null;

const storyBackgrounds = {
  1: "document/background/kernel-media",
  2: "document/background/greenways-os"
};

function runtimeReady() {
  return document.body.dataset.kernel === "live";
}

function selectBackground(documentId) {
  if (!backgroundSelect || !documentId) return false;
  const available = [...backgroundSelect.options].some((option) => option.value === documentId);
  if (!available) return false;
  backgroundSelect.value = documentId;
  backgroundSelect.dispatchEvent(new Event("change", { bubbles: true }));
  return true;
}

function updateNavigation() {
  if (activeScreen === 0) {
    previous.disabled = true;
    next.disabled = !runtimeReady();
    start.disabled = !runtimeReady();
    return;
  }
  previous.disabled = false;
  next.disabled = activeScreen >= 2;
  start.disabled = false;
}

function showScreen(index) {
  activeScreen = Math.max(0, Math.min(2, index));
  const open = activeScreen > 0;
  story.hidden = !open;
  story.setAttribute("aria-hidden", String(!open));

  if (!open) {
    delete document.body.dataset.storyScreen;
    queryAll("[data-story-screen]", story).forEach((screen) => screen.classList.remove("is-active"));
    if (returnBackground) selectBackground(returnBackground);
    updateNavigation();
    start?.focus();
    return;
  }

  if (!returnBackground) returnBackground = backgroundSelect?.value || null;
  document.body.dataset.storyScreen = String(activeScreen);
  queryAll("[data-story-screen]", story).forEach((screen) => {
    screen.classList.toggle("is-active", Number(screen.dataset.storyScreen) === activeScreen);
  });
  selectBackground(storyBackgrounds[activeScreen]);
  updateNavigation();
  query(".story-screen.is-active .story-copy", story)?.focus({ preventScroll: true });
}

start.addEventListener("click", (event) => {
  if (!runtimeReady()) return;
  event.preventDefault();
  event.stopImmediatePropagation();
  showScreen(1);
}, true);

previous.addEventListener("click", (event) => {
  if (!activeScreen) return;
  event.preventDefault();
  event.stopImmediatePropagation();
  showScreen(activeScreen - 1);
}, true);

next.addEventListener("click", (event) => {
  if (!activeScreen && runtimeReady()) {
    event.preventDefault();
    event.stopImmediatePropagation();
    showScreen(1);
    return;
  }
  if (activeScreen === 1) {
    event.preventDefault();
    event.stopImmediatePropagation();
    showScreen(2);
  }
}, true);

story.addEventListener("click", (event) => {
  if (event.target.closest("[data-story-code]")) {
    query("[data-source-toggle]")?.click();
    return;
  }
  if (event.target.closest("[data-story-debug]")) {
    query("[data-runtime-toggle]")?.click();
    return;
  }
  if (event.target.closest("[data-story-create]")) {
    query("[data-new-workspace]")?.click();
  }
});

document.addEventListener("keydown", (event) => {
  if (!activeScreen || event.defaultPrevented || event.metaKey || event.ctrlKey || event.altKey) return;
  if (event.key === "ArrowLeft") {
    event.preventDefault();
    showScreen(activeScreen - 1);
  } else if (event.key === "ArrowRight" && activeScreen < 2) {
    event.preventDefault();
    showScreen(activeScreen + 1);
  } else if (event.key === "Escape") {
    event.preventDefault();
    showScreen(0);
  }
}, true);

new MutationObserver(() => {
  if (document.body.dataset.workspace === "1" && activeScreen) {
    activeScreen = 0;
    story.hidden = true;
    story.setAttribute("aria-hidden", "true");
    delete document.body.dataset.storyScreen;
  }
  updateNavigation();
}).observe(document.body, {
  attributes: true,
  attributeFilter: ["data-kernel", "data-workspace"]
});

updateNavigation();

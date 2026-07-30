import { HaraAmpRuntime } from "./amp-runtime.js";
import { ampLineDecorations, completionOptions, completionPrefix, patchAmpParameter, projectAmpSource } from "./amp-source-model.js";
import { createNodeGraph } from "./amp-node-graph.js";

const $ = (selector, root = document) => root.querySelector(selector);
const $$ = (selector, root = document) => [...root.querySelectorAll(selector)];
const NOTES = ["C", "C♯", "D", "D♯", "E", "F", "F♯", "G", "G♯", "A", "A♯", "B"];
const RECIPES = [
  ["STATUS", '(sonic/status "hara-amp")'],
  ["PLAY", '(sonic/update "hara-amp" "transport" "playing" true)'],
  ["PAUSE", '(sonic/update "hara-amp" "transport" "playing" false)'],
  ["TEMPO", '(sonic/update "hara-amp" "transport" "tempo" 96)'],
  ["SEQUENCE", '(sonic/update "hara-amp" "sequence" "steps" [0 7 12 7 nil 15])'],
  ["VISUAL", '(sonic/update "hara-amp" "visualizer" "mode" "scope")']
];

const sheet = document.createElement("link");
sheet.rel = "stylesheet";
sheet.href = "./story.css?v=amp-editor-1";
document.head.append(sheet);

const story = document.createElement("div");
story.className = "kernel-story";
story.dataset.kernelStory = "";
story.hidden = true;
story.innerHTML = `
<section class="story-screen" data-story-screen="1" aria-labelledby="instrument-title">
 <div class="story-layout story-instrument">
  <header class="story-heading"><p class="story-step">02 // PLAY THE PROGRAM</p>
   <h2 id="instrument-title">Make a signal.<br>Make it yours.</h2>
   <p>The silent probe starts automatically. Press Play for sound, then tap or drag notes to rewrite the live sequence.</p>
   <output class="story-runtime-state" data-amp-runtime-state aria-live="polite">WAITING FOR RUNTIME</output>
  </header>
  <section class="story-amp story-amp-first" aria-label="Playable Hara Amp">
   <button type="button" class="story-hero-play" data-story-play aria-pressed="false"><i>▶</i><span>PLAY HARA AMP</span></button>
   <div class="story-visual"><canvas data-story-visualizer aria-label="Live Hara Amp visualizer"></canvas>
    <img src="./assets/hara-amp/hara-amp-artwork-original.png" alt="">
    <div class="story-no-signal" data-story-no-signal><strong>SILENT PROBE LIVE</strong><span>PRESS PLAY TO AUTHORIZE AUDIO</span></div>
    <output data-story-frame-status>HAL · PROBE</output></div>
   <div class="story-quick-controls" data-story-controls></div>
   <section class="story-sequencer"><header><div><strong>NOTE SEQUENCE</strong><span>NAME · OFFSET FROM ROOT</span></div>
    <button type="button" data-step-rest>SELECT REST</button></header>
    <div class="story-step-slots" data-step-slots aria-label="Editable note sequence"></div>
    <div class="story-note-bank" data-note-bank aria-label="Two octave note palette"></div>
    <p>Tap a note, then a slot. Drag to replace or insert. Long-press a slot on touch to reorder.</p>
   </section>
   <dl class="story-telemetry"><div><dt>FFT → HTA</dt><dd data-story-emitted>0000</dd></div>
    <div><dt>HAL → CANVAS</dt><dd data-story-rendered>0000</dd></div>
    <div><dt>QUEUE</dt><dd data-story-queue>0 / LATEST</dd></div>
    <div><dt>AUDIO</dt><dd data-story-audio>GESTURE REQUIRED</dd></div></dl>
  </section>
  <footer class="story-next-copy"><strong>Build View ↔ Stage View</strong>The two screens are continuous views of the same running Amp.</footer>
 </div>
</section>
<section class="story-screen" data-story-screen="2" aria-labelledby="editor-title">
 <div class="story-layout story-editor">
  <header class="story-heading"><p class="story-step">03 // EDIT THE SYSTEM</p>
   <h2 id="editor-title">A player is<br>a program.</h2>
   <p>Every component below is real. Node and Text are synchronized views of <code>src/amp.hal</code>.</p>
  </header>
  <section class="story-program">
   <header class="story-program-bar"><div role="tablist">
    <button type="button" data-program-view="node" role="tab" aria-selected="true">NODE VIEW</button>
    <button type="button" data-program-view="text" role="tab" aria-selected="false">TEXT VIEW</button></div>
    <output data-story-source-status>LOADING SOURCE</output></header>
   <div class="story-node-view" data-program-panel="node"><div data-amp-node-graph></div></div>
   <div class="story-text-view" data-program-panel="text" hidden>
    <div class="story-source-editor"><pre data-source-colors aria-hidden="true"></pre>
     <textarea data-story-source spellcheck="false" wrap="off" aria-label="Editable Hara Amp source">;; Loading src/amp.hal…</textarea></div>
    <footer><span>LINES USE THE SAME COLOUR AS THEIR NODE TYPE</span><div>
     <button type="button" data-story-source-reset>RESET</button>
     <button type="button" class="story-source-apply" data-story-source-apply>APPLY + REBUILD</button></div></footer>
    <output class="story-source-error" data-story-source-error hidden></output>
   </div>
  </section>
  <section class="story-repl">
   <header><div><strong>AMP REPL · ACTIVE DOCUMENT</strong><span>ENTER TO EVAL · SHIFT+ENTER FOR A NEW LINE</span></div>
    <button type="button" data-story-repl-clear>CLEAR</button></header>
   <div class="story-repl-recipes" data-repl-recipes></div>
   <label class="story-repl-select"><span>COMMAND</span><select data-repl-recipe-select></select></label>
   <div data-story-repl-history aria-live="polite"></div>
   <form data-story-repl-form><label for="story-repl-input">HAL</label>
    <div class="story-repl-input"><textarea id="story-repl-input" data-story-repl-input spellcheck="false"
     aria-autocomplete="list" aria-controls="story-repl-completions">(sonic/status "hara-amp")</textarea>
     <div id="story-repl-completions" data-repl-completions role="listbox" hidden></div></div>
    <button type="submit">EVAL</button></form>
  </section>
  <footer class="story-closeout"><p><strong>MAKE IT YOURS.</strong>Create a workspace containing this tune and HAL file.</p>
   <div class="story-actions"><button type="button" class="story-primary" data-story-create>CREATE THIS WORKSPACE</button></div>
   <output class="story-inline-error" data-story-error hidden></output></footer>
 </div>
</section>`;
document.body.append(story);

const start = $("[data-start]"), previous = $("[data-workspace-prev]"), next = $("[data-workspace-next]");
let screen = 0, amp = null, boot = null, graphView = null, sourceModel = null;
let chosenNote = 0, activeCompletion = 0, longPress = null, preset = "hara", mode = "spectrum";
let draggedStep = null;

function ready() { return document.body.dataset.kernel === "live"; }
function navigation() {
  previous.disabled = !screen;
  next.disabled = screen ? screen >= 2 : !ready();
  start.disabled = !ready();
}
function sourceStatus(state, detail = "") {
  const output = $("[data-story-source-status]", story);
  output.textContent = state === "ready" ? `GEN ${amp?.generation ?? 0} // LIVE`
    : state === "unsaved" ? `GEN ${amp?.generation ?? 0} // LIVE · UNSAVED`
    : state === "changed" ? `GEN ${amp?.generation ?? 0} // STALE`
    : state === "error" ? `GEN ${amp?.generation ?? 0} // PREVIOUS VERSION LIVE`
    : detail.toUpperCase() || "REBUILDING";
  output.dataset.state = state;
}
function showError(message) {
  const output = $("[data-story-error]", story);
  output.hidden = false;
  output.textContent = message;
}
function createAmp() {
  return new HaraAmpRuntime({
    canvas: $("[data-story-visualizer]", story), dbName: "hara-story-amp",
    onStatus({ stage, state, detail }) {
      if (stage === "runtime") {
        const output = $("[data-amp-runtime-state]", story);
        output.textContent = state === "ready" ? "LIVE // SILENT PROBE COMPLETED"
          : state === "error" ? `UNAVAILABLE // ${detail}` : detail.toUpperCase();
        output.dataset.state = state;
      }
      if (stage === "hal") sourceStatus(state, detail);
      if (state === "error") showError(detail);
    },
    onFrame({ count }) { $("[data-story-frame-status]", story).textContent = `HAL · FRAME ${count}`; },
    onPlayback({ state }) {
      const playing = state === "playing", button = $("[data-story-play]", story);
      button.setAttribute("aria-pressed", String(playing));
      $("i", button).textContent = playing ? "Ⅱ" : "▶";
      $("span", button).textContent = playing ? "PAUSE HARA AMP" : "PLAY HARA AMP";
      $("[data-story-no-signal]", story).classList.toggle("is-hidden", playing);
      $("[data-story-audio]", story).textContent = playing ? "PLAYING / WASM" : state.toUpperCase();
    },
    onTelemetry({ emittedFrames, renderedFrames, nodeQueued }) {
      $("[data-story-emitted]", story).textContent = String(emittedFrames).padStart(4, "0");
      $("[data-story-rendered]", story).textContent = String(renderedFrames).padStart(4, "0");
      $("[data-story-queue]", story).textContent = `${nodeQueued} / LATEST`;
    },
    onGraph(snapshot) {
      renderInstrument(snapshot);
      if (!graphView) mountGraph(snapshot); else graphView.replaceModel(snapshot);
      renderCompletions();
    }
  });
}
async function ensureAmp() {
  if (!amp) amp = createAmp();
  if (!boot) boot = amp.boot().catch((error) => { showError(error.message); throw error; });
  const instance = await boot, editor = $("[data-story-source]", story);
  if (!editor.dataset.loaded) {
    editor.value = instance.source;
    editor.dataset.loaded = "true";
    renderSource();
    sourceStatus("ready");
  }
  return instance;
}
function mountGraph(snapshot) {
  graphView = createNodeGraph($("[data-amp-node-graph]", story), {
    model: snapshot,
    onSelect: (node) => revealNode(node.id),
    onParameter: ({ node, control, value }) => void updateParameter(node.id, control.parameter, value)
  });
}
function renderInstrument(snapshot) {
  const controls = $("[data-story-controls]", story);
  controls.replaceChildren();
  const visible = new Set(["transport/tempo", "source/waveform", "eq/character", "mixer/volume", "visualizer/mode"]);
  for (const node of snapshot.nodes) for (const control of node.controls ?? []) {
    if (!visible.has(`${node.id}/${control.parameter}`)) continue;
    const label = document.createElement("label");
    label.dataset.graphNode = node.id;
    label.dataset.graphParameter = control.parameter;
    label.innerHTML = `<span>${html(control.label)}</span>`;
    let input = document.createElement(control.type === "choice" ? "select" : "input");
    if (control.type === "choice") for (const choice of control.choices) {
      const option = document.createElement("option");
      option.value = typeof choice === "object" ? choice.value : choice;
      option.textContent = typeof choice === "object" ? choice.label : String(choice).toUpperCase();
      input.append(option);
    } else Object.assign(input, { type: "range", min: control.min, max: control.max, step: control.step });
    input.value = node.params[control.parameter];
    label.append(input);
    controls.append(label);
  }
  const source = snapshot.nodes.find((node) => node.id === "source");
  const sequence = snapshot.nodes.find((node) => node.id === "sequence");
  if (source && sequence) renderSequence(source.params.root, sequence.params.steps);
  preset = snapshot.nodes.find((node) => node.id === "eq")?.params.character ?? preset;
  mode = snapshot.nodes.find((node) => node.id === "visualizer")?.params.mode ?? mode;
}
function renderSequence(root, steps) {
  const slots = $("[data-step-slots]", story), bank = $("[data-note-bank]", story);
  slots.replaceChildren(...steps.map((step, index) => noteButton(step, root, index)));
  bank.replaceChildren(...Array.from({ length: 24 }, (_, offset) => noteButton(offset, root)));
}
function noteButton(offset, root, index = null) {
  const button = document.createElement("button"), rest = offset == null;
  button.type = "button";
  button.className = index == null ? "story-note" : "story-step";
  button.innerHTML = rest ? "<strong>REST</strong><small>—</small>"
    : `<strong>${noteName(root + offset)}</strong><small>${offset >= 0 ? "+" : ""}${offset}</small>`;
  button.draggable = true;
  if (index == null) {
    button.dataset.noteOffset = offset;
    button.setAttribute("aria-pressed", String(chosenNote === offset));
  } else button.dataset.stepIndex = index;
  return button;
}
function noteName(midi) { return `${NOTES[((midi % 12) + 12) % 12]}${Math.floor(midi / 12) - 1}`; }
async function updateParameter(nodeId, parameter, value) {
  const instance = await ensureAmp();
  await instance.update(nodeId, parameter, value);
  const editor = $("[data-story-source]", story);
  editor.value = patchAmpParameter(editor.value, nodeId, parameter, value);
  instance.source = editor.value;
  renderSource();
  sourceStatus("unsaved");
}
function renderSource() {
  const editor = $("[data-story-source]", story), layer = $("[data-source-colors]", story);
  try {
    sourceModel = projectAmpSource(editor.value);
    const lines = new Map();
    for (const item of ampLineDecorations(editor.value, sourceModel)) if (!lines.has(item.line)) lines.set(item.line, item);
    layer.innerHTML = editor.value.split("\n").map((line, index) => {
      const item = lines.get(index);
      return `<span data-node-id="${html(item?.nodeId ?? "")}" data-color="${item?.color ?? ""}">${html(line) || " "}</span>`;
    }).join("");
    $("[data-story-source-error]", story).hidden = true;
  } catch (error) {
    sourceModel = null;
    layer.textContent = editor.value;
    const output = $("[data-story-source-error]", story);
    output.hidden = false;
    output.textContent = error.message;
  }
}
function revealNode(id) {
  const range = sourceModel?.nodes.find((node) => node.id === id)?.range;
  if (!range) return;
  $("[data-story-source]", story).setSelectionRange(range.start, range.end);
  $$("[data-source-colors] span", story).forEach((line) => line.classList.toggle("is-selected", line.dataset.nodeId === id));
}
async function rebuildSource(reset = false) {
  const editor = $("[data-story-source]", story), output = $("[data-story-source-error]", story);
  output.hidden = true;
  try {
    const instance = await ensureAmp();
    if (reset) editor.value = instance.originalSource;
    projectAmpSource(editor.value);
    await instance.rebuild(editor.value);
    renderSource();
    sourceStatus("ready");
  } catch (error) {
    sourceStatus("error");
    output.hidden = false;
    output.textContent = error.message;
  }
}
function setView(view) {
  $$("[data-program-view]", story).forEach((button) => button.setAttribute("aria-selected", String(button.dataset.programView === view)));
  $$("[data-program-panel]", story).forEach((panel) => { panel.hidden = panel.dataset.programPanel !== view; });
}
function renderRecipes() {
  const chips = $("[data-repl-recipes]", story), select = $("[data-repl-recipe-select]", story);
  for (const [label, form] of RECIPES) {
    const button = document.createElement("button"), option = document.createElement("option");
    button.type = "button"; button.textContent = label; button.dataset.replRecipe = form; chips.append(button);
    option.value = form; option.textContent = label; select.append(option);
  }
}
function renderCompletions(force = false) {
  const input = $("[data-story-repl-input]", story), list = $("[data-repl-completions]", story);
  const prefix = completionPrefix(input.value, input.selectionStart);
  const options = completionOptions({ graph: amp?.graphSnapshot, prefix: force ? "" : prefix.value });
  activeCompletion = Math.min(activeCompletion, Math.max(0, options.length - 1));
  list.replaceChildren(...options.map((option, index) => {
    const button = document.createElement("button");
    button.type = "button"; button.role = "option"; button.dataset.completion = option.insert;
    button.dataset.completionStart = prefix.start;
    button.setAttribute("aria-selected", String(index === activeCompletion));
    button.innerHTML = `<strong>${html(option.label)}</strong><small>${html(option.kind)}</small>`;
    return button;
  }));
  list.hidden = !options.length || (!force && prefix.value.length < 2);
}
function acceptCompletion(button = $$("[data-completion]", story)[activeCompletion]) {
  if (!button) return;
  const input = $("[data-story-repl-input]", story), startAt = Number(button.dataset.completionStart);
  input.value = input.value.slice(0, startAt) + button.dataset.completion + input.value.slice(input.selectionStart);
  const caret = startAt + button.dataset.completion.length;
  input.setSelectionRange(caret, caret);
  $("[data-repl-completions]", story).hidden = true;
  input.focus();
}
async function evalRepl() {
  const input = $("[data-story-repl-input]", story), form = input.value.trim();
  if (!form) return;
  const entry = document.createElement("div");
  entry.innerHTML = `<code>${html(form)}</code><output>…</output>`;
  $("[data-story-repl-history]", story).append(entry);
  try {
    $("output", entry).textContent = renderValue(await ensureAmp().then((instance) => instance.eval(form)));
    entry.dataset.state = "ready";
  } catch (error) { $("output", entry).textContent = error.message; entry.dataset.state = "error"; }
}
function steps() { return amp?.graphSnapshot?.nodes.find((node) => node.id === "sequence")?.params.steps ?? []; }
function setSteps(value) { if (value.length && value.length <= 64) void updateParameter("sequence", "steps", value); }
function clearDropPreview() {
  const slots = $("[data-step-slots]", story);
  slots.classList.remove("is-drag-active");
  $$("[data-step-index]", slots).forEach((step) => {
    step.classList.remove("is-dragging", "is-drop-target", "shift-left", "shift-right");
    delete step.dataset.dropEdge;
  });
}
function previewSequenceDrop(target, clientX, sourceIndex = draggedStep) {
  if (!target) return clearDropPreview();
  const slots = $("[data-step-slots]", story), targetIndex = Number(target.dataset.stepIndex);
  const box = target.getBoundingClientRect();
  const edge = clientX < box.left + box.width * .35 ? "before"
    : clientX > box.right - box.width * .35 ? "after" : "replace";
  slots.classList.add("is-drag-active");
  $$("[data-step-index]", slots).forEach((step) => {
    const index = Number(step.dataset.stepIndex);
    step.classList.toggle("is-drop-target", index === targetIndex);
    step.classList.toggle("shift-left", sourceIndex != null && targetIndex < sourceIndex &&
      index >= targetIndex && index < sourceIndex);
    step.classList.toggle("shift-right", sourceIndex != null && targetIndex > sourceIndex &&
      index <= targetIndex && index > sourceIndex);
    if (index === targetIndex) step.dataset.dropEdge = edge;
    else delete step.dataset.dropEdge;
  });
  return edge;
}
function commitSequenceDrop(value, target) {
  target.classList.add("is-dropping");
  clearDropPreview();
  setTimeout(() => {
    target.classList.remove("is-dropping");
    setSteps(value);
  }, 150);
}
function show(index) {
  screen = Math.max(0, Math.min(2, index));
  story.hidden = !screen;
  if (!screen) {
    delete document.body.dataset.storyScreen;
    $$("[data-story-screen]", story).forEach((panel) => panel.classList.remove("is-active"));
    void dispose();
  } else {
    document.body.dataset.storyScreen = screen;
    $$("[data-story-screen]", story).forEach((panel) => panel.classList.toggle("is-active", Number(panel.dataset.storyScreen) === screen));
    void ensureAmp().catch(() => {});
  }
  navigation();
}
async function dispose() {
  const current = amp;
  amp = null; boot = null; graphView?.destroy(); graphView = null;
  if (current) await current.dispose();
}

start.addEventListener("click", (event) => { if (ready()) { event.preventDefault(); event.stopImmediatePropagation(); show(1); } }, true);
previous.addEventListener("click", (event) => { if (screen) { event.preventDefault(); event.stopImmediatePropagation(); show(screen - 1); } }, true);
next.addEventListener("click", (event) => { if ((!screen && ready()) || screen === 1) { event.preventDefault(); event.stopImmediatePropagation(); show(screen + 1); } }, true);
story.addEventListener("click", (event) => {
  const view = event.target.closest("[data-program-view]");
  if (view) return setView(view.dataset.programView);
  const recipe = event.target.closest("[data-repl-recipe]");
  if (recipe) { $("[data-story-repl-input]", story).value = recipe.dataset.replRecipe; return; }
  const completion = event.target.closest("[data-completion]");
  if (completion) return acceptCompletion(completion);
  const note = event.target.closest("[data-note-offset]");
  if (note) { chosenNote = Number(note.dataset.noteOffset); return renderInstrument(amp.graphSnapshot); }
  const slot = event.target.closest("[data-step-index]");
  if (slot) { const nextSteps = [...steps()]; nextSteps[Number(slot.dataset.stepIndex)] = chosenNote; return setSteps(nextSteps); }
  if (event.target.closest("[data-step-rest]")) chosenNote = null;
  if (event.target.closest("[data-story-play]")) {
    const playing = amp?.audio?.playing;
    const action = playing ? Promise.resolve(amp.pause()) : ensureAmp().then((instance) => instance.play());
    action.then(() => updateParameter("transport", "playing", !playing)).catch((error) => showError(error.message));
  }
  if (event.target.closest("[data-story-source-apply]")) void rebuildSource();
  if (event.target.closest("[data-story-source-reset]")) void rebuildSource(true);
  if (event.target.closest("[data-story-repl-clear]")) $("[data-story-repl-history]", story).replaceChildren();
  if (event.target.closest("[data-story-create]")) document.dispatchEvent(new CustomEvent("hara:create-amp-workspace", {
    detail: { preset, mode, source: $("[data-story-source]", story).value }
  }));
});
$("[data-story-controls]", story).addEventListener("change", (event) => {
  const label = event.target.closest("[data-graph-node]");
  if (label) void updateParameter(label.dataset.graphNode, label.dataset.graphParameter,
    event.target.type === "range" ? Number(event.target.value) : event.target.value);
});
story.addEventListener("dragstart", (event) => {
  const note = event.target.closest("[data-note-offset]"), step = event.target.closest("[data-step-index]");
  if (note) event.dataTransfer.setData("application/x-hara-note", note.dataset.noteOffset);
  if (step) {
    draggedStep = Number(step.dataset.stepIndex);
    step.classList.add("is-dragging");
    event.dataTransfer.setData("application/x-hara-step", step.dataset.stepIndex);
  }
});
story.addEventListener("dragend", () => { draggedStep = null; clearDropPreview(); });
$("[data-step-slots]", story).addEventListener("dragleave", (event) => {
  if (!event.currentTarget.contains(event.relatedTarget)) clearDropPreview();
});
$("[data-step-slots]", story).addEventListener("dragover", (event) => {
  event.preventDefault();
  previewSequenceDrop(event.target.closest("[data-step-index]"), event.clientX);
});
$("[data-step-slots]", story).addEventListener("drop", (event) => {
  event.preventDefault();
  const target = event.target.closest("[data-step-index]");
  if (!target) return;
  const value = [...steps()], targetIndex = Number(target.dataset.stepIndex);
  const sourceIndex = event.dataTransfer.getData("application/x-hara-step"), note = event.dataTransfer.getData("application/x-hara-note");
  if (sourceIndex !== "") { const [moved] = value.splice(Number(sourceIndex), 1); value.splice(targetIndex, 0, moved); }
  else if (note !== "") {
    const box = target.getBoundingClientRect(), offset = Number(note);
    if (event.clientX < box.left + box.width / 4) value.splice(targetIndex, 0, offset);
    else if (event.clientX > box.right - box.width / 4) value.splice(targetIndex + 1, 0, offset);
    else value[targetIndex] = offset;
  }
  draggedStep = null;
  commitSequenceDrop(value, target);
});
$("[data-step-slots]", story).addEventListener("pointerdown", (event) => {
  const step = event.target.closest("[data-step-index]");
  if (!step || event.pointerType === "mouse") return;
  longPress = setTimeout(() => { step.classList.add("is-dragging"); longPress = { index: Number(step.dataset.stepIndex) }; }, 450);
});
$("[data-step-slots]", story).addEventListener("pointermove", (event) => {
  if (!longPress || typeof longPress !== "object") return;
  previewSequenceDrop(document.elementFromPoint(event.clientX, event.clientY)?.closest("[data-step-index]"),
    event.clientX, longPress.index);
});
$("[data-step-slots]", story).addEventListener("pointerup", (event) => {
  if (typeof longPress === "number") clearTimeout(longPress);
  if (longPress && typeof longPress === "object") {
    const target = document.elementFromPoint(event.clientX, event.clientY)?.closest("[data-step-index]");
    if (target) {
      const value = [...steps()], [moved] = value.splice(longPress.index, 1);
      value.splice(Number(target.dataset.stepIndex), 0, moved);
      commitSequenceDrop(value, target);
    } else clearDropPreview();
  }
  longPress = null;
});
$("[data-story-source]", story).addEventListener("input", () => { renderSource(); sourceStatus("changed"); });
$("[data-story-source]", story).addEventListener("scroll", (event) => {
  const layer = $("[data-source-colors]", story); layer.scrollTop = event.target.scrollTop; layer.scrollLeft = event.target.scrollLeft;
});
$("[data-story-source]", story).addEventListener("keyup", (event) => {
  const node = sourceModel?.nodes.find(({ range }) => event.target.selectionStart >= range.start && event.target.selectionStart <= range.end);
  if (node) { graphView?.select(node.id, { notify: false }); revealNode(node.id); }
});
$("[data-story-source]", story).addEventListener("keydown", (event) => {
  if ((event.metaKey || event.ctrlKey) && event.key === "Enter") { event.preventDefault(); void rebuildSource(); }
});
$("[data-repl-recipe-select]", story).addEventListener("change", (event) => { $("[data-story-repl-input]", story).value = event.target.value; });
$("[data-story-repl-form]", story).addEventListener("submit", (event) => { event.preventDefault(); void evalRepl(); });
$("[data-story-repl-input]", story).addEventListener("input", () => renderCompletions());
$("[data-story-repl-input]", story).addEventListener("keydown", (event) => {
  const list = $("[data-repl-completions]", story);
  if (event.ctrlKey && event.code === "Space") { event.preventDefault(); renderCompletions(true); }
  else if (!list.hidden && ["ArrowDown", "ArrowUp"].includes(event.key)) {
    event.preventDefault();
    const count = $$("[data-completion]", list).length;
    activeCompletion = (activeCompletion + (event.key === "ArrowDown" ? 1 : -1) + count) % count;
    renderCompletions(true);
  } else if (!list.hidden && ["Tab", "Enter"].includes(event.key)) { event.preventDefault(); acceptCompletion(); }
  else if (event.key === "Escape") list.hidden = true;
  else if (event.key === "Enter" && !event.shiftKey) { event.preventDefault(); void evalRepl(); }
});
document.addEventListener("hara:amp-workspace-error", (event) => showError(event.detail?.message ?? "Workspace failed"));
new MutationObserver(() => {
  if (document.body.dataset.workspace === "1" && screen) show(0);
  else if (!screen && ready() && new URLSearchParams(location.search).get("amp") === "editor") show(2);
  navigation();
})
  .observe(document.body, { attributes: true, attributeFilter: ["data-kernel", "data-workspace"] });

renderRecipes();
navigation();

function html(value) { return String(value ?? "").replace(/[&<>"']/g, (c) => ({ "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;", "'": "&#39;" })[c]); }
function renderValue(value) { return typeof value === "string" ? value : JSON.stringify(plain(value)) ?? String(value); }
function plain(value) {
  if (value instanceof Map) return Object.fromEntries([...value].map(([key, item]) => [String(key?.name ?? key), plain(item)]));
  if (Array.isArray(value)) return value.map(plain);
  return value;
}

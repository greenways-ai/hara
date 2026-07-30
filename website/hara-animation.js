import { AnimationRuntime } from "./animation-runtime.js";
import { patchAnimationActions, patchAnimationCharacter, projectAnimationSource } from "./animation-source-model.js";

const $ = (selector) => document.querySelector(selector);
const CHARACTERS = [
  ["robot", "R-01", "CYAN MECHANICAL RIG"],
  ["mage", "MAGE", "VIOLET CLOTH RIG"],
  ["fox", "FOX", "AMBER CREATURE RIG"]
];
const ACTIONS = ["walk", "wave", "jump", "spin", "bow"];
let original = "";
let model = null;
const runtime = new AnimationRuntime($("[data-stage]"), {
  onFrame({ frame, action }) {
    $("[data-current-action]").textContent = action.toUpperCase();
    $("[data-frame]").textContent = `FRAME ${String(frame).padStart(4, "0")}`;
  }
});

function setView(view) {
  const next = view === "build" ? "build" : "stage";
  document.body.dataset.view = next;
  document.querySelectorAll("[data-view-stage]").forEach((button) =>
    button.setAttribute("aria-pressed", String(next === "stage")));
  document.querySelectorAll("[data-view-build]").forEach((button) =>
    button.setAttribute("aria-pressed", String(next === "build")));
}

async function boot() {
  const response = await fetch("./examples/hara-animation/src/animation.hal");
  if (!response.ok) throw new Error(`Animation source: ${response.status}`);
  original = await response.text();
  $("[data-source]").value = original;
  applySource();
}

function applySource() {
  try {
    model = projectAnimationSource($("[data-source]").value);
    runtime.setPlan(model);
    render();
    state("SYNCED");
  } catch (error) {
    state(error.message, true);
  }
}

function render() {
  const cast = $("[data-characters]");
  cast.replaceChildren(...CHARACTERS.map(([id, name, detail]) => {
    const button = document.createElement("button");
    button.className = id === model.selected ? "active" : "";
    button.innerHTML = `<strong>${name}</strong><small>${detail}</small>`;
    button.addEventListener("click", () => {
      $("[data-source]").value = patchAnimationCharacter($("[data-source]").value, id);
      applySource();
    });
    return button;
  }));
  const timeline = $("[data-timeline]");
  timeline.replaceChildren(...model.actions.map((action, index) => {
    const item = document.createElement("li");
    item.innerHTML = `<b>${String(index + 1).padStart(2, "0")}</b><span>${action.toUpperCase()}</span><button aria-label="Remove ${action}">×</button>`;
    item.querySelector("button").addEventListener("click", () => updateActions(model.actions.filter((_, candidate) => candidate !== index)));
    return item;
  }));
}

function updateActions(actions) {
  if (!actions.length) { state("THE PIPELINE NEEDS AT LEAST ONE ACTION", true); return; }
  $("[data-source]").value = patchAnimationActions($("[data-source]").value, actions);
  applySource();
}

for (const action of ACTIONS) {
  const button = document.createElement("button");
  button.textContent = `+ ${action.toUpperCase()}`;
  button.addEventListener("click", () => updateActions([...model.actions, action]));
  $("[data-action-bank]").append(button);
}
$("[data-play]").addEventListener("click", () => runtime.play());
$("[data-pause]").addEventListener("click", () => runtime.pause());
$("[data-apply]").addEventListener("click", applySource);
$("[data-reset]").addEventListener("click", () => { $("[data-source]").value = original; applySource(); });
$("[data-clear]").addEventListener("click", () => updateActions(["idle"]));
$("[data-source]").addEventListener("input", () => state("SOURCE CHANGED · APPLY TO REBUILD"));
document.querySelectorAll("[data-view-stage]").forEach((button) =>
  button.addEventListener("click", () => setView("stage")));
document.querySelectorAll("[data-view-build]").forEach((button) =>
  button.addEventListener("click", () => setView("build")));
document.addEventListener("keydown", (event) => {
  if (event.target.matches("textarea, input, select")) return;
  if (event.key === "ArrowLeft") setView("stage");
  if (event.key === "ArrowRight") setView("build");
});
setView(new URLSearchParams(location.search).get("view"));

function state(message, error = false) {
  $("[data-source-state]").textContent = message.toUpperCase();
  $("[data-source-state]").classList.toggle("error", error);
  $("[data-status]").textContent = error ? "HAL SOURCE · ERROR" : "HAL SOURCE · LIVE";
}

boot().catch((error) => state(error.message, true));

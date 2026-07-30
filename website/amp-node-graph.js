const SVG = "http://www.w3.org/2000/svg";
const COLOR_BY_TYPE = Object.freeze({
  "data/playlist": "amber",
  "control/transport": "amber",
  "data/sequence": "amber",
  "wasm/oscillator": "cyan",
  "audio/gain": "green",
  "audio/equalizer": "green",
  "audio/mixer": "green",
  "audio/output": "green",
  "analysis/fft": "violet",
  "hal/transform": "magenta",
  "host/canvas-ui": "blue"
});

export function createNodeGraph(root, {
  model = { nodes: [], connections: [] },
  onSelect = () => {},
  onParameter = () => {}
} = {}) {
  let graph = structuredClone(model);
  let selected = graph.nodes[0]?.id ?? null;
  root.classList.add("hara-amp-node-graph");
  root.innerHTML = `
    <div class="hara-amp-graph-canvas">
      <svg class="hara-cable-layer" data-graph-cables aria-hidden="true"></svg>
      <div class="hara-amp-node-layer" data-graph-nodes></div>
    </div>
    <aside class="hara-amp-inspector" data-graph-inspector aria-live="polite"></aside>`;
  const canvas = root.querySelector(".hara-amp-graph-canvas");
  const nodeLayer = root.querySelector("[data-graph-nodes]");
  const cableLayer = root.querySelector("[data-graph-cables]");
  const inspector = root.querySelector("[data-graph-inspector]");

  const select = (id, { notify = true } = {}) => {
    if (!graph.nodes.some((node) => node.id === id)) return;
    selected = id;
    render();
    if (notify) onSelect(graph.nodes.find((node) => node.id === id));
  };

  const render = () => {
    nodeLayer.replaceChildren();
    cableLayer.replaceChildren();
    graph.nodes.forEach((node, index) => {
      const card = document.createElement("button");
      card.type = "button";
      card.className = "hara-patch-node hara-amp-graph-node";
      card.dataset.nodeId = node.id;
      card.dataset.nodeColor = COLOR_BY_TYPE[node.type] ?? "blue";
      card.classList.toggle("is-selected", node.id === selected);
      card.innerHTML = `
        <header><i>${String(index + 1).padStart(2, "0")}</i>${escapeHtml(node.label ?? node.id)}</header>
        <strong>${escapeHtml(node.id)}</strong>
        <small>${escapeHtml(node.type)}</small>
        <span class="hara-port in" aria-hidden="true"></span>
        <span class="hara-port out" aria-hidden="true"></span>`;
      card.addEventListener("click", () => select(node.id));
      nodeLayer.append(card);
    });
    requestAnimationFrame(renderConnections);
    renderInspector();
  };

  const renderConnections = () => {
    cableLayer.replaceChildren();
    const canvasBox = canvas.getBoundingClientRect();
    for (const connection of graph.connections ?? []) {
      const from = nodeLayer.querySelector(`[data-node-id="${CSS.escape(connection.from[0])}"]`);
      const to = nodeLayer.querySelector(`[data-node-id="${CSS.escape(connection.to[0])}"]`);
      if (!from || !to) continue;
      const a = from.getBoundingClientRect();
      const b = to.getBoundingClientRect();
      const x1 = a.right - canvasBox.left;
      const y1 = a.top - canvasBox.top + a.height / 2;
      const x2 = b.left - canvasBox.left;
      const y2 = b.top - canvasBox.top + b.height / 2;
      const bend = Math.max(32, Math.abs(x2 - x1) / 2);
      const path = document.createElementNS(SVG, "path");
      path.setAttribute("d", `M ${x1} ${y1} C ${x1 + bend} ${y1}, ${x2 - bend} ${y2}, ${x2} ${y2}`);
      path.setAttribute("class", `hara-cable is-${connectionColor(connection.kind)}`);
      cableLayer.append(path);
    }
  };

  const renderInspector = () => {
    const node = graph.nodes.find((candidate) => candidate.id === selected);
    if (!node) {
      inspector.innerHTML = "<p>Select a component.</p>";
      return;
    }
    inspector.innerHTML = `
      <header><b>${escapeHtml(node.label)}</b><span>${escapeHtml(node.runtime || "HAL graph")}</span></header>
      <p>${escapeHtml(node.summary || "")}</p>
      <dl><div><dt>INPUT</dt><dd>${escapeHtml(node.input || "—")}</dd></div>
      <div><dt>OUTPUT</dt><dd>${escapeHtml(node.output || "—")}</dd></div></dl>
      <div data-node-controls></div>`;
    const controls = inspector.querySelector("[data-node-controls]");
    for (const control of node.controls ?? []) {
      if (control.type === "steps") continue;
      const label = document.createElement("label");
      label.innerHTML = `<span>${escapeHtml(control.label)}</span>`;
      let input;
      if (control.type === "choice") {
        input = document.createElement("select");
        for (const choice of control.choices ?? []) {
          const option = document.createElement("option");
          option.value = typeof choice === "object" ? choice.value : choice;
          option.textContent = typeof choice === "object" ? choice.label : String(choice).toUpperCase();
          input.append(option);
        }
      } else {
        input = document.createElement("input");
        input.type = control.type === "boolean" ? "checkbox" : "range";
        if (control.min != null) input.min = control.min;
        if (control.max != null) input.max = control.max;
        if (control.step != null) input.step = control.step;
      }
      if (input.type === "checkbox") input.checked = Boolean(node.params[control.parameter]);
      else input.value = node.params[control.parameter];
      input.addEventListener("change", () => {
        const value = input.type === "checkbox" ? input.checked :
          input.type === "range" ? Number(input.value) : input.value;
        onParameter({ node, control, value });
      });
      label.append(input);
      controls.append(label);
    }
  };

  const resize = new ResizeObserver(renderConnections);
  resize.observe(canvas);
  render();
  return {
    replaceModel(next) {
      graph = structuredClone(next);
      if (!graph.nodes.some((node) => node.id === selected)) selected = graph.nodes[0]?.id ?? null;
      render();
    },
    select,
    selected: () => selected,
    destroy() { resize.disconnect(); root.replaceChildren(); }
  };
}

function connectionColor(kind) {
  if (kind === "audio") return "green";
  if (kind === "analysis" || kind?.includes("latest")) return "violet";
  if (kind === "host") return "blue";
  return "amber";
}

function escapeHtml(value) {
  return String(value ?? "").replace(/[&<>"']/g, (character) => ({
    "&": "&amp;", "<": "&lt;", ">": "&gt;", '"': "&quot;", "'": "&#39;"
  })[character]);
}

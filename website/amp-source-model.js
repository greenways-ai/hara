const PAIRS = Object.freeze({ "(": ")", "[": "]", "{": "}" });
const CLOSERS = new Set(Object.values(PAIRS));

export const AMP_TYPE_COLORS = Object.freeze({
  sequence: "amber",
  transport: "amber",
  source: "cyan",
  audio: "green",
  analysis: "violet",
  hal: "magenta",
  host: "blue"
});

export function projectAmpSource(source) {
  const graph = findDefinitionValue(source, "amp-graph");
  if (!graph || source[graph.start] !== "{") throw new Error("amp-graph must be a map");
  const nodesValue = mapEntry(source, graph, "nodes");
  const connectionsValue = mapEntry(source, graph, "connections");
  if (!nodesValue || source[nodesValue.start] !== "[") throw new Error("amp-graph nodes must be a vector");
  if (!connectionsValue || source[connectionsValue.start] !== "[") {
    throw new Error("amp-graph connections must be a vector");
  }
  const nodes = directForms(source, nodesValue)
    .filter((range) => source[range.start] === "{")
    .map((range) => projectNode(source, range));
  const connections = directForms(source, connectionsValue)
    .filter((range) => source[range.start] === "{")
    .map((range, index) => projectConnection(source, range, index));
  return { graph, nodes, connections };
}

export function patchAmpParameter(source, nodeId, parameter, value) {
  const model = projectAmpSource(source);
  const node = model.nodes.find((candidate) => candidate.id === nodeId);
  const range = node?.params?.[parameter];
  if (!range) throw new Error(`Source parameter not found: ${nodeId}/${parameter}`);
  return source.slice(0, range.start) + halLiteral(value) + source.slice(range.end);
}

export function ampLineDecorations(source, model = projectAmpSource(source)) {
  const starts = lineStarts(source);
  const decorations = [];
  for (const node of model.nodes) {
    const start = lineAt(starts, node.range.start);
    const end = lineAt(starts, Math.max(node.range.start, node.range.end - 1));
    for (let line = start; line <= end; line += 1) {
      decorations.push({ line, color: node.color, nodeId: node.id, kind: "node" });
    }
  }
  for (const connection of model.connections) {
    const start = lineAt(starts, connection.range.start);
    const end = lineAt(starts, Math.max(connection.range.start, connection.range.end - 1));
    for (let line = start; line <= end; line += 1) {
      decorations.push({ line, color: connection.color, connectionId: connection.id, kind: "connection" });
    }
  }
  return decorations;
}

export function completionOptions({ graph, prefix = "" } = {}) {
  const forms = [
    { label: "Status graph", insert: '(sonic/status "hara-amp")', kind: "recipe" },
    { label: "Play transport", insert: '(sonic/update "hara-amp" "transport" "playing" true)', kind: "recipe" },
    { label: "Pause transport", insert: '(sonic/update "hara-amp" "transport" "playing" false)', kind: "recipe" },
    { label: "Set tempo", insert: '(sonic/update "hara-amp" "transport" "tempo" 120)', kind: "recipe" },
    { label: "Set sequence", insert: '(sonic/update "hara-amp" "sequence" "steps" [0 7 12 7])', kind: "recipe" },
    { label: "Set visual mode", insert: '(sonic/update "hara-amp" "visualizer" "mode" "scope")', kind: "recipe" }
  ];
  const symbols = [
    "sonic/status", "sonic/update", "sonic/start", "sonic/stop",
    "status", "update", "true", "false", "nil"
  ].map((insert) => ({ label: insert, insert, kind: "symbol" }));
  const graphValues = [];
  for (const node of graph?.nodes ?? []) {
    graphValues.push({ label: `${node.id} · node`, insert: `"${node.id}"`, kind: "node" });
    for (const control of node.controls ?? []) {
      graphValues.push({
        label: `${node.id}/${control.parameter}`,
        insert: `"${control.parameter}"`,
        kind: "parameter"
      });
      for (const choice of control.choices ?? []) {
        const value = typeof choice === "object" ? choice.value : choice;
        graphValues.push({ label: `${control.parameter}: ${value}`, insert: halLiteral(value), kind: "value" });
      }
    }
  }
  const needle = prefix.trim().toLowerCase();
  return [...forms, ...symbols, ...graphValues]
    .filter((option) => !needle || `${option.label} ${option.insert}`.toLowerCase().includes(needle))
    .slice(0, 24);
}

export function completionPrefix(source, caret = source.length) {
  let start = Math.max(0, caret);
  while (start > 0 && !/[\s()[\]{}"]/.test(source[start - 1])) start -= 1;
  return { start, end: caret, value: source.slice(start, caret) };
}

function projectNode(source, range) {
  const id = literal(source, mapEntry(source, range, "id"));
  const type = literal(source, mapEntry(source, range, "type"));
  if (typeof id !== "string" || typeof type !== "string") throw new Error("Every Amp node needs an id and type");
  const paramsRange = mapEntry(source, range, "params");
  const params = {};
  if (paramsRange && source[paramsRange.start] === "{") {
    for (const [key, value] of mapEntries(source, paramsRange)) {
      const name = literal(source, key);
      if (typeof name === "string") params[name] = value;
    }
  }
  return {
    id,
    type,
    color: nodeColor(id, type),
    range,
    params,
    label: literal(source, mapEntry(source, range, "label")) ?? id
  };
}

function projectConnection(source, range, index) {
  const kind = literal(source, mapEntry(source, range, "kind")) ?? "audio";
  return {
    id: literal(source, mapEntry(source, range, "id")) ?? `connection/${index + 1}`,
    kind,
    color: connectionColor(kind),
    range
  };
}

function nodeColor(id, type) {
  if (id === "sequence" || id === "playlist" || id === "transport") return AMP_TYPE_COLORS.sequence;
  if (type.startsWith("wasm/")) return AMP_TYPE_COLORS.source;
  if (type.startsWith("audio/")) return AMP_TYPE_COLORS.audio;
  if (type.includes("fft") || type.startsWith("analysis/")) return AMP_TYPE_COLORS.analysis;
  if (type.startsWith("hal/")) return AMP_TYPE_COLORS.hal;
  return AMP_TYPE_COLORS.host;
}

function connectionColor(kind) {
  if (kind === "audio") return AMP_TYPE_COLORS.audio;
  if (kind === "analysis" || kind.includes("latest")) return AMP_TYPE_COLORS.analysis;
  if (kind === "host") return AMP_TYPE_COLORS.host;
  return AMP_TYPE_COLORS.sequence;
}

function findDefinitionValue(source, name) {
  let cursor = 0;
  while (cursor < source.length) {
    cursor = skipTrivia(source, cursor);
    if (cursor >= source.length) break;
    const form = scanForm(source, cursor);
    if (source[cursor] === "(") {
      const parts = directForms(source, form);
      if (token(source, parts[0]) === "def" && token(source, parts[1]) === name) return parts[2] ?? null;
    }
    cursor = form.end;
  }
  return null;
}

function mapEntry(source, mapRange, name) {
  for (const [key, value] of mapEntries(source, mapRange)) {
    if (literal(source, key) === name) return value;
  }
  return null;
}

function mapEntries(source, range) {
  const forms = directForms(source, range);
  const entries = [];
  for (let index = 0; index + 1 < forms.length; index += 2) entries.push([forms[index], forms[index + 1]]);
  return entries;
}

function directForms(source, collection) {
  const forms = [];
  let cursor = skipTrivia(source, collection.start + 1);
  while (cursor < collection.end - 1) {
    const range = scanForm(source, cursor);
    forms.push(range);
    cursor = skipTrivia(source, range.end);
  }
  return forms;
}

function scanForm(source, start) {
  const first = source[start];
  if (Object.hasOwn(PAIRS, first)) return scanCollection(source, start);
  if (first === '"') return scanString(source, start);
  if (first === "'" || first === "`") return scanForm(source, skipTrivia(source, start + 1));
  let cursor = start;
  while (cursor < source.length && !/\s|,/.test(source[cursor]) &&
         !Object.hasOwn(PAIRS, source[cursor]) && !CLOSERS.has(source[cursor])) cursor += 1;
  if (cursor === start) throw new Error(`Unexpected HAL token at ${start}`);
  return { start, end: cursor };
}

function scanCollection(source, start) {
  const stack = [PAIRS[source[start]]];
  let cursor = start + 1;
  while (cursor < source.length && stack.length) {
    const character = source[cursor];
    if (character === '"') {
      cursor = scanString(source, cursor).end;
      continue;
    }
    if (character === ";") {
      while (cursor < source.length && source[cursor] !== "\n") cursor += 1;
      continue;
    }
    if (Object.hasOwn(PAIRS, character)) stack.push(PAIRS[character]);
    else if (character === stack.at(-1)) stack.pop();
    else if (CLOSERS.has(character)) throw new Error(`Mismatched HAL delimiter at ${cursor}`);
    cursor += 1;
  }
  if (stack.length) throw new Error(`Unclosed HAL collection at ${start}`);
  return { start, end: cursor };
}

function scanString(source, start) {
  let cursor = start + 1;
  while (cursor < source.length) {
    if (source[cursor] === "\\") cursor += 2;
    else if (source[cursor++] === '"') return { start, end: cursor };
  }
  throw new Error(`Unclosed HAL string at ${start}`);
}

function skipTrivia(source, start) {
  let cursor = start;
  while (cursor < source.length) {
    if (/[\s,]/.test(source[cursor])) cursor += 1;
    else if (source[cursor] === ";") {
      while (cursor < source.length && source[cursor] !== "\n") cursor += 1;
    } else break;
  }
  return cursor;
}

function literal(source, range) {
  if (!range) return undefined;
  const value = token(source, range);
  if (value.startsWith('"')) {
    try { return JSON.parse(value); } catch { return value.slice(1, -1); }
  }
  if (value === "nil") return null;
  if (value === "true") return true;
  if (value === "false") return false;
  if (/^-?\d+(?:\.\d+)?$/.test(value)) return Number(value);
  return value;
}

function token(source, range) {
  return range ? source.slice(range.start, range.end) : "";
}

function halLiteral(value) {
  if (value == null) return "nil";
  if (typeof value === "string") return JSON.stringify(value);
  if (typeof value === "number" || typeof value === "boolean") return String(value);
  if (Array.isArray(value)) return `[${value.map(halLiteral).join(" ")}]`;
  throw new Error("Unsupported HAL parameter value");
}

function lineStarts(source) {
  const starts = [0];
  for (let index = 0; index < source.length; index += 1) {
    if (source[index] === "\n") starts.push(index + 1);
  }
  return starts;
}

function lineAt(starts, offset) {
  let low = 0;
  let high = starts.length;
  while (low + 1 < high) {
    const middle = Math.floor((low + high) / 2);
    if (starts[middle] <= offset) low = middle;
    else high = middle;
  }
  return low;
}

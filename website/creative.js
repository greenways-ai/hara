import { keywordName, mapValue } from "./scene.js";

const hex = (value, fallback = "#020408") => typeof value === "string" && /^#[0-9a-f]{6}$/i.test(value) ? value : fallback;
const vector = (value, fallback) => Array.isArray(value) && value.length === fallback.length && value.every(Number.isFinite) ? value : fallback;

export function normalizeCreative(value) {
  if (!(value instanceof Map) || mapValue(value, "creative/version") !== 1) {
    throw new Error("program must return a :creative/version 1 scene");
  }
  const entities = mapValue(value, "entities");
  if (!Array.isArray(entities)) throw new Error("creative scene :entities must be a vector");
  return {
    background: hex(mapValue(value, "background")),
    entities: entities.map((entity, index) => normalizeEntity(entity, index)),
    audio: normalizeAudio(mapValue(value, "audio")),
    video: normalizeVideo(mapValue(value, "video"))
  };
}

function normalizeEntity(value, index) {
  if (!(value instanceof Map)) throw new Error(`entity ${index + 1} must be a map`);
  const mesh = mapValue(value, "mesh") ?? new Map();
  const transform = mapValue(value, "transform") ?? new Map();
  const rig = mapValue(value, "rig");
  return {
    id: String(mapValue(value, "id") ?? `entity/${index + 1}`),
    primitive: keywordName(mapValue(mesh, "primitive")) ?? "box",
    color: hex(mapValue(mapValue(value, "material") ?? new Map(), "color"), "#41f5e4"),
    position: vector(mapValue(transform, "position"), [0, 0, 0]),
    rotation: vector(mapValue(transform, "rotation"), [0, 0, 0]),
    scale: vector(mapValue(transform, "scale"), [1, 1, 1]),
    rig: normalizeRig(rig)
  };
}

function normalizeRig(value) {
  if (!(value instanceof Map)) return null;
  const bones = mapValue(value, "bones");
  if (!Array.isArray(bones)) throw new Error("rig :bones must be a vector");
  return { bones: bones.map((bone, index) => ({ id: String(mapValue(bone, "id") ?? `bone/${index}`), parent: mapValue(bone, "parent") ?? null, length: Number(mapValue(bone, "length") ?? 1) })) };
}

function normalizeAudio(value) {
  if (!(value instanceof Map)) return null;
  return { tempo: Number(mapValue(value, "tempo") ?? 120), midi: Boolean(mapValue(value, "midi")), voices: Array.isArray(mapValue(value, "voices")) ? mapValue(value, "voices") : [] };
}

function normalizeVideo(value) {
  if (!(value instanceof Map)) return null;
  const source = mapValue(value, "src");
  return typeof source === "string" ? { source, muted: mapValue(value, "muted") !== false } : null;
}

export function solveTwoBone(root, target, upperLength, lowerLength) {
  const dx = target[0] - root[0], dy = target[1] - root[1];
  const distance = Math.max(.0001, Math.min(Math.hypot(dx, dy), upperLength + lowerLength - .0001));
  const base = Math.atan2(dy, dx);
  const elbow = Math.acos((upperLength ** 2 + distance ** 2 - lowerLength ** 2) / (2 * upperLength * distance));
  return { shoulder: base - elbow, elbow: Math.PI - Math.acos((upperLength ** 2 + lowerLength ** 2 - distance ** 2) / (2 * upperLength * lowerLength)) };
}

export class CreativeRuntime {
  constructor(canvas) {
    this.canvas = canvas;
    this.gl = canvas.getContext("webgl2", { antialias: true, alpha: false });
    this.audio = null;
    this.video = null;
    if (!this.gl) throw new Error("WebGL2 is unavailable");
    this.program = program(this.gl);
    this.buffer = cube(this.gl);
  }

  render(scene) {
    const gl = this.gl;
    const rect = this.canvas.getBoundingClientRect();
    this.canvas.width = Math.max(1, Math.round(rect.width * devicePixelRatio));
    this.canvas.height = Math.max(1, Math.round(rect.height * devicePixelRatio));
    gl.viewport(0, 0, this.canvas.width, this.canvas.height);
    const bg = rgb(scene.background);
    gl.clearColor(...bg, 1); gl.clear(gl.COLOR_BUFFER_BIT | gl.DEPTH_BUFFER_BIT); gl.enable(gl.DEPTH_TEST);
    gl.useProgram(this.program); gl.bindBuffer(gl.ARRAY_BUFFER, this.buffer);
    const position = gl.getAttribLocation(this.program, "position"); gl.enableVertexAttribArray(position); gl.vertexAttribPointer(position, 3, gl.FLOAT, false, 0, 0);
    const entity = scene.entities[0];
    if (entity) {
      const colour = rgb(entity.color); gl.uniform4f(gl.getUniformLocation(this.program, "colour"), ...colour, 1);
      gl.uniform1f(gl.getUniformLocation(this.program, "angle"), performance.now() / 1000 + entity.rotation[1]);
      gl.drawArrays(gl.TRIANGLES, 0, 36);
    }
    this.playAudio(scene.audio); this.playVideo(scene.video);
  }

  playAudio(audio) {
    if (!audio || this.audio) return;
    const Context = globalThis.AudioContext ?? globalThis.webkitAudioContext; if (!Context) return;
    const context = new Context(), oscillator = context.createOscillator(), gain = context.createGain(), analyser = context.createAnalyser();
    oscillator.frequency.value = 55; gain.gain.value = .045; oscillator.connect(gain).connect(analyser).connect(context.destination); oscillator.start();
    this.audio = { context, oscillator, analyser };
    if (audio.midi && navigator.requestMIDIAccess) navigator.requestMIDIAccess().catch(() => {});
  }

  playVideo(video) {
    if (!video || this.video?.src === video.source) return;
    this.video?.remove(); const element = document.createElement("video");
    element.src = video.source; element.muted = video.muted; element.loop = true; element.playsInline = true; element.play().catch(() => {}); this.video = element;
  }

  capture() {
    if (!this.canvas.captureStream || !globalThis.MediaRecorder) throw new Error("video capture is unavailable");
    const recorder = new MediaRecorder(this.canvas.captureStream(30), { mimeType: "video/webm" }); const chunks = [];
    recorder.ondataavailable = (event) => event.data.size && chunks.push(event.data);
    const finished = new Promise((resolve) => recorder.onstop = () => resolve(new Blob(chunks, { type: "video/webm" })));
    recorder.start(); return { stop: () => recorder.stop(), finished };
  }
}

function program(gl) { const vertex = `#version 300 es\nin vec3 position; uniform float angle; void main(){float c=cos(angle),s=sin(angle);vec3 p=vec3(position.x*c-position.z*s,position.y,position.x*s+position.z*c);gl_Position=vec4(p*.58,1.);}`; const fragment = `#version 300 es\nprecision highp float; uniform vec4 colour; out vec4 outColor; void main(){outColor=colour;}`; const compile = (type, source) => { const shader = gl.createShader(type); gl.shaderSource(shader, source); gl.compileShader(shader); if (!gl.getShaderParameter(shader, gl.COMPILE_STATUS)) throw new Error(gl.getShaderInfoLog(shader)); return shader; }; const result = gl.createProgram(); gl.attachShader(result, compile(gl.VERTEX_SHADER, vertex)); gl.attachShader(result, compile(gl.FRAGMENT_SHADER, fragment)); gl.linkProgram(result); return result; }
function cube(gl) { const values = [-1,-1,1,1,-1,1,1,1,1,-1,-1,1,1,1,1,-1,1,1,-1,-1,-1,-1,1,-1,1,1,-1,-1,-1,-1,1,1,-1,1,-1,-1,-1,1,1,1,1,1,1,1,-1,-1,1,1,-1,-1,-1,-1,-1,1,-1,1,1,-1,-1,-1,1,1,-1,1,1,1,1,-1,1,-1,-1,-1,-1,-1,1,-1,1,-1,1,1,1,-1,1,1,1,-1,-1,-1,-1,1,-1,1,1,-1,1,1,1,1,-1,-1,1,-1,1,1,-1,-1,-1,-1,-1,1,-1,1,-1,-1,1,1,-1,1,1,-1,-1,1,-1,-1,-1,-1,1,-1,1,1,-1,1,1,1,1]; const buffer = gl.createBuffer(); gl.bindBuffer(gl.ARRAY_BUFFER, buffer); gl.bufferData(gl.ARRAY_BUFFER, new Float32Array(values), gl.STATIC_DRAW); return buffer; }
function rgb(value) { const integer = Number.parseInt(value.slice(1), 16); return [(integer >> 16 & 255) / 255, (integer >> 8 & 255) / 255, (integer & 255) / 255]; }

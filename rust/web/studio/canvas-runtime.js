const DEFAULT_CAPABILITIES = new Set([
  "canvas/2d",
  "canvas/webgl2",
  "input/keyboard",
  "input/pointer"
]);

export class CanvasRuntime {
  constructor({
    window: windowObject = globalThis.window,
    requestFrame = globalThis.requestAnimationFrame?.bind(globalThis),
    cancelFrame = globalThis.cancelAnimationFrame?.bind(globalThis),
    capabilities = DEFAULT_CAPABILITIES,
    onDiagnostic = () => {}
  } = {}) {
    this.window = windowObject;
    this.requestFrame = requestFrame;
    this.cancelFrame = cancelFrame;
    this.capabilities = new Set(capabilities);
    this.onDiagnostic = onDiagnostic;
    this.canvases = new Map();
    this.events = [];
    this.listeners = [];
    this.visible = true;
    this.sequence = 0;
    this.installInput();
  }

  register(canvasId, canvas) {
    if (this.canvases.has(canvasId)) throw new Error(`CANVAS_EXISTS ${canvasId}`);
    const slot = {
      id: canvasId,
      canvas,
      owner: null,
      candidate: null,
      pending: new Map(),
      firstRender: new Map(),
      lastTime: null,
      webgl: null,
      lastFrame: null
    };
    this.canvases.set(canvasId, slot);
    return () => this.unregister(canvasId);
  }

  unregister(canvasId) {
    const slot = this.canvases.get(canvasId);
    if (!slot) return false;
    this.cancelSlot(slot, "canvas closed");
    this.disposeWebGl(slot);
    this.canvases.delete(canvasId);
    return true;
  }

  claim(nodeId, canvasId) {
    const slot = this.requireCanvas(canvasId);
    if (slot.owner && slot.owner !== nodeId) this.cancelOwner(slot, slot.owner, "canvas ownership replaced");
    slot.owner = nodeId;
    slot.lastTime = null;
    return true;
  }

  stage(nodeId, canvasId) {
    const slot = this.requireCanvas(canvasId);
    if (slot.candidate && slot.candidate !== nodeId) {
      this.cancelOwner(slot, slot.candidate, "candidate generation replaced");
    }
    slot.candidate = nodeId;
    return true;
  }

  commit(nodeId, canvasId) {
    const slot = this.requireCanvas(canvasId);
    if (slot.candidate !== nodeId) throw structuredError("canvas/not-candidate", `${nodeId} is not staged`);
    if (slot.owner && slot.owner !== nodeId) this.cancelOwner(slot, slot.owner, "canvas generation replaced");
    slot.owner = nodeId;
    slot.candidate = null;
    slot.lastTime = null;
    return true;
  }

  discard(nodeId, canvasId) {
    const slot = this.requireCanvas(canvasId);
    if (slot.candidate !== nodeId) return false;
    this.cancelOwner(slot, nodeId, "candidate generation discarded");
    slot.candidate = null;
    return true;
  }

  release(nodeId, canvasId = null) {
    let released = false;
    for (const slot of this.canvases.values()) {
      if (canvasId !== null && slot.id !== canvasId) continue;
      if (slot.candidate === nodeId) {
        this.cancelOwner(slot, nodeId, "canvas candidate released");
        slot.candidate = null;
        released = true;
      }
      if (slot.owner !== nodeId) continue;
      this.cancelOwner(slot, nodeId, "canvas generation released");
      slot.owner = null;
      released = true;
    }
    return released;
  }

  setVisible(visible) {
    this.visible = Boolean(visible);
    if (this.visible) return;
    for (const slot of this.canvases.values()) this.cancelSlot(slot, "workspace hidden");
  }

  nextFrame(nodeId, canvasId) {
    const slot = this.assertAccess(nodeId, canvasId);
    if (!this.visible) return Promise.reject(structuredError("canvas/hidden", "workspace is hidden"));
    return new Promise((resolve, reject) => {
      const token = this.requestFrame((time) => {
        slot.pending.delete(token);
        if (!this.visible || (slot.owner !== nodeId && slot.candidate !== nodeId)) {
          reject(structuredError("canvas/generation-inactive", "canvas generation is no longer active"));
          return;
        }
        this.resize(slot.canvas);
        const now = Math.max(0, Math.trunc(time));
        const delta = slot.lastTime === null ? 0 : Math.max(0, now - slot.lastTime);
        slot.lastTime = now;
        const events = this.events.splice(0);
        resolve(toHta({
          "frame/id": ++this.sequence,
          "frame/time-ms": now,
          "frame/delta-ms": delta,
          "canvas/width": slot.canvas.clientWidth || slot.canvas.width,
          "canvas/height": slot.canvas.clientHeight || slot.canvas.height,
          "canvas/pixel-ratio-milli": Math.round(this.pixelRatio() * 1000),
          "input/events": events
        }));
      });
      slot.pending.set(token, reject);
    });
  }

  render(nodeId, canvasId, value) {
    const slot = this.assertAccess(nodeId, canvasId);
    const frame = plain(value);
    const backend = keyName(frame.type ?? frame["frame/type"] ?? frame["render/type"]);
    try {
      if (backend === "webgl2") this.renderWebGl(slot, frame);
      else if (backend === "canvas-2d") this.renderCanvas2d(slot, frame);
      else throw new Error(`unsupported frame type: ${backend ?? "nil"}`);
      slot.lastFrame = frame;
      this.resolveFirstRender(slot, nodeId);
      return true;
    } catch (error) {
      const fallback = frame.fallback ?? frame["frame/fallback"];
      if (fallback) {
        this.renderCanvas2d(slot, plain(fallback));
        this.onDiagnostic(structuredError("canvas/webgl-fallback", error.message));
        this.resolveFirstRender(slot, nodeId);
        return true;
      }
      const diagnostic = structuredError("canvas/render-failed", error.message);
      this.rejectFirstRender(slot, nodeId, diagnostic);
      this.onDiagnostic(diagnostic);
      throw diagnostic;
    }
  }

  waitForFirstRender(nodeId, canvasId, timeout = 2000) {
    const slot = this.assertAccess(nodeId, canvasId);
    return new Promise((resolve, reject) => {
      const timer = setTimeout(() => {
        slot.firstRender.delete(nodeId);
        reject(structuredError("canvas/first-frame-timeout", "generation did not render a frame"));
      }, timeout);
      slot.firstRender.set(nodeId, {
        resolve: () => { clearTimeout(timer); resolve(true); },
        reject: (error) => { clearTimeout(timer); reject(error); }
      });
    });
  }

  installInput() {
    if (!this.window?.addEventListener) return;
    const listen = (name, handler, options) => {
      this.window.addEventListener(name, handler, options);
      this.listeners.push(() => this.window.removeEventListener(name, handler, options));
    };
    if (this.capabilities.has("input/keyboard")) {
      listen("keydown", (event) => this.pushEvent({
        type: "key", phase: "down", key: event.key, code: event.code,
        repeat: event.repeat, modifiers: modifiers(event)
      }));
      listen("keyup", (event) => this.pushEvent({
        type: "key", phase: "up", key: event.key, code: event.code,
        repeat: false, modifiers: modifiers(event)
      }));
    }
    if (this.capabilities.has("input/pointer")) {
      let touchStart = null;
      listen("pointerdown", (event) => {
        if (event.pointerType === "touch") touchStart = point(event);
        this.pushEvent({ type: "pointer", phase: "down", ...point(event) });
      });
      listen("pointermove", (event) =>
        this.pushEvent({ type: "pointer", phase: "move", ...point(event) }));
      listen("pointerup", (event) => {
        const end = point(event);
        this.pushEvent({ type: "pointer", phase: "up", ...end });
        if (event.pointerType === "touch" && touchStart) {
          const dx = end.x - touchStart.x;
          const dy = end.y - touchStart.y;
          if (Math.max(Math.abs(dx), Math.abs(dy)) >= 24) {
            this.pushEvent({
              type: "swipe",
              direction: Math.abs(dx) > Math.abs(dy)
                ? (dx > 0 ? "right" : "left")
                : (dy > 0 ? "down" : "up")
            });
          }
          touchStart = null;
        }
      });
    }
  }

  pushEvent(event) {
    if (!this.visible || ![...this.canvases.values()].some((slot) => slot.owner)) return;
    this.events.push(toHta(event));
    if (this.events.length > 128) this.events.shift();
  }

  renderCanvas2d(slot, frame) {
    if (!this.capabilities.has("canvas/2d")) throw new Error("missing :canvas/2d capability");
    const canvas = slot.canvas;
    this.resize(canvas);
    const context = canvas.getContext("2d");
    if (!context) throw new Error("Canvas2D is unavailable");
    const ratio = this.pixelRatio();
    const width = canvas.clientWidth || canvas.width / ratio;
    const height = canvas.clientHeight || canvas.height / ratio;
    context.setTransform(ratio, 0, 0, ratio, 0, 0);
    context.globalAlpha = 1;
    context.shadowBlur = 0;
    context.clearRect(0, 0, width, height);
    context.fillStyle = frame.background ?? "#020408";
    context.fillRect(0, 0, width, height);
    for (const command of frame.commands ?? []) execute2d(context, plain(command), width, height);
  }

  renderWebGl(slot, frame) {
    if (!this.capabilities.has("canvas/webgl2")) throw new Error("missing :canvas/webgl2 capability");
    this.resize(slot.canvas);
    const width = slot.canvas.width;
    const height = slot.canvas.height;
    if (!slot.webgl) {
      const surface = this.window?.document?.createElement?.("canvas");
      if (!surface) throw new Error("WebGL surface cannot be created");
      slot.webgl = { surface, gl: surface.getContext("webgl2"), programs: new Map() };
    }
    const { surface, gl, programs } = slot.webgl;
    if (!gl) throw new Error("WebGL2 is unavailable");
    surface.width = width;
    surface.height = height;
    const vertex = frame.vertex ?? frame["shader/vertex"];
    const fragment = frame.fragment ?? frame["shader/fragment"];
    if (typeof vertex !== "string" || typeof fragment !== "string") {
      throw new Error("WebGL frame requires vertex and fragment shader sources");
    }
    const hash = `${hashText(vertex)}:${hashText(fragment)}`;
    let program = programs.get(hash);
    if (!program) {
      program = linkProgram(gl, vertex, fragment);
      programs.set(hash, program);
    }
    gl.viewport(0, 0, width, height);
    gl.useProgram(program);
    const uniforms = plain(frame.uniforms ?? {});
    for (const [name, value] of Object.entries(uniforms)) {
      const location = gl.getUniformLocation(program, name);
      if (location === null) continue;
      if (Array.isArray(value)) {
        if (value.length === 2) gl.uniform2fv(location, value);
        else if (value.length === 3) gl.uniform3fv(location, value);
        else if (value.length === 4) gl.uniform4fv(location, value);
      } else {
        gl.uniform1f(location, Number(value));
      }
    }
    gl.drawArrays(gl.TRIANGLES, 0, 3);
    const context = slot.canvas.getContext("2d");
    if (!context) throw new Error("Canvas2D compositor is unavailable");
    context.setTransform(1, 0, 0, 1, 0, 0);
    context.clearRect(0, 0, width, height);
    context.drawImage(surface, 0, 0);
  }

  resize(canvas) {
    const ratio = this.pixelRatio();
    const width = Math.max(1, Math.round((canvas.clientWidth || canvas.width || 1) * ratio));
    const height = Math.max(1, Math.round((canvas.clientHeight || canvas.height || 1) * ratio));
    if (canvas.width !== width) canvas.width = width;
    if (canvas.height !== height) canvas.height = height;
  }

  pixelRatio() {
    return Math.min(2, Math.max(1, this.window?.devicePixelRatio || 1));
  }

  assertOwner(nodeId, canvasId) {
    const slot = this.requireCanvas(canvasId);
    if (slot.owner !== nodeId) {
      throw structuredError("canvas/not-owner", `${nodeId} does not own ${canvasId}`);
    }
    return slot;
  }

  assertAccess(nodeId, canvasId) {
    const slot = this.requireCanvas(canvasId);
    if (slot.owner !== nodeId && slot.candidate !== nodeId) {
      throw structuredError("canvas/not-owner", `${nodeId} does not own ${canvasId}`);
    }
    return slot;
  }

  requireCanvas(canvasId) {
    const slot = this.canvases.get(canvasId);
    if (!slot) throw structuredError("canvas/not-found", `unknown canvas ${canvasId}`);
    return slot;
  }

  resolveFirstRender(slot, nodeId) {
    slot.firstRender.get(nodeId)?.resolve();
    slot.firstRender.delete(nodeId);
  }

  rejectFirstRender(slot, nodeId, error) {
    slot.firstRender.get(nodeId)?.reject(error);
    slot.firstRender.delete(nodeId);
  }

  cancelOwner(slot, nodeId, reason) {
    for (const [token, reject] of slot.pending) {
      this.cancelFrame(token);
      reject(structuredError("canvas/cancelled", reason));
    }
    slot.pending.clear();
    this.rejectFirstRender(slot, nodeId, structuredError("canvas/cancelled", reason));
  }

  cancelSlot(slot, reason) {
    if (slot.owner) this.cancelOwner(slot, slot.owner, reason);
  }

  disposeWebGl(slot) {
    if (!slot.webgl?.gl) return;
    for (const program of slot.webgl.programs.values()) slot.webgl.gl.deleteProgram(program);
    slot.webgl = null;
  }

  close() {
    for (const remove of this.listeners.splice(0)) remove();
    for (const slot of this.canvases.values()) {
      this.cancelSlot(slot, "canvas runtime closed");
      this.disposeWebGl(slot);
    }
    this.canvases.clear();
  }
}

function execute2d(context, command, width, height) {
  if (!Array.isArray(command) || command.length === 0) return;
  const name = keyName(command[0]);
  if (name === "grid") {
    const spacing = Number(command[1] ?? 48);
    context.strokeStyle = command[2] ?? "rgba(65,245,228,.08)";
    context.lineWidth = Number(command[3] ?? 1);
    context.beginPath();
    for (let x = 0; x <= width; x += spacing) { context.moveTo(x, 0); context.lineTo(x, height); }
    for (let y = 0; y <= height; y += spacing) { context.moveTo(0, y); context.lineTo(width, y); }
    context.stroke();
  } else if (name === "line") {
    context.strokeStyle = command[5] ?? "#41f5e4";
    context.lineWidth = Number(command[6] ?? 2);
    context.globalAlpha = Number(command[7] ?? 1);
    context.beginPath();
    context.moveTo(Number(command[1]), Number(command[2]));
    context.lineTo(Number(command[3]), Number(command[4]));
    context.stroke();
    context.globalAlpha = 1;
  } else if (name === "polyline") {
    const points = command[1] ?? [];
    if (points.length < 2) return;
    context.strokeStyle = command[2] ?? "#41f5e4";
    context.lineWidth = Number(command[3] ?? 2);
    context.globalAlpha = Number(command[4] ?? 1);
    context.beginPath();
    context.moveTo(Number(points[0][0]), Number(points[0][1]));
    for (const point of points.slice(1)) context.lineTo(Number(point[0]), Number(point[1]));
    context.stroke();
    context.globalAlpha = 1;
  } else if (name === "rect") {
    context.fillStyle = command[5] ?? "#41f5e4";
    context.globalAlpha = Number(command[6] ?? 1);
    context.fillRect(Number(command[1]), Number(command[2]), Number(command[3]), Number(command[4]));
    context.globalAlpha = 1;
  } else if (name === "circle") {
    context.fillStyle = command[4] ?? "#41f5e4";
    context.globalAlpha = Number(command[5] ?? 1);
    context.beginPath();
    context.arc(Number(command[1]), Number(command[2]), Number(command[3]), 0, Math.PI * 2);
    context.fill();
    context.globalAlpha = 1;
  } else if (name === "text") {
    context.fillStyle = command[4] ?? "#eaffff";
    context.font = `${Number(command[5] ?? 12)}px ui-monospace, monospace`;
    context.fillText(String(command[1]), Number(command[2]), Number(command[3]));
  }
}

function linkProgram(gl, vertexSource, fragmentSource) {
  const compile = (type, source, label) => {
    const shader = gl.createShader(type);
    gl.shaderSource(shader, source);
    gl.compileShader(shader);
    if (!gl.getShaderParameter(shader, gl.COMPILE_STATUS)) {
      const log = gl.getShaderInfoLog(shader);
      gl.deleteShader(shader);
      throw new Error(`${label} shader: ${log}`);
    }
    return shader;
  };
  const vertex = compile(gl.VERTEX_SHADER, vertexSource, "vertex");
  const fragment = compile(gl.FRAGMENT_SHADER, fragmentSource, "fragment");
  const program = gl.createProgram();
  gl.attachShader(program, vertex);
  gl.attachShader(program, fragment);
  gl.linkProgram(program);
  gl.deleteShader(vertex);
  gl.deleteShader(fragment);
  if (!gl.getProgramParameter(program, gl.LINK_STATUS)) {
    const log = gl.getProgramInfoLog(program);
    gl.deleteProgram(program);
    throw new Error(`program link: ${log}`);
  }
  return program;
}

function plain(value) {
  if (value instanceof Map) {
    return Object.fromEntries([...value].map(([key, entry]) => [keyName(key), plain(entry)]));
  }
  if (Array.isArray(value)) return value.map(plain);
  return value;
}

function toHta(value) {
  if (Array.isArray(value)) return value.map(toHta);
  if (value && typeof value === "object") {
    return new Map(Object.entries(value).map(([key, entry]) => [key, toHta(entry)]));
  }
  return value;
}

function keyName(value) {
  return value?.constructor?.name === "HtaKeyword" ? value.name : String(value);
}

function point(event) {
  return {
    x: Math.round(event.clientX ?? 0),
    y: Math.round(event.clientY ?? 0),
    button: event.button ?? 0,
    pointer: event.pointerType ?? "mouse"
  };
}

function modifiers(event) {
  return [event.ctrlKey && "ctrl", event.altKey && "alt", event.shiftKey && "shift", event.metaKey && "meta"]
    .filter(Boolean);
}

function structuredError(code, message) {
  const error = new Error(message);
  error.code = code;
  error.origin = "studio.canvas";
  error.retryable = false;
  return error;
}

function hashText(text) {
  let hash = 2166136261;
  for (let index = 0; index < text.length; index += 1) {
    hash ^= text.charCodeAt(index);
    hash = Math.imul(hash, 16777619);
  }
  return (hash >>> 0).toString(16);
}

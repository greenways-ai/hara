const HEX_COLOR = /^#(?:[0-9a-f]{3}|[0-9a-f]{6}|[0-9a-f]{8})$/i;
const MAX_COMMANDS = 5000;

export function keywordName(value) {
  if (typeof value === "string") return value.replace(/^:/, "");
  if (value && typeof value.name === "string") return value.name.replace(/^:/, "");
  return null;
}

export function mapValue(value, key) {
  if (!(value instanceof Map)) return undefined;
  for (const [candidate, item] of value) {
    if (keywordName(candidate) === key || candidate === key) return item;
  }
  return undefined;
}

function finite(value, label) {
  if (typeof value !== "number" || !Number.isFinite(value)) {
    throw new Error(`${label} must be a finite number`);
  }
  return value;
}

function positive(value, label, limit = 8192) {
  value = finite(value, label);
  if (value <= 0 || value > limit) throw new Error(`${label} must be between 1 and ${limit}`);
  return value;
}

function color(value, label) {
  if (typeof value !== "string" || !HEX_COLOR.test(value)) {
    throw new Error(`${label} must be a hexadecimal color`);
  }
  return value;
}

function point(value, label) {
  if (!Array.isArray(value) || value.length !== 2) {
    throw new Error(`${label} must be [x y]`);
  }
  return [finite(value[0], `${label} x`), finite(value[1], `${label} y`)];
}

function command(value, index) {
  if (!Array.isArray(value) || value.length === 0) {
    throw new Error(`command ${index + 1} must be a vector`);
  }
  const type = keywordName(value[0]);
  const label = `command ${index + 1}`;
  switch (type) {
    case "line":
      if (value.length !== 7) throw new Error(`${label} :line expects 6 arguments`);
      return {
        type,
        x1: finite(value[1], `${label} x1`),
        y1: finite(value[2], `${label} y1`),
        x2: finite(value[3], `${label} x2`),
        y2: finite(value[4], `${label} y2`),
        color: color(value[5], `${label} color`),
        width: positive(value[6], `${label} width`, 128)
      };
    case "circle":
      if (value.length !== 5) throw new Error(`${label} :circle expects 4 arguments`);
      return {
        type,
        x: finite(value[1], `${label} x`),
        y: finite(value[2], `${label} y`),
        radius: positive(value[3], `${label} radius`),
        color: color(value[4], `${label} color`)
      };
    case "rect":
      if (value.length !== 6) throw new Error(`${label} :rect expects 5 arguments`);
      return {
        type,
        x: finite(value[1], `${label} x`),
        y: finite(value[2], `${label} y`),
        width: positive(value[3], `${label} width`),
        height: positive(value[4], `${label} height`),
        color: color(value[5], `${label} color`)
      };
    case "polyline":
      if (value.length !== 4) throw new Error(`${label} :polyline expects 3 arguments`);
      if (!Array.isArray(value[1]) || value[1].length < 2) {
        throw new Error(`${label} points must contain at least two points`);
      }
      return {
        type,
        points: value[1].map((item, pointIndex) => point(item, `${label} point ${pointIndex + 1}`)),
        color: color(value[2], `${label} color`),
        width: positive(value[3], `${label} width`, 128)
      };
    default:
      throw new Error(`${label} has unsupported type :${type ?? "unknown"}`);
  }
}

export function validateScene(value) {
  if (!(value instanceof Map)) throw new Error("program must return a scene map");
  const version = mapValue(value, "version");
  if (version !== 1) throw new Error("scene :version must be 1");
  const width = positive(mapValue(value, "width"), "scene width");
  const height = positive(mapValue(value, "height"), "scene height");
  const background = color(mapValue(value, "background"), "scene background");
  const rawCommands = mapValue(value, "commands");
  if (!Array.isArray(rawCommands)) throw new Error("scene :commands must be a vector or list");
  if (rawCommands.length > MAX_COMMANDS) {
    throw new Error(`scene cannot contain more than ${MAX_COMMANDS} commands`);
  }
  return {
    version,
    width,
    height,
    background,
    commands: rawCommands.map(command)
  };
}

export function renderScene(canvas, scene, options = {}) {
  const bounds = canvas.getBoundingClientRect();
  const cssWidth = Math.max(1, options.width ?? bounds.width);
  const cssHeight = Math.max(1, options.height ?? bounds.height);
  const pixelRatio = Math.min(2, options.pixelRatio ?? globalThis.devicePixelRatio ?? 1);
  canvas.width = Math.round(cssWidth * pixelRatio);
  canvas.height = Math.round(cssHeight * pixelRatio);

  const context = canvas.getContext("2d");
  if (!context) throw new Error("2D canvas is unavailable");
  const scale = Math.min(cssWidth / scene.width, cssHeight / scene.height);
  const offsetX = (cssWidth - scene.width * scale) / 2;
  const offsetY = (cssHeight - scene.height * scale) / 2;

  context.setTransform(pixelRatio, 0, 0, pixelRatio, 0, 0);
  context.clearRect(0, 0, cssWidth, cssHeight);
  context.fillStyle = scene.background;
  context.fillRect(0, 0, cssWidth, cssHeight);
  context.save();
  context.translate(offsetX, offsetY);
  context.scale(scale, scale);
  context.lineCap = "round";
  context.lineJoin = "round";

  for (const item of scene.commands) {
    context.shadowColor = item.color;
    context.shadowBlur = 12 / Math.max(scale, .1);
    if (item.type === "line") {
      context.beginPath();
      context.moveTo(item.x1, item.y1);
      context.lineTo(item.x2, item.y2);
      context.strokeStyle = item.color;
      context.lineWidth = item.width;
      context.stroke();
    } else if (item.type === "circle") {
      context.beginPath();
      context.arc(item.x, item.y, item.radius, 0, Math.PI * 2);
      context.fillStyle = item.color;
      context.fill();
    } else if (item.type === "rect") {
      context.fillStyle = item.color;
      context.fillRect(item.x, item.y, item.width, item.height);
    } else if (item.type === "polyline") {
      context.beginPath();
      context.moveTo(...item.points[0]);
      for (const pointValue of item.points.slice(1)) context.lineTo(...pointValue);
      context.strokeStyle = item.color;
      context.lineWidth = item.width;
      context.stroke();
    }
  }
  context.restore();
  return { cssWidth, cssHeight, scale };
}

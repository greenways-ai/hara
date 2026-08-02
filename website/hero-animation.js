const canvas = document.querySelector("[data-hero-canvas]");
const sceneButtons = [...document.querySelectorAll("[data-hero-scene]")];
const installDialog = document.querySelector("[data-install-dialog]");
const installCommand = "brew install hara-lang/tap/hara";

const vertexSource = `#version 300 es
precision highp float;
void main() {
  vec2 point = vec2((gl_VerID << 1) & 2, gl_VerID & 2);
  gl_Position = vec4(point * 2.0 - 1.0, 0.0, 1.0);
}`.replaceAll("gl_VerID", "gl_VertexID");

const columnSource = `#version 300 es
precision highp float;
out vec4 color;
uniform vec2 u_resolution;
uniform float u_time;

mat2 turn(float angle) {
  float c = cos(angle), s = sin(angle);
  return mat2(c, -s, s, c);
}

float boxDistance(vec3 p, vec3 bounds) {
  vec3 q = abs(p) - bounds;
  return length(max(q, 0.0)) + min(max(q.x, max(q.y, q.z)), 0.0);
}

float columnDistance(vec3 p, float halfHeight, float radius) {
  vec2 q = abs(vec2(length(p.xz), p.y)) - vec2(radius, halfHeight);
  return min(max(q.x, q.y), 0.0) + length(max(q, 0.0));
}

float marble(vec3 p) {
  float vein = sin(p.x * 2.7 + sin(p.z * 1.9) * 1.8 + sin(p.x * .37 + p.z * .61) * 5.0);
  return smoothstep(.72, .98, abs(vein));
}

vec2 world(vec3 p) {
  float floorDistance = p.y + 1.25;
  vec2 result = vec2(floorDistance, 1.0);

  vec3 repeated = p;
  repeated.z = mod(repeated.z + 2.0, 4.0) - 2.0;
  repeated.x = abs(repeated.x) - 3.1;
  vec3 local = repeated - vec3(0.0, .95, 0.0);
  float shaft = columnDistance(local, 2.18, .34);
  float flutes = .025 * sin(atan(local.z, local.x) * 16.0);
  shaft += flutes;
  float base = boxDistance(repeated - vec3(0.0, -1.08, 0.0), vec3(.57, .17, .57));
  base = min(base, boxDistance(repeated - vec3(0.0, -.82, 0.0), vec3(.46, .12, .46)));
  float capital = boxDistance(repeated - vec3(0.0, 3.02, 0.0), vec3(.57, .18, .57));
  capital = min(capital, boxDistance(repeated - vec3(0.0, 2.74, 0.0), vec3(.45, .12, .45)));
  float architecture = min(shaft, min(base, capital));
  if (architecture < result.x) result = vec2(architecture, 2.0);

  float beam = boxDistance(vec3(p.x, p.y - 3.32, mod(p.z + 2.0, 4.0) - 2.0), vec3(3.75, .18, .34));
  if (beam < result.x) result = vec2(beam, 3.0);
  return result;
}

vec3 normalAt(vec3 p) {
  vec2 e = vec2(.002, 0.0);
  float d = world(p).x;
  return normalize(vec3(
    world(p + e.xyy).x - d,
    world(p + e.yxy).x - d,
    world(p + e.yyx).x - d));
}

mat3 camera(vec3 origin, vec3 target) {
  vec3 forward = normalize(target - origin);
  vec3 right = normalize(cross(forward, vec3(0.0, 1.0, 0.0)));
  return mat3(right, cross(right, forward), forward);
}

void main() {
  vec2 uv = (gl_FragCoord.xy * 2.0 - u_resolution) / u_resolution.y;
  float time = u_time * .001;
  vec3 origin = vec3(sin(time * .09) * .22, .18, -7.0 + mod(time * .23, 4.0));
  vec3 ray = camera(origin, vec3(0.0, .42, origin.z + 8.0)) * normalize(vec3(uv, 1.42));
  float travel = .05;
  vec2 hit = vec2(0.0);
  for (int stepIndex = 0; stepIndex < 82; stepIndex++) {
    vec3 point = origin + ray * travel;
    hit = world(point);
    if (hit.x < .002 || travel > 45.0) break;
    travel += hit.x * .72;
  }

  vec3 result = vec3(.006, .011, .010);
  float stars = pow(max(0.0, sin(uv.x * 173.0) * sin(uv.y * 211.0)), 42.0);
  result += vec3(.44, .58, .52) * stars * smoothstep(.2, 1.0, uv.y);
  if (travel < 45.0) {
    vec3 point = origin + ray * travel;
    vec3 normal = normalAt(point);
    vec3 sun = normalize(vec3(-.42, .78, -.36));
    float diffuse = max(dot(normal, sun), 0.0);
    float rim = pow(1.0 - max(dot(normal, -ray), 0.0), 3.0);
    vec3 material = vec3(.12, .20, .16);
    if (hit.y > 1.5) material = mix(vec3(.43, .39, .31), vec3(.84, .77, .63), .55 + .45 * marble(point));
    if (hit.y > 2.5) material *= vec3(.76, .68, .52);
    if (hit.y < 1.5) {
      vec2 tile = abs(fract(point.xz * .5) - .5);
      float joint = smoothstep(.455, .49, max(tile.x, tile.y));
      material = mix(material, vec3(.035, .045, .038), joint);
      material += vec3(.17, .14, .09) * marble(point) * .32;
    }
    float lamp = 1.0 / (1.0 + .035 * length(point - vec3(0.0, 4.5, origin.z + 6.0)));
    result = material * (.13 + diffuse * 1.35) + vec3(.95, .69, .34) * lamp * .16 + rim * vec3(.18, .31, .25);
    result = mix(result, vec3(.025, .034, .030), smoothstep(15.0, 43.0, travel));
  }
  float vignette = smoothstep(1.65, .18, dot(uv * vec2(.65, .82), uv * vec2(.65, .82)));
  result *= .34 + .66 * vignette;
  result = result / (result + vec3(.82));
  color = vec4(pow(result, vec3(.4545)), 1.0);
}`;

const mosaicSource = `#version 300 es
precision highp float;
out vec4 color;
uniform vec2 u_resolution;
uniform float u_time;

float hash(vec2 p) {
  p = fract(p * vec2(123.34, 456.21));
  p += dot(p, p + 45.32);
  return fract(p.x * p.y);
}

float marble(vec2 p) {
  float value = sin(p.x * 3.4 + sin(p.y * 2.1) * 2.2);
  value += .45 * sin(p.x * 9.0 - p.y * 5.0);
  return smoothstep(.65, 1.25, abs(value));
}

void main() {
  float time = u_time * .001;
  vec2 uv = (gl_FragCoord.xy * 2.0 - u_resolution) / u_resolution.y;
  uv *= turn_placeholder;
  float radius = length(uv);
  float angle = atan(uv.y, uv.x);
  float ring = floor(radius * 12.0);
  float wedge = floor((angle + 3.14159265) / 6.2831853 * 32.0 + mod(ring, 2.0) * .5);
  float identity = hash(vec2(ring, wedge));
  float radialJoint = smoothstep(.43, .49, abs(fract(radius * 12.0) - .5));
  float angularCell = fract((angle + 3.14159265) / 6.2831853 * 32.0 + mod(ring, 2.0) * .5);
  float angularJoint = smoothstep(.455, .495, abs(angularCell - .5)) * smoothstep(.08, .2, radius);
  float joint = max(radialJoint, angularJoint);

  vec3 ivory = vec3(.69, .65, .54);
  vec3 green = vec3(.035, .19, .13);
  vec3 lapis = vec3(.035, .13, .24);
  vec3 bronze = vec3(.43, .27, .11);
  vec3 tile = mix(green, lapis, step(.52, identity));
  tile = mix(tile, ivory, step(.82, identity));
  tile = mix(tile, bronze, step(.94, identity));
  float veins = marble(uv * 3.0 + identity * 4.0);
  tile = mix(tile, tile + vec3(.20, .17, .11), veins * .28);
  tile *= .82 + .18 * hash(vec2(wedge, ring + 11.0));

  float medallion = smoothstep(.34, .32, radius);
  float star = .5 + .5 * cos(angle * 8.0 + time * .08);
  vec3 centre = mix(bronze, ivory, smoothstep(.38, .72, star));
  tile = mix(tile, centre, medallion);
  tile = mix(tile, vec3(.014, .021, .018), joint * .82);

  vec2 lightPoint = vec2(sin(time * .13) * .72, cos(time * .09) * .34);
  float light = exp(-2.2 * length(uv - lightPoint));
  tile += vec3(.38, .27, .13) * light * .42;
  float vignette = smoothstep(1.45, .18, radius);
  tile *= .34 + .66 * vignette;
  color = vec4(pow(tile, vec3(.78)), 1.0);
}`.replace("uv *= turn_placeholder;", "uv *= mat2(cos(time * .012), -sin(time * .012), sin(time * .012), cos(time * .012));");

function compile(gl, type, source) {
  const shader = gl.createShader(type);
  gl.shaderSource(shader, source);
  gl.compileShader(shader);
  if (!gl.getShaderParameter(shader, gl.COMPILE_STATUS)) {
    const message = gl.getShaderInfoLog(shader);
    gl.deleteShader(shader);
    throw new Error(message || "Unable to compile hero shader");
  }
  return shader;
}

function programFor(gl, fragmentSource) {
  const program = gl.createProgram();
  gl.attachShader(program, compile(gl, gl.VERTEX_SHADER, vertexSource));
  gl.attachShader(program, compile(gl, gl.FRAGMENT_SHADER, fragmentSource));
  gl.linkProgram(program);
  if (!gl.getProgramParameter(program, gl.LINK_STATUS)) {
    throw new Error(gl.getProgramInfoLog(program) || "Unable to link hero shader");
  }
  return {
    program,
    resolution: gl.getUniformLocation(program, "u_resolution"),
    time: gl.getUniformLocation(program, "u_time")
  };
}

function startHero() {
  if (!canvas) return;
  const gl = canvas.getContext("webgl2", { alpha: false, antialias: false, powerPreference: "high-performance" });
  if (!gl) {
    canvas.dataset.unavailable = "true";
    return;
  }

  let programs;
  try {
    programs = {
      columns: programFor(gl, columnSource),
      mosaic: programFor(gl, mosaicSource)
    };
  } catch (error) {
    console.warn("Hara hero animation unavailable", error);
    canvas.dataset.unavailable = "true";
    return;
  }

  let activeScene = "columns";
  let frame = 0;
  let visible = true;
  const reduceMotion = matchMedia("(prefers-reduced-motion: reduce)");
  const startedAt = performance.now();

  function resize() {
    const bounds = canvas.getBoundingClientRect();
    const ratio = Math.min(devicePixelRatio || 1, 1.5);
    const width = Math.max(1, Math.round(bounds.width * ratio));
    const height = Math.max(1, Math.round(bounds.height * ratio));
    if (canvas.width !== width || canvas.height !== height) {
      canvas.width = width;
      canvas.height = height;
    }
  }

  function draw(now) {
    resize();
    const active = programs[activeScene];
    gl.viewport(0, 0, canvas.width, canvas.height);
    gl.useProgram(active.program);
    gl.uniform2f(active.resolution, canvas.width, canvas.height);
    gl.uniform1f(active.time, reduceMotion.matches ? 0 : now - startedAt);
    gl.drawArrays(gl.TRIANGLES, 0, 3);
    frame = visible && !reduceMotion.matches ? requestAnimationFrame(draw) : 0;
  }

  function renderOnce() {
    if (frame) cancelAnimationFrame(frame);
    frame = requestAnimationFrame(draw);
  }

  sceneButtons.forEach((button) => button.addEventListener("click", () => {
    activeScene = button.dataset.heroScene;
    canvas.setAttribute("aria-label", activeScene === "columns"
      ? "Animated monumental Roman colonnade"
      : "Animated marble mosaic medallion");
    sceneButtons.forEach((item) => item.setAttribute("aria-pressed", String(item === button)));
    renderOnce();
  }));

  new IntersectionObserver(([entry]) => {
    visible = entry.isIntersecting;
    if (visible && !frame) renderOnce();
    if (!visible && frame) {
      cancelAnimationFrame(frame);
      frame = 0;
    }
  }, { threshold: .02 }).observe(canvas);
  reduceMotion.addEventListener?.("change", renderOnce);
  addEventListener("resize", renderOnce, { passive: true });
  renderOnce();
}

document.querySelectorAll("[data-install-trigger]").forEach((button) => {
  button.addEventListener("click", () => installDialog?.showModal());
});
document.querySelector("[data-install-close]")?.addEventListener("click", () => installDialog.close());
installDialog?.addEventListener("click", (event) => {
  if (event.target === installDialog) installDialog.close();
});
document.querySelector("[data-install-copy]")?.addEventListener("click", async (event) => {
  try {
    await navigator.clipboard.writeText(installCommand);
    event.currentTarget.textContent = "COPIED";
    setTimeout(() => { event.currentTarget.textContent = "COPY"; }, 1600);
  } catch {
    event.currentTarget.textContent = "SELECT COMMAND";
    const selection = getSelection();
    const range = document.createRange();
    range.selectNodeContents(document.querySelector(".install-command code"));
    selection.removeAllRanges();
    selection.addRange(range);
  }
});

startHero();

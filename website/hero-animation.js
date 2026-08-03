const canvas = document.querySelector("[data-hero-canvas]");
const sceneButtons = [...document.querySelectorAll("[data-hero-scene]")];
const mosaicBackdrop = document.querySelector("[data-hero-mosaic]");
const installDialog = document.querySelector("[data-install-dialog]");
const installCommand = "brew install hara-lang/tap/hara";
const mosaicSeedDataUrl = "data:image/jpeg;base64,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";

const mosaicSeedDataUrl64 = "data:image/jpeg;base64,/9j/4AAQSkZJRgABAQAASABIAAD/4QBMRXhpZgAATU0AKgAAAAgAAYdpAAQAAAABAAAAGgAAAAAAA6ABAAMAAAABAAEAAKACAAQAAAABAAAAQKADAAQAAAABAAAAJAAAAAD/7QA4UGhvdG9zaG9wIDMuMAA4QklNBAQAAAAAAAA4QklNBCUAAAAAABDUHYzZjwCyBOmACZjs+EJ+/8AAEQgAJABAAwEiAAIRAQMRAf/EAB8AAAEFAQEBAQEBAAAAAAAAAAABAgMEBQYHCAkKC//EALUQAAIBAwMCBAMFBQQEAAABfQECAwAEEQUSITFBBhNRYQcicRQygZGhCCNCscEVUtHwJDNicoIJChYXGBkaJSYnKCkqNDU2Nzg5OkNERUZHSElKU1RVVldYWVpjZGVmZ2hpanN0dXZ3eHl6g4SFhoeIiYqSk5SVlpeYmZqio6Slpqeoqaqys7S1tre4ubrCw8TFxsfIycrS09TV1tfY2drh4uPk5ebn6Onq8fLz9PX29/j5+v/EAB8BAAMBAQEBAQEBAQEAAAAAAAABAgMEBQYHCAkKC//EALURAAIBAgQEAwQHBQQEAAECdwABAgMRBAUhMQYSQVEHYXETIjKBCBRCkaGxwQkjM1LwFWJy0QoWJDThJfEXGBkaJicoKSo1Njc4OTpDREVGR0hJSlNUVVZXWFlaY2RlZmdoaWpzdHV2d3h5eoKDhIWGh4iJipKTlJWWl5iZmqKjpKWmp6ipqrKztLW2t7i5usLDxMXGx8jJytLT1NXW19jZ2uLj5OXm5+jp6vLz9PX29/j5+v/bAEMAAgICAgICAwICAwQDAwMEBgQEBAQGBwYGBgYGBwkHBwcHBwcJCQkJCQkJCQoKCgoKCgwMDAwMDg4ODg4ODg4ODv/bAEMBAgICAwMDBgMDBg4KCAoODg4ODg4ODg4ODg4ODg4ODg4ODg4ODg4ODg4ODg4ODg4ODg4ODg4ODg4ODg4ODg4ODv/dAAQABP/aAAwDAQACEQMRAD8A/D07iVweQMZXoaeYwT5eeOMj3qqrs6qW44//AF1ct8ElnznGa1Uo2JaZ0uh2ENzcLFNkRFdp24zz0HINd/F4NV7yHTbICV7iQIjEgYUqrEEjgYJ+bpkCsTwbDDPdStKwwi7to4LABmIB6A4WvqXw7olhDZaPLPp6tH9vnAYYLP5aodrEkEhg6469O+K8fGYtUauv9abno4eg6lOy/rUwvCPwS0+dIZbqKTM6M6qqo0joGKGUu5WOKLcNokdwGP3Q2M12XiP4Y+HdUiMsUD2a2luIHnQw3MYRerTLERIi92fYyr1Nehavdz2llpWmajGtpb3NxH9seFjsMSQpHboCfmCqo6EDlie9V9buf7E8Radd6TCsM63kSQLEc+fGSfM3gZDLjB59cV5UsW51VaWru1omtO78/nbc6Y0eWm3y6K19dfkj5P8AFXwvHhmYsY/LthKsM67g+3eNyMhBIZGBDBhxnaQccVwms+DNPsdCgvo5GkvZFZyiKNihc7sjGeMHv1Br7o8T6RbXNzZW9xpsTQ/arf7FyHJtvOkMccpJ2kAOBxuG1R6CvJvE+i2llo+vJLEli+6ZmGciQrI0axoo6ZZCVxwQeTXdRzJVXGMVrp/w3fr+GpjPCuEXJ7f18j//0Pw2ijcxbyOEOCasxuVO1enetnTYtJa1P2lbh5RgbY8KoJ/Mn8qty2USbXg5SRflVlwwOeh64xjr3pqor8rFPRcxr+FPNe6kCKW3W04wvPJgkAH4mvqPR28UDSrW1tLeXy7KWaT7pG0tHGzcn/dyPpXgHhaXTrG9+0ytLbxIBjyEDyFvpkdOSTnFe3afePdzRrc3BmtXHEytIg2MvVkxgYAIP5968vG0HXqarSx1UMSqcPd3v/wD3zRNOc+Jo4L/AMu7trR41ud0gK7Vt1UMgYjPzZHFTxaO6aDcCRFN4t/DMJZHQNHb8+bsIbsDwBz6V5vJeW9pqEV/oizXDiZOLgFM7lC72CucDpwefXB4q1e3Mn2aGyt5Ibi2luEaaSLzQyMilgcMRmM9iOD61xvBU01H2mrt+H9fMf1+s3KfJom7699v+D2L81t4kKaPc6fuuE+ziRgp3lCkpRQR69/oa898TJqc2n6zqGoW8kTXrwS73QqpPnTFyAewOc47Vs3lzYxwWw02ScukYQpKWj2jeW3BkZsqT7bgeCOhqDVtfguvDv8AY+o38z3cqFlhWLdAmV4R5WOfMIJH3SM8E4rpoYKVKoq1Jp6/r89NiZ49TpuFbstvRd7dj//R/ECLUrkAtxliBnnt+NX08Q6ghAAi+VMDK+uF59enX1rAi+4P97+lP/iP+6P/AEKuOye50I6/TvFWppPEAIvk4GVPOOeeefxrt9P+J/iaERSILfKfu8bDggkDJG7qMcGvI7H/AI+F+prXtf8AVL/10H865MRCN9juoJWPcV+MXi1LSF9lm2YnUhoiQQFEfPzZJwM5Jzu561Wg+LXiqzYpAtmBOAjfufUglhg8Mdo59MjvXmZ/48YP9yT/ANCpkn+ti/31/rXF7KF07HXsrI9Avvit4puLtkcWw3HblYyCBGDjHzcZ74696yrr4jeIZI0jYW+2Ikj5Dzzn5vm5HOOe3FcVP/x/f8Df+VQTfxfj/IV1whHTQ5pxWp//2Q==";

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
  repeated.z = mod(repeated.z + 2.4, 4.8) - 2.4;
  repeated.x = abs(repeated.x) - 3.65;
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

  vec3 crossBay = p;
  crossBay.z = mod(crossBay.z + 2.4, 4.8) - 2.4;
  vec2 crossProfile = vec2(crossBay.x, (crossBay.y - 3.08) * 1.62);
  float crossArch = abs(length(crossProfile) - 3.65) - .16;
  crossArch = max(crossArch, 3.02 - crossBay.y);
  crossArch = max(crossArch, abs(crossBay.z) - .24);
  if (crossArch < result.x) result = vec2(crossArch, 3.0);

  vec3 sideBay = p;
  sideBay.x = abs(sideBay.x) - 3.65;
  sideBay.z = mod(sideBay.z, 4.8) - 2.4;
  vec2 sideProfile = vec2(sideBay.z, (sideBay.y - 2.96) * 1.28);
  float sideArch = abs(length(sideProfile) - 2.4) - .15;
  sideArch = max(sideArch, 2.90 - sideBay.y);
  sideArch = max(sideArch, abs(sideBay.x) - .25);
  if (sideArch < result.x) result = vec2(sideArch, 3.0);

  vec2 vaultProfile = vec2(p.x, (p.y - 3.08) * 1.62);
  float vault = abs(length(vaultProfile) - 3.82) - .09;
  vault = max(vault, 3.04 - p.y);
  if (vault < result.x) result = vec2(vault, 4.0);

  float sanctuary = boxDistance(p - vec3(0.0, 2.02, 20.0), vec3(4.55, 3.27, .24));
  float oculus = length(vec2(p.x, p.y - 3.34)) - .94;
  sanctuary = max(sanctuary, -oculus);
  if (sanctuary < result.x) result = vec2(sanctuary, 5.0);

  float oculusRim = abs(length(vec2(p.x, p.y - 3.34)) - 1.08) - .10;
  oculusRim = max(oculusRim, abs(p.z - 19.70) - .15);
  if (oculusRim < result.x) result = vec2(oculusRim, 3.0);

  float altar = boxDistance(p - vec3(0.0, -1.06, 17.72), vec3(2.30, .18, 1.34));
  altar = min(altar, boxDistance(p - vec3(0.0, -.79, 18.12), vec3(1.48, .13, .72)));
  if (altar < result.x) result = vec2(altar, 6.0);
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
  vec3 origin = vec3(sin(time * .09) * .22, .18, -7.0 + mod(time * .23, 4.8));
  vec3 ray = camera(origin, vec3(0.0, .42, origin.z + 8.0)) * normalize(vec3(uv, 1.42));
  float travel = .05;
  vec2 hit = vec2(0.0);
  for (int stepIndex = 0; stepIndex < 82; stepIndex++) {
    vec3 point = origin + ray * travel;
    hit = world(point);
    if (hit.x < .002 || travel > 45.0) break;
    travel += hit.x * .72;
  }

  vec3 result = vec3(.004, .008, .012);
  float skyGradient = smoothstep(-.72, 1.08, uv.y);
  float vanishingGlow = exp(-1.45 * length((uv - vec2(.0, .12)) * vec2(.62, 1.0)));
  result += vec3(.008, .060, .082) * skyGradient;
  result += vec3(.012, .095, .12) * vanishingGlow * .58;
  if (travel < 45.0) {
    vec3 point = origin + ray * travel;
    vec3 normal = normalAt(point);
    vec3 sun = normalize(vec3(-.42, .78, -.36));
    float diffuse = max(dot(normal, sun), 0.0);
    float rim = pow(1.0 - max(dot(normal, -ray), 0.0), 3.0);
    vec3 neon = vec3(.035, .78, 1.0);
    vec3 emission = vec3(0.0);
    vec3 material = vec3(.055, .095, .10);
    if (hit.y > 1.5) {
      material = mix(vec3(.15, .17, .17), vec3(.70, .65, .54), .52 + .34 * marble(point));
      vec3 repeated = point;
      repeated.z = mod(repeated.z + 2.4, 4.8) - 2.4;
      repeated.x = abs(repeated.x) - 3.65;
      vec3 local = repeated - vec3(0.0, .95, 0.0);
      float fluteLine = pow(.5 + .5 * cos(atan(local.z, local.x) * 16.0), 28.0);
      float shaftWindow = smoothstep(-.96, -.72, repeated.y) * (1.0 - smoothstep(2.50, 2.72, repeated.y));
      float ringLine = smoothstep(.065, .0, min(
        min(abs(repeated.y + 1.22), abs(repeated.y + .70)),
        min(abs(repeated.y - 2.62), abs(repeated.y - 3.20))));
      float pulse = .82 + .18 * sin(time * .9 - point.z * .58);
      emission += neon * (fluteLine * shaftWindow * .72 + ringLine * 1.15) * pulse;
    }
    if (hit.y > 2.5 && hit.y < 3.5) {
      material *= vec3(.58, .64, .61);
      float ribPulse = .70 + .30 * sin(time * .64 - point.z * .46);
      emission += neon * .19 * ribPulse;
    }
    if (hit.y > 3.5 && hit.y < 4.5) {
      material = mix(vec3(.055, .075, .078), vec3(.29, .29, .25), .28 + .25 * marble(point));
      float vaultBay = abs(mod(point.z + 2.4, 4.8) - 2.4);
      float vaultGlow = 1.0 - smoothstep(.18, .72, vaultBay);
      emission += neon * vaultGlow * .12;
    }
    if (hit.y > 4.5) {
      material = mix(vec3(.075, .09, .09), vec3(.46, .42, .34), .32 + .30 * marble(point));
      if (hit.y > 5.5) emission += neon * .08;
    }
    if (hit.y < 1.5) {
      vec2 tile = abs(fract(point.xz * .5) - .5);
      float joint = smoothstep(.455, .49, max(tile.x, tile.y));
      material = mix(material, vec3(.012, .025, .032), joint);
      material += vec3(.17, .14, .09) * marble(point) * .32;
      float gridPulse = .74 + .26 * sin(time * .72 - point.z * .42);
      emission += neon * joint * gridPulse * .82;
    }
    float lamp = 1.0 / (1.0 + .035 * length(point - vec3(0.0, 4.5, origin.z + 6.0)));
    result = material * (.10 + diffuse * 1.18) + vec3(.78, .58, .34) * lamp * .11 + rim * vec3(.08, .34, .42) + emission;
    result = mix(result, vec3(.012, .025, .034), smoothstep(15.0, 43.0, travel));
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
uniform sampler2D u_seed;

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

float boxDistance(vec2 p, vec2 bounds) {
  vec2 q = abs(p) - bounds;
  return length(max(q, 0.0)) + min(max(q.x, q.y), 0.0);
}

float ellipseDistance(vec2 p, vec2 radius) {
  return length(p / radius) - 1.0;
}

float shape(float distanceValue) {
  return 1.0 - smoothstep(-.006, .014, distanceValue);
}

void legacyMosaic() {
  float time = u_time * .001;
  vec2 uv = (gl_FragCoord.xy * 2.0 - u_resolution) / u_resolution.y;

  vec3 obsidian = vec3(.008, .012, .032);
  vec3 lapis = vec3(.015, .075, .48);
  vec3 turquoise = vec3(.0, .64, .69);
  vec3 malachite = vec3(.015, .34, .18);
  vec3 oxblood = vec3(.62, .018, .075);
  vec3 violet = vec3(.29, .035, .54);
  vec3 gold = vec3(1.0, .49, .035);
  vec3 ivory = vec3(.88, .68, .34);

  vec2 cell = floor((uv + vec2(1.7, 1.1)) * vec2(29.0, 31.0));
  float identity = hash(cell);
  vec2 tessera = abs(fract((uv + vec2(1.7, 1.1)) * vec2(29.0, 31.0)) - .5);
  float joint = smoothstep(.425, .485, max(tessera.x, tessera.y));
  float edge = max(abs(uv.x) / 1.48, abs(uv.y) / .94);
  float muralMask = shape(edge - 1.0);

  vec3 mural = mix(vec3(.34, .075, .055), violet, .34 + .30 * identity);
  mural += ivory * marble(uv * 2.7 + identity * 3.0) * .13;

  vec2 p = uv;
  float outerArchDistance = min(
    boxDistance(p - vec2(0.0, -.15), vec2(.68, .45)),
    ellipseDistance(p - vec2(0.0, .26), vec2(.68, .68)));
  float innerArchDistance = min(
    boxDistance(p - vec2(0.0, -.16), vec2(.49, .40)),
    ellipseDistance(p - vec2(0.0, .25), vec2(.49, .49)));
  float outerArch = shape(outerArchDistance);
  float innerArch = shape(innerArchDistance);
  float archStone = clamp(outerArch - innerArch, 0.0, 1.0);
  mural = mix(mural, lapis * (.68 + .32 * identity), innerArch);
  float archStripe = .5 + .5 * cos(atan(p.y - .26, p.x) * 18.0);
  mural = mix(mural, mix(gold, turquoise, step(.46, archStripe)), archStone);

  float shaft = shape(boxDistance(vec2(abs(p.x) - .79, p.y + .05), vec2(.105, .55)));
  float capital = shape(boxDistance(vec2(abs(p.x) - .79, p.y - .53), vec2(.18, .075)));
  float base = shape(boxDistance(vec2(abs(p.x) - .79, p.y + .61), vec2(.17, .075)));
  float columns = max(shaft, max(capital, base));
  float fluting = pow(.5 + .5 * cos((abs(p.x) - .79) * 105.0), 12.0);
  vec3 columnColor = mix(ivory, turquoise, fluting * shaft * .72);
  columnColor = mix(columnColor, gold, max(capital, base) * .78);
  mural = mix(mural, columnColor, columns);

  float basinFace = shape(boxDistance(p - vec2(0.0, -.57), vec2(.61, .17)));
  float basinRim = shape(boxDistance(p - vec2(0.0, -.39), vec2(.68, .045)));
  vec3 basinColor = mix(lapis, turquoise, smoothstep(-.72, -.40, p.y));
  float wave = .5 + .5 * sin(p.x * 25.0 + sin(p.y * 36.0) + time * .75);
  basinColor += turquoise * pow(wave, 8.0) * .42;
  mural = mix(mural, basinColor, basinFace);
  mural = mix(mural, gold, basinRim);

  vec2 amphoraPoint = vec2(abs(p.x) - 1.10, p.y + .08);
  float amphoraBody = shape(ellipseDistance(amphoraPoint, vec2(.16, .25)));
  float amphoraNeck = shape(boxDistance(vec2(amphoraPoint.x, p.y - .22), vec2(.055, .12)));
  float amphoraFoot = shape(boxDistance(vec2(amphoraPoint.x, p.y + .34), vec2(.095, .035)));
  float amphora = max(amphoraBody, max(amphoraNeck, amphoraFoot));
  float amphoraBand = smoothstep(.018, .0, abs(abs(p.y + .08) - .07));
  vec3 amphoraColor = mix(oxblood, gold, amphoraBand);
  mural = mix(mural, amphoraColor, amphora);

  vec2 medallionPoint = p - vec2(0.0, .27);
  float medallionRadius = length(medallionPoint);
  float medallionAngle = atan(medallionPoint.y, medallionPoint.x);
  float medallion = shape(medallionRadius - .18);
  float corona = shape(medallionRadius - (.235 + .025 * cos(medallionAngle * 12.0)));
  corona *= 1.0 - medallion;
  mural = mix(mural, gold, corona);
  mural = mix(mural, mix(oxblood, violet, .5 + .5 * cos(medallionAngle * 8.0)), medallion);
  float jewel = shape(medallionRadius - .065);
  mural = mix(mural, turquoise * 1.35, jewel);

  float steamWindow = smoothstep(-.32, -.08, p.y) * (1.0 - smoothstep(.34, .48, p.y));
  float steam = exp(-105.0 * abs(p.x - .18 * sin(p.y * 8.0 + time * .36))) * steamWindow * innerArch;
  mural += turquoise * steam * .20;

  float outerFrame = shape(edge - .985);
  float innerFrame = shape(edge - .855);
  float frame = clamp(outerFrame - innerFrame, 0.0, 1.0);
  float key = step(.48, fract((abs(p.x) + abs(p.y)) * 11.0));
  vec3 frameColor = mix(lapis, gold, key * .92);
  frameColor = mix(frameColor, oxblood, step(.84, identity) * .38);
  mural = mix(mural, frameColor, frame);

  mural *= .84 + .22 * identity;
  mural = mix(mural, obsidian, joint * .55);
  float movingLight = exp(-2.5 * length(p - vec2(sin(time * .11) * .58, .12)));
  mural += mix(gold, turquoise, .38) * movingLight * .16;
  mural *= muralMask;
  mural *= .46 + .54 * smoothstep(1.30, .18, length(p * vec2(.62, .90)));
  color = vec4(pow(clamp(mural, 0.0, 1.0), vec3(.72)), 1.0);
}

// Voronoi tessellation concept adapted from the Flopine shader supplied by the user.
vec2 randomPoint(vec2 p) {
  return fract(sin(vec2(dot(p, vec2(1.2, 5.5)), dot(p, vec2(4.54, 2.41)))) * 43758.45);
}

vec3 voronoiCells(vec2 uv, float time) {
  vec2 cellId = floor(uv);
  vec2 cellPosition = fract(uv);
  vec2 nearestPoint = vec2(0.0);
  vec2 nearestOffset = vec2(0.0);
  vec2 nearestCell = vec2(0.0);
  float nearestDistance = 10.0;

  for (int y = -1; y <= 1; y++) {
    for (int x = -1; x <= 1; x++) {
      vec2 neighbor = vec2(float(x), float(y));
      vec2 point = randomPoint(cellId + neighbor);
      point = .5 + .34 * sin(6.2831853 * point + time * .38);
      vec2 offset = neighbor + point - cellPosition;
      float distanceToPoint = length(offset);
      if (distanceToPoint < nearestDistance) {
        nearestDistance = distanceToPoint;
        nearestPoint = point;
        nearestOffset = offset;
        nearestCell = neighbor;
      }
    }
  }

  float edgeDistance = 10.0;
  for (int y = -2; y <= 2; y++) {
    for (int x = -2; x <= 2; x++) {
      vec2 neighbor = nearestCell + vec2(float(x), float(y));
      vec2 point = randomPoint(cellId + neighbor);
      point = .5 + .34 * sin(6.2831853 * point + time * .38);
      vec2 offset = neighbor + point - cellPosition;
      vec2 separation = offset - nearestOffset;
      if (length(separation) > .001) {
        edgeDistance = min(edgeDistance, dot(.5 * (nearestOffset + offset), normalize(separation)));
      }
    }
  }
  return vec3(nearestPoint, edgeDistance);
}

vec3 palette3(vec3 first, vec3 second, vec3 third, float selector) {
  vec3 value = mix(first, second, smoothstep(.18, .62, selector));
  return mix(value, third, smoothstep(.72, .94, selector));
}

float rectangle(vec2 p, vec2 bounds) {
  return shape(boxDistance(p, bounds));
}

float archedOpening(vec2 p, float width, float height) {
  float lower = rectangle(p - vec2(0.0, -height * .28), vec2(width, height * .72));
  float upper = shape(length(p - vec2(0.0, height * .43)) - width);
  return max(lower, upper);
}

void proceduralTemple() {
  float time = u_time * .001;
  vec2 uv = gl_FragCoord.xy / u_resolution;
  uv -= .5;
  uv /= vec2(u_resolution.y / u_resolution.x, 1.0);

  vec3 lapis = vec3(.008, .045, .42);
  vec3 cobalt = vec3(.015, .15, .78);
  vec3 turquoise = vec3(.0, .70, .72);
  vec3 malachite = vec3(.01, .39, .18);
  vec3 porphyry = vec3(.62, .018, .07);
  vec3 violet = vec3(.31, .025, .55);
  vec3 gold = vec3(1.0, .55, .045);
  vec3 ivory = vec3(.92, .72, .40);
  vec3 obsidian = vec3(.006, .009, .025);

  vec3 cells = voronoiCells(uv * 31.0, time);
  float selector = fract(cells.x * 3.71 + cells.y * 5.13);
  float tesseraInterior = smoothstep(.035, .072, cells.z);
  float tileVariation = .78 + .28 * selector;

  vec3 scene = palette3(lapis, cobalt, violet, selector);
  float skyLight = smoothstep(-.18, .42, uv.y);
  scene = mix(scene, palette3(lapis, turquoise, cobalt, selector), skyLight * .34);

  vec2 sunPoint = uv - vec2(-.22, .23);
  float sun = shape(length(sunPoint) - .085);
  float sunAngle = atan(sunPoint.y, sunPoint.x) + time * .035;
  float sunRays = pow(.5 + .5 * cos(sunAngle * 14.0), 18.0);
  sunRays *= shape(length(sunPoint) - .15) * (1.0 - sun);
  scene = mix(scene, gold, max(sun, sunRays * .82));

  float farHillLine = -.055 + .035 * sin(uv.x * 5.2) + .018 * sin(uv.x * 13.0);
  float nearHillLine = -.14 + .045 * sin(uv.x * 4.0 + .8) - .02 * sin(uv.x * 11.0);
  float farHills = 1.0 - smoothstep(-.006, .008, uv.y - farHillLine);
  float nearHills = 1.0 - smoothstep(-.006, .008, uv.y - nearHillLine);
  scene = mix(scene, palette3(violet, porphyry, gold * .68, selector), farHills);
  scene = mix(scene, palette3(malachite, turquoise * .55, lapis, selector), nearHills);

  float water = 1.0 - smoothstep(-.008, .008, uv.y + .205);
  vec3 waterColor = palette3(lapis, cobalt, turquoise, selector);
  float waterLine = pow(.5 + .5 * sin(uv.x * 34.0 + uv.y * 58.0 + time * .52), 13.0);
  waterColor += turquoise * waterLine * .28;
  scene = mix(scene, waterColor, water);

  vec2 temple = uv - vec2(.22, -.015);
  float domeCircle = shape(length(temple - vec2(0.0, .105)) - .225);
  float dome = domeCircle * smoothstep(.086, .105, temple.y);
  float drum = rectangle(temple - vec2(0.0, .075), vec2(.255, .034));
  float body = rectangle(temple - vec2(0.0, -.075), vec2(.305, .16));
  float firstStep = rectangle(temple - vec2(0.0, -.252), vec2(.35, .026));
  float secondStep = rectangle(temple - vec2(0.0, -.292), vec2(.39, .018));

  float centralOpening = archedOpening(temple - vec2(0.0, -.085), .052, .12);
  float sideOpening = archedOpening(vec2(abs(temple.x) - .155, temple.y + .085), .038, .095);
  float openings = max(centralOpening, sideOpening);
  float wall = body * (1.0 - openings);

  float columnX = abs(mod(temple.x + .30, .12) - .06);
  float columnShafts = 1.0 - smoothstep(.018, .027, columnX);
  columnShafts *= rectangle(temple - vec2(0.0, -.075), vec2(.292, .135));
  float capitals = rectangle(vec2(temple.x, abs(temple.y + .075) - .132), vec2(.30, .011));
  float columns = max(columnShafts, capitals);

  vec3 stoneColor = palette3(ivory * .72, gold, porphyry, selector);
  scene = mix(scene, stoneColor, max(wall, max(drum, max(firstStep, secondStep))));
  vec3 columnColor = mix(ivory, turquoise, pow(.5 + .5 * cos(temple.x * 165.0), 14.0) * .58);
  scene = mix(scene, columnColor, columns);
  scene = mix(scene, obsidian, openings);

  float domeAngle = atan(temple.y - .105, temple.x);
  float domeRibs = pow(abs(sin(domeAngle * 9.0)), 22.0);
  vec3 domeColor = palette3(lapis, cobalt, turquoise, selector);
  domeColor = mix(domeColor, gold, domeRibs * .78);
  scene = mix(scene, domeColor, dome);
  float oculus = shape(length(temple - vec2(0.0, .17)) - .048);
  scene = mix(scene, mix(porphyry, gold, .62), oculus * dome);

  scene *= tileVariation;
  scene = mix(obsidian, scene, .25 + .75 * tesseraInterior);
  float quietLeft = smoothstep(-.72, -.18, uv.x);
  scene *= .50 + .50 * quietLeft;
  float vignette = smoothstep(.86, .18, length(uv * vec2(.72, 1.05)));
  scene *= .50 + .50 * vignette;
  color = vec4(pow(clamp(scene, 0.0, 1.0), vec3(.76)), 1.0);
}

vec4 seededVoronoi(vec2 uv, float time) {
  vec2 cellId = floor(uv);
  vec2 cellPosition = fract(uv);
  vec2 nearestOffset = vec2(0.0);
  vec2 nearestCell = vec2(0.0);
  vec2 nearestSite = vec2(0.0);
  float nearestDistance = 10.0;
  float evolution = smoothstep(.7, 8.0, time);

  for (int y = -1; y <= 1; y++) {
    for (int x = -1; x <= 1; x++) {
      vec2 neighbor = vec2(float(x), float(y));
      vec2 seed = randomPoint(cellId + neighbor);
      vec2 moving = .5 + .36 * sin(6.2831853 * seed + time * .34);
      vec2 point = mix(seed, moving, evolution);
      vec2 offset = neighbor + point - cellPosition;
      float distanceToPoint = length(offset);
      if (distanceToPoint < nearestDistance) {
        nearestDistance = distanceToPoint;
        nearestOffset = offset;
        nearestCell = neighbor;
        nearestSite = cellId + neighbor + point;
      }
    }
  }

  float edgeDistance = 10.0;
  for (int y = -2; y <= 2; y++) {
    for (int x = -2; x <= 2; x++) {
      vec2 neighbor = nearestCell + vec2(float(x), float(y));
      vec2 seed = randomPoint(cellId + neighbor);
      vec2 moving = .5 + .36 * sin(6.2831853 * seed + time * .34);
      vec2 point = mix(seed, moving, evolution);
      vec2 offset = neighbor + point - cellPosition;
      vec2 separation = offset - nearestOffset;
      if (length(separation) > .001) {
        edgeDistance = min(edgeDistance, dot(.5 * (nearestOffset + offset), normalize(separation)));
      }
    }
  }
  return vec4(nearestSite, edgeDistance, nearestDistance);
}

vec2 coverSeedCoordinates(vec2 uv) {
  float canvasAspect = u_resolution.x / u_resolution.y;
  float imageAspect = 1672.0 / 941.0;
  if (canvasAspect < imageAspect) {
    uv.x = (uv.x - .5) * (canvasAspect / imageAspect) + .5;
  } else {
    uv.y = (uv.y - .5) * (imageAspect / canvasAspect) + .5;
  }
  return clamp(uv, 0.0, 1.0);
}

void main() {
  float time = u_time * .001;
  float evolution = smoothstep(.7, 8.0, time);
  vec2 screenUv = gl_FragCoord.xy / u_resolution;
  float aspect = u_resolution.x / u_resolution.y;
  vec2 gridScale = vec2(42.0 * aspect, 42.0);
  vec4 cell = seededVoronoi(screenUv * gridScale, time);
  vec2 siteUv = cell.xy / gridScale;
  siteUv += evolution * .006 * vec2(
    sin(siteUv.y * 11.0 + time * .28),
    cos(siteUv.x * 9.0 - time * .23));

  vec3 original = texture(u_seed, coverSeedCoordinates(screenUv)).rgb;
  vec3 tessera = texture(u_seed, coverSeedCoordinates(siteUv)).rgb;
  float identity = hash(floor(cell.xy));
  tessera *= .82 + .24 * identity;
  float luminance = dot(tessera, vec3(.2126, .7152, .0722));
  tessera = mix(vec3(luminance), tessera, 1.32) * 1.08;
  float interior = smoothstep(.028, .073, cell.z);
  tessera = mix(vec3(.008, .010, .016), tessera, .22 + .78 * interior);

  float waterBand = 1.0 - smoothstep(.35, .60, screenUv.y);
  float shimmer = pow(.5 + .5 * sin(screenUv.x * 38.0 + screenUv.y * 71.0 + time * .72), 16.0);
  tessera += vec3(.02, .22, .24) * shimmer * waterBand * evolution * .22;
  vec3 scene = mix(original, tessera, evolution * .78);
  color = vec4(pow(clamp(scene, 0.0, 1.0), vec3(.94)), 1.0);
}`;

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
    time: gl.getUniformLocation(program, "u_time"),
    seed: gl.getUniformLocation(program, "u_seed")
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
  let seedReady = false;
  let sceneStartedAt = performance.now();
  let evolutionTimer = 0;
  const reduceMotion = matchMedia("(prefers-reduced-motion: reduce)");
  const startedAt = performance.now();
  const seedTexture = gl.createTexture();
  gl.bindTexture(gl.TEXTURE_2D, seedTexture);
  gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MIN_FILTER, gl.LINEAR);
  gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_MAG_FILTER, gl.LINEAR);
  gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_S, gl.CLAMP_TO_EDGE);
  gl.texParameteri(gl.TEXTURE_2D, gl.TEXTURE_WRAP_T, gl.CLAMP_TO_EDGE);
  gl.texImage2D(gl.TEXTURE_2D, 0, gl.RGBA, 1, 1, 0, gl.RGBA, gl.UNSIGNED_BYTE, new Uint8Array([7, 13, 18, 255]));

  const seedImage = new Image();
  seedImage.decoding = "async";
  seedImage.addEventListener("load", () => {
    gl.bindTexture(gl.TEXTURE_2D, seedTexture);
    gl.pixelStorei(gl.UNPACK_FLIP_Y_WEBGL, true);
    gl.texImage2D(gl.TEXTURE_2D, 0, gl.RGBA, gl.RGBA, gl.UNSIGNED_BYTE, seedImage);
    seedReady = true;
    if (activeScene === "mosaic") beginMosaicEvolution();
    renderOnce();
  });
  seedImage.src = mosaicSeedDataUrl64;

  function beginMosaicEvolution() {
    if (!seedReady || reduceMotion.matches) return;
    clearTimeout(evolutionTimer);
    evolutionTimer = setTimeout(() => {
      if (activeScene === "mosaic") mosaicBackdrop?.classList.add("is-evolving");
    }, 850);
  }

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
    const elapsed = activeScene === "mosaic" ? now - sceneStartedAt : now - startedAt;
    gl.uniform1f(active.time, reduceMotion.matches ? 0 : elapsed);
    if (active.seed) {
      gl.activeTexture(gl.TEXTURE0);
      gl.bindTexture(gl.TEXTURE_2D, seedTexture);
      gl.uniform1i(active.seed, 0);
    }
    gl.drawArrays(gl.TRIANGLES, 0, 3);
    frame = visible && !reduceMotion.matches ? requestAnimationFrame(draw) : 0;
  }

  function renderOnce() {
    if (frame) cancelAnimationFrame(frame);
    frame = requestAnimationFrame(draw);
  }

  sceneButtons.forEach((button) => button.addEventListener("click", () => {
    activeScene = button.dataset.heroScene;
    sceneStartedAt = performance.now();
    clearTimeout(evolutionTimer);
    mosaicBackdrop?.classList.remove("is-evolving");
    mosaicBackdrop?.classList.toggle("is-active", activeScene === "mosaic");
    if (activeScene === "mosaic") beginMosaicEvolution();
    canvas.setAttribute("aria-label", activeScene === "columns"
      ? "Animated luminous Roman cathedral nave"
      : "Animated Greco-Roman bathhouse mosaic mural");
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

const canvas = document.querySelector("[data-hero-canvas]");
const sceneButtons = [...document.querySelectorAll("[data-hero-scene]")];
const mosaicBackdrop = document.querySelector("[data-hero-mosaic]");
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

void main() {
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
    mosaicBackdrop?.classList.toggle("is-active", activeScene === "mosaic");
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

// Generates the six motif master SVG sources (3 motifs x light/dark) into
// assets/motifs/source/. The artwork is parametric: gothic, biological curves
// in brushed black/white metal with one restrained signal accent.
// Run: node scripts/generate-motif-sources.mjs
import { mkdir, writeFile } from "node:fs/promises";
import { dirname, join } from "node:path";
import { fileURLToPath } from "node:url";

const outDir = join(dirname(fileURLToPath(import.meta.url)), "..", "assets", "motifs", "source");
const S = 4096;
const rad = (deg) => (deg * Math.PI) / 180;
const pt = (cx, cy, r, deg) => [cx + r * Math.cos(rad(deg)), cy + r * Math.sin(rad(deg))];
const f = (n) => n.toFixed(1);

const palettes = {
  dark: {
    ground: "#050608",
    metalHi: "#4a525d",
    metalMid: "#262b33",
    metalLo: "#101318",
    groove: "#040507",
    hairline: "rgba(235,240,248,.07)",
    seamGlow: "#6fb1ff",
    seamCore: "#bfe0ff",
    vignetteInner: "rgba(0,0,0,0)",
    vignetteOuter: "rgba(0,0,0,.5)",
    brushOpacity: ".16"
  },
  light: {
    ground: "#f4f6f8",
    metalHi: "#fbfcfd",
    metalMid: "#c6ccd3",
    metalLo: "#98a0a9",
    groove: "#6e757e",
    hairline: "rgba(20,26,34,.08)",
    seamGlow: "#2f7cff",
    seamCore: "#9cc2ff",
    vignetteInner: "rgba(255,255,255,0)",
    vignetteOuter: "rgba(120,130,142,.25)",
    brushOpacity: ".10"
  }
};

// Shared defs: brushed-metal streak texture, soft glows, corner vignette.
const defs = (p, seed) => `
  <defs>
    <filter id="brush" x="-5%" y="-5%" width="110%" height="110%">
      <feTurbulence type="fractalNoise" baseFrequency="0.0016 0.42" numOctaves="3" seed="${seed}" result="n"/>
      <feColorMatrix in="n" type="matrix" values="0 0 0 0 0.5 0 0 0 0 0.5 0 0 0 0 0.5 0 0 0 0.9 0"/>
    </filter>
    <filter id="glow" x="-120%" y="-120%" width="340%" height="340%">
      <feGaussianBlur stdDeviation="26"/>
    </filter>
    <filter id="glowWide" x="-120%" y="-120%" width="340%" height="340%">
      <feGaussianBlur stdDeviation="60"/>
    </filter>
    <radialGradient id="vignette" cx=".5" cy=".46" r=".78">
      <stop offset=".55" stop-color="${p.vignetteInner}"/>
      <stop offset="1" stop-color="${p.vignetteOuter}"/>
    </radialGradient>
    <linearGradient id="plateSweep" x1="0" y1="0" x2="1" y2="1">
      <stop offset="0" stop-color="${p.metalHi}"/>
      <stop offset=".45" stop-color="${p.metalMid}"/>
      <stop offset="1" stop-color="${p.metalLo}"/>
    </linearGradient>
  </defs>`;

const finish = (p, body) => `<?xml version="1.0" encoding="UTF-8"?>
<svg xmlns="http://www.w3.org/2000/svg" width="${S}" height="${S}" viewBox="0 0 ${S} ${S}">
  <rect width="${S}" height="${S}" fill="${p.ground}"/>
  ${body}
  <rect width="${S}" height="${S}" filter="url(#brush)" opacity="${p.brushOpacity}" style="mix-blend-mode:overlay"/>
  <rect width="${S}" height="${S}" fill="url(#vignette)"/>
</svg>
`;

// ---------------------------------------------------------------- rack
// Ribcage vault: crescent ribs from an off-canvas left centre, vertebra
// bulged, receding across the canvas, one signal node.
const rack = (p) => {
  const cx = -S * 0.3, cy = S * 0.52;
  const count = 16;
  const a0 = -42, a1 = 42;
  const gradients = [];
  const shapes = [];
  for (let i = 0; i < count; i++) {
    const t = i / (count - 1);
    const r = 1500 + t * 4100;
    const half = 88 * (1 - t * 0.22);
    const seg = 48;
    const outer = [], inner = [];
    for (let k = 0; k <= seg; k++) {
      const a = a0 + ((a1 - a0) * k) / seg;
      const bulge = 1 + 0.22 * Math.cos(rad(a * 2.6)) + 0.06 * Math.sin(rad(a * 7 + i * 40));
      outer.push(pt(cx, cy, r + half * bulge, a));
      inner.push(pt(cx, cy, r - half * bulge, a));
    }
    const d =
      `M ${f(outer[0][0])} ${f(outer[0][1])} ` +
      outer.slice(1).map(([x, y]) => `L ${f(x)} ${f(y)}`).join(" ") +
      ` L ${f(inner[seg][0])} ${f(inner[seg][1])} ` +
      inner.slice(0, seg).reverse().map(([x, y]) => `L ${f(x)} ${f(y)}`).join(" ") +
      " Z";
    const lit = 1 - t * 0.5;
    gradients.push(`
    <linearGradient id="rib${i}" x1="0" y1="0" x2="1" y2="0">
      <stop offset="0" stop-color="${p.groove}"/>
      <stop offset=".34" stop-color="${p.metalMid}"/>
      <stop offset=".5" stop-color="${p.metalHi}" stop-opacity="${lit}"/>
      <stop offset=".68" stop-color="${p.metalMid}"/>
      <stop offset="1" stop-color="${p.groove}"/>
    </linearGradient>`);
    shapes.push(`<path d="${d}" fill="url(#rib${i})"/>`);
    const [hx0, hy0] = pt(cx, cy, r, a0), [hx1, hy1] = pt(cx, cy, r, a1);
    shapes.push(`<path d="M ${f(hx0)} ${f(hy0)} A ${f(r)} ${f(r)} 0 0 1 ${f(hx1)} ${f(hy1)}" fill="none" stroke="${p.hairline}" stroke-width="3"/>`);
  }
  const [nx, ny] = pt(cx, cy, 1500 + (5 / (count - 1)) * 4100, -12);
  shapes.push(`
    <circle cx="${f(nx)}" cy="${f(ny)}" r="52" fill="${p.seamGlow}" filter="url(#glowWide)" opacity=".8"/>
    <circle cx="${f(nx)}" cy="${f(ny)}" r="16" fill="${p.seamCore}"/>
    <circle cx="${f(nx)}" cy="${f(ny)}" r="32" fill="none" stroke="${p.seamGlow}" stroke-width="4" opacity=".85"/>`);
  return `<defs>${gradients.join("")}</defs>\n  ${shapes.join("\n  ")}`;
};

// ------------------------------------------------------------- aperture
// Organic iris: sickle blades spiralling into a dark core with a signal ring.
const aperture = (p) => {
  const cx = S / 2, cy = S / 2;
  const blades = 12;
  const gradients = [];
  const shapes = [];
  const r0 = 360, r1 = 1620;
  // sickle blade: pointed tip spiralling into the core, broad swept root
  const blade = `
      M ${f(r1 * 1.00)} ${f(r1 * 0.34)}
      C ${f(r1 * 0.72)} ${f(r1 * 0.02)}, ${f(r1 * 0.46)} ${f(-r1 * 0.42)}, ${f(r0 * 1.35)} ${f(-r0 * 0.95)}
      C ${f(r0 * 0.98)} ${f(-r0 * 0.66)}, ${f(r0 * 0.98)} ${f(-r0 * 0.28)}, ${f(r0 * 1.28)} ${f(-r0 * 0.08)}
      C ${f(r0 * 1.75)} ${f(r0 * 0.22)}, ${f(r1 * 0.42)} ${f(r0 * 0.92)}, ${f(r1 * 0.68)} ${f(r1 * 0.52)}
      C ${f(r1 * 0.82)} ${f(r1 * 0.50)}, ${f(r1 * 0.94)} ${f(r1 * 0.44)}, ${f(r1 * 1.00)} ${f(r1 * 0.34)} Z`;
  for (let i = blades - 1; i >= 0; i--) {
    const rot = ((360 / blades) * i).toFixed(2);
    const shade = (1 - (i / blades) * 0.45).toFixed(3);
    gradients.push(`
    <linearGradient id="blade${i}" x1="0" y1="0" x2=".9" y2="-.5" gradientUnits="objectBoundingBox">
      <stop offset="0" stop-color="${p.groove}"/>
      <stop offset=".45" stop-color="${p.metalMid}"/>
      <stop offset=".8" stop-color="${p.metalHi}" stop-opacity="${shade}"/>
      <stop offset="1" stop-color="${p.metalLo}"/>
    </linearGradient>`);
    shapes.push(`<g transform="rotate(${rot} ${cx} ${cy}) translate(${cx} ${cy})"><path d="${blade}" fill="url(#blade${i})" stroke="${p.groove}" stroke-width="8"/></g>`);
  }
  for (let k = 0; k < 4; k++) {
    const r = 1750 + k * 210;
    const fa0 = -30 + k * 24, fa1 = fa0 + 205;
    const [x0, y0] = pt(cx, cy, r, fa0), [x1, y1] = pt(cx, cy, r, fa1);
    shapes.push(`<path d="M ${f(x0)} ${f(y0)} A ${r} ${r} 0 1 1 ${f(x1)} ${f(y1)}" fill="none" stroke="${p.hairline}" stroke-width="${10 - k * 2}"/>`);
  }
  shapes.push(`
    <circle cx="${cx}" cy="${cy}" r="356" fill="${p.seamGlow}" filter="url(#glowWide)" opacity=".55"/>
    <circle cx="${cx}" cy="${cy}" r="338" fill="none" stroke="${p.seamGlow}" stroke-width="10" opacity=".9"/>
    <circle cx="${cx}" cy="${cy}" r="352" fill="none" stroke="${p.seamCore}" stroke-width="3"/>
    <circle cx="${cx}" cy="${cy}" r="326" fill="${p.ground}"/>`);
  return `<defs>${gradients.join("")}</defs>\n  ${shapes.join("\n  ")}`;
};

// ----------------------------------------------------------------- edge
// Carapace seam: two plates meeting along one long biological S-curve.
const edge = (p) => {
  const seam = `M 4360 -120 C 3050 620, 2450 1350, 2380 2000 C 2310 2620, 1450 2760, -160 3960`;
  const seamReversed = `M -160 3960 C 1450 2760, 2310 2620, 2380 2000 C 2450 1350, 3050 620, 4360 -120`;
  const a = `${seam} L 4400 4400 L -200 4400 L -200 3960 Z`;
  const b = `${seamReversed} L 4400 -200 L -200 -200 L -160 3960 Z`;
  const shapes = [];
  shapes.push(`<path d="${b}" fill="${p.metalMid}"/><path d="${b}" fill="url(#plateSweep)" opacity=".7"/>`);
  shapes.push(`<path d="${a}" fill="${p.metalLo}"/><path d="${a}" fill="url(#plateSweep)" opacity=".3"/>`);
  const offsets = [[-90, 8], [-210, 6], [-380, 5], [110, 7], [250, 5], [430, 4]];
  for (const [dy, w] of offsets) {
    shapes.push(`<g transform="translate(${(dy * 0.55).toFixed(0)} ${dy})"><path d="${seam}" fill="none" stroke="${p.hairline}" stroke-width="${w}"/></g>`);
  }
  shapes.push(`
    <path d="${seam}" fill="none" stroke="${p.seamGlow}" stroke-width="60" opacity=".55" filter="url(#glow)"/>
    <path d="${seam}" fill="none" stroke="${p.seamGlow}" stroke-width="9" opacity=".95"/>
    <path d="${seam}" fill="none" stroke="${p.seamCore}" stroke-width="3"/>`);
  return shapes.join("\n  ");
};

const builders = { rack, aperture, edge };
await mkdir(outDir, { recursive: true });
for (const [name, build] of Object.entries(builders)) {
  for (const [theme, p] of Object.entries(palettes)) {
    const svg = finish(p, `${defs(p, name.length * 13 + (theme === "dark" ? 5 : 29))}\n  ${build(p)}`);
    await writeFile(join(outDir, `${name}-${theme}.svg`), svg);
  }
}
console.log(`Wrote 6 motif sources to ${outDir}`);

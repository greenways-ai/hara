/* Hara grid game — lightweight 4-player Tron.
   Cycles spawn in random corners, race to the diagonally opposite corner,
   fence off territory there, and score by survival plus fenced area.
   Tails are capped to 30 segments and dead walls fade quickly so the arena
   never clogs. Positions are kept in normalized arena space (k, t*T_SCALE)
   and projected with a fan geometry. */
(() => {
  const canvas = document.querySelector('[data-hara-component="game"]');
  if (!canvas) return;
  if (window.matchMedia('(prefers-reduced-motion: reduce)').matches) return;
  const ctx = canvas.getContext('2d');
  if (!ctx) return;

  /* cover-fit of the 1440x810 scene viewBox */
  const VB_W = 1440, VB_H = 810;
  let vw = 0, vh = 0, dpr = 1, scale = 1, offX = 0, offY = 0;

  const resize = () => {
    vw = window.innerWidth;
    vh = window.innerHeight;
    dpr = Math.min(2, window.devicePixelRatio || 1);
    canvas.width = Math.round(vw * dpr);
    canvas.height = Math.round(vh * dpr);
    scale = Math.max(vw / VB_W, vh / VB_H);
    offX = (vw - VB_W * scale) / 2;
    offY = (vh - VB_H * scale) / 2;
  };
  window.addEventListener('resize', resize);
  resize();

  const project = (k, t) => {
    const x = 720 + k * (36 + 304 * t);
    const y = -160 + 970 * t;
    return [offX + x * scale, offY + y * scale];
  };

  /* ----------------------------------------------------------------------
     arena and movement
     ---------------------------------------------------------------------- */
  const T_SCALE = 28;
  const X_MIN = -2.2, X_MAX = 2.2;
  const Y_MIN = 0.30 * T_SCALE, Y_MAX = 0.90 * T_SCALE;
  const SPEED_K = 0.55;
  const SPEED_T = 0.085;
  const BOOST_MAX = 2.4;
  const BOOST_GAIN = 0.25;
  const LOOK_T = 0.5;
  const AI_MS = 40;
  const TAIL_CAP = 30;
  const MAX_TAIL_LENGTH = 16; /* normalized units */
  const TAIL_LIFE = 8; /* seconds before a trail segment fades away */
  const DEBRIS_LIFE = 5;
  const SCORE_SURVIVAL_PER_S = 0.5;
  const SCORE_CENTER_PER_S = 8;
  const SCORE_INVASION_PER_S = -6;

  const COLORS = ['#41f5e4', '#ff2e88', '#9c7bff', '#f5d742'];
  const DIRS = [[1, 0], [-1, 0], [0, 1], [0, -1]];

  /* four corners in normalized arena space */
  const CORNER_W = (X_MAX - X_MIN) * 0.38;
  const CORNER_H = (Y_MAX - Y_MIN) * 0.38;
  const CORNERS = [
    { xMin: X_MIN, xMax: X_MIN + CORNER_W, yMin: Y_MIN, yMax: Y_MIN + CORNER_H },          // 0 TL
    { xMin: X_MAX - CORNER_W, xMax: X_MAX, yMin: Y_MIN, yMax: Y_MIN + CORNER_H },          // 1 TR
    { xMin: X_MIN, xMax: X_MIN + CORNER_W, yMax: Y_MAX, yMin: Y_MAX - CORNER_H },          // 2 BL
    { xMin: X_MAX - CORNER_W, xMax: X_MAX, yMax: Y_MAX, yMin: Y_MAX - CORNER_H }           // 3 BR
  ];
  const DIAGONAL = { 0: 3, 3: 0, 1: 2, 2: 1 };
  const CENTER = { xMin: -0.6, xMax: 0.6, yMin: (Y_MIN + Y_MAX) / 2 - 2.5, yMax: (Y_MIN + Y_MAX) / 2 + 2.5 };

  const rand = (a, b) => a + Math.random() * (b - a);
  const clamp = (v, lo, hi) => Math.max(lo, Math.min(hi, v));
  const outOfBounds = (x, y) => x < X_MIN || x > X_MAX || y < Y_MIN || y > Y_MAX;

  /* ----------------------------------------------------------------------
     geometry: segment intersection for real collision
     ---------------------------------------------------------------------- */
  const orient = (a, b, c) =>
    (b[0] - a[0]) * (c[1] - a[1]) - (b[1] - a[1]) * (c[0] - a[0]);

  const onSeg = (a, b, c) =>
    Math.min(a[0], b[0]) - 1e-9 <= c[0] && c[0] <= Math.max(a[0], b[0]) + 1e-9 &&
    Math.min(a[1], b[1]) - 1e-9 <= c[1] && c[1] <= Math.max(a[1], b[1]) + 1e-9;

  const segIntersect = (a, b, c, d) => {
    const o1 = orient(a, b, c);
    const o2 = orient(a, b, d);
    const o3 = orient(c, d, a);
    const o4 = orient(c, d, b);
    if (o1 * o2 < 0 && o3 * o4 < 0) return true;
    if (Math.abs(o1) < 1e-9 && onSeg(a, b, c)) return true;
    if (Math.abs(o2) < 1e-9 && onSeg(a, b, d)) return true;
    if (Math.abs(o3) < 1e-9 && onSeg(c, d, a)) return true;
    if (Math.abs(o4) < 1e-9 && onSeg(c, d, b)) return true;
    return false;
  };

  const lineIntersect = (a, b, c, d) => {
    const d1x = b[0] - a[0], d1y = b[1] - a[1];
    const d2x = d[0] - c[0], d2y = d[1] - c[1];
    const denom = d1x * d2y - d1y * d2x;
    if (Math.abs(denom) < 1e-9) return null;
    const t = ((c[0] - a[0]) * d2y - (c[1] - a[1]) * d2x) / denom;
    return [a[0] + t * d1x, a[1] + t * d1y];
  };

  const segLength = (s) => Math.hypot(s.b[0] - s.a[0], s.b[1] - s.a[1]);

  /* ----------------------------------------------------------------------
     occupancy grid for pathfinding
     ---------------------------------------------------------------------- */
  const CELL = 0.15;
  const GX = Math.ceil((X_MAX - X_MIN) / CELL);
  const GY = Math.ceil((Y_MAX - Y_MIN) / CELL);
  const FLOOD_CAP = 500;
  const MIN_SPACE = 6;

  const cellOf = (x, y) => {
    const cx = Math.floor((x - X_MIN) / CELL);
    const cy = Math.floor((y - Y_MIN) / CELL);
    return (cx < 0 || cx >= GX || cy < 0 || cy >= GY) ? -1 : cy * GX + cx;
  };

  const mark = (g, x, y) => {
    const cx = Math.floor((x - X_MIN) / CELL);
    const cy = Math.floor((y - Y_MIN) / CELL);
    for (let dy = -1; dy <= 1; dy++) {
      for (let dx = -1; dx <= 1; dx++) {
        if (Math.abs(dx) + Math.abs(dy) > 1) continue;
        const nx = cx + dx, ny = cy + dy;
        if (nx >= 0 && nx < GX && ny >= 0 && ny < GY) g[ny * GX + nx] = 1;
      }
    }
  };

  const markSeg = (g, ax, ay, bx, by) => {
    const len = Math.hypot(bx - ax, by - ay);
    const steps = Math.max(1, Math.ceil(len / (CELL * 0.5)));
    for (let i = 0; i <= steps; i++) {
      mark(g, ax + ((bx - ax) * i) / steps, ay + ((by - ay) * i) / steps);
    }
  };

  const markSegments = (g, segs) => {
    for (const s of segs) markSeg(g, s.a[0], s.a[1], s.b[0], s.b[1]);
  };

  const buildGrid = (self) => {
    const g = new Uint8Array(GX * GY);
    for (const c of cycles) {
      if (!c.alive && c !== self) continue;
      if (c === self) {
        /* exclude current run and the segment it is extending from */
        const segs = c.segments;
        for (let i = 0; i < segs.length - 1; i++) markSeg(g, segs[i].a[0], segs[i].a[1], segs[i].b[0], segs[i].b[1]);
      } else {
        markSegments(g, c.segments);
        if (c.alive) markSeg(g, c.lastTurn[0], c.lastTurn[1], c.x, c.y);
      }
    }
    for (const d of debris) markSegments(g, d.segments);
    return g;
  };

  const flood = (g, start, cap) => {
    if (start < 0 || g[start]) return 0;
    const seen = new Uint8Array(g);
    const stack = [start];
    seen[start] = 1;
    let count = 0;
    while (stack.length && count < cap) {
      const i = stack.pop();
      count++;
      const cx = i % GX, cy = (i - cx) / GX;
      if (cx > 0 && !seen[i - 1]) { seen[i - 1] = 1; stack.push(i - 1); }
      if (cx < GX - 1 && !seen[i + 1]) { seen[i + 1] = 1; stack.push(i + 1); }
      if (cy > 0 && !seen[i - GX]) { seen[i - GX] = 1; stack.push(i - GX); }
      if (cy < GY - 1 && !seen[i + GX]) { seen[i + GX] = 1; stack.push(i + GX); }
    }
    return count;
  };

  const rayBlocked = (g, x1, y1, x2, y2) => {
    const len = Math.hypot(x2 - x1, y2 - y1);
    const steps = Math.max(1, Math.ceil(len / (CELL * 0.5)));
    for (let i = 1; i <= steps; i++) {
      const idx = cellOf(x1 + ((x2 - x1) * i) / steps, y1 + ((y2 - y1) * i) / steps);
      if (idx < 0 || g[idx]) return true;
    }
    return false;
  };

  /* ----------------------------------------------------------------------
     game state
     ---------------------------------------------------------------------- */
  const debris = [];
  const turnEvents = [];

  const inZone = (x, y, z) => x >= z.xMin && x <= z.xMax && y >= z.yMin && y <= z.yMax;

  const shuffleCorners = () => {
    const order = [0, 1, 2, 3].sort(() => Math.random() - 0.5);
    cycles.forEach((c, i) => {
      c.corner = order[i];
      c.targetCorner = DIAGONAL[c.corner];
    });
  };

  const pickCornerDir = (c, g) => {
    const zone = CORNERS[c.corner];
    const cx = (zone.xMin + zone.xMax) / 2;
    const cy = (zone.yMin + zone.yMax) / 2;
    let best = DIRS[0], bestSpace = -1;
    for (const d of DIRS) {
      const nx = c.x + d[0] * SPEED_K * 0.6;
      const ny = c.y + d[1] * SPEED_T * T_SCALE * 0.6;
      if (outOfBounds(nx, ny)) continue;
      const idx = cellOf(nx, ny);
      const space = idx < 0 ? 0 : flood(g, idx, 200);
      /* prefer direction away from corner center so they leave the corner */
      const away = d[0] * (c.x - cx) + d[1] * (c.y - cy);
      const score = space + away * 0.5;
      if (score > bestSpace) { bestSpace = score; best = d; }
    }
    return best.slice();
  };

  const spawn = (colorIndex) => ({
    color: COLORS[colorIndex],
    colorIndex,
    corner: 0,
    targetCorner: 0,
    x: 0, y: 0,
    lastTurn: [0, 0],
    dir: [1, 0],
    segments: [],
    boost: 0,
    alive: true,
    respawnIn: 0,
    score: 0,
    aggression: Math.random() < 0.5 ? 0.8 : 0.2,
  });

  const cycles = COLORS.map((_, i) => spawn(i));

  const placeInCorner = (c) => {
    const z = CORNERS[c.corner];
    c.x = rand(z.xMin + 0.3, z.xMax - 0.3);
    c.y = rand(z.yMin + 0.3, z.yMax - 0.3);
    c.lastTurn = [c.x, c.y];
    c.segments = [];
    c.boost = 0;
    c.alive = true;
    const g = buildGrid(null);
    c.dir = pickCornerDir(c, g);
  };

  const tailLength = (c) => {
    let len = 0;
    for (const s of c.segments) len += segLength(s);
    return len;
  };

  const trimTail = (c, nowS) => {
    while (c.segments.length > 1 && tailLength(c) > MAX_TAIL_LENGTH) {
      c.segments.shift();
    }
    while (c.segments.length > TAIL_CAP) {
      c.segments.shift();
    }
    while (c.segments.length && c.segments[0].born < nowS - TAIL_LIFE) {
      c.segments.shift();
    }
  };

  const commitSegment = (c, nowS) => {
    const seg = {
      a: [c.lastTurn[0], c.lastTurn[1]],
      b: [c.x, c.y],
      born: nowS
    };
    c.segments.push(seg);
    trimTail(c, nowS);
    c.lastTurn = [c.x, c.y];
  };

  const crash = (c, nowS) => {
    c.alive = false;
    c.respawnIn = rand(1.0, 2.2);
    /* finalize the run up to the crash point as a segment */
    if (Math.hypot(c.x - c.lastTurn[0], c.y - c.lastTurn[1]) > 0.01) {
      c.segments.push({
        a: [c.lastTurn[0], c.lastTurn[1]],
        b: [c.x, c.y],
        born: nowS
      });
      trimTail(c, nowS);
    }
    if (c.segments.length) debris.push({ color: c.color, segments: c.segments, born: nowS });
  };

  const respawn = (c) => {
    /* pick a random corner, preferring one not currently occupied by a live cycle */
    const occupied = new Set(cycles.filter((o) => o.alive && o !== c).map((o) => o.corner));
    const free = [0, 1, 2, 3].filter((i) => !occupied.has(i));
    const pool = free.length ? free : [0, 1, 2, 3];
    c.corner = pool[Math.floor(Math.random() * pool.length)];
    c.targetCorner = DIAGONAL[c.corner];
    placeInCorner(c);
  };

  /* ----------------------------------------------------------------------
     collision detection
     ---------------------------------------------------------------------- */
  const checkCollision = (c, px, py) => {
    const segA = [px, py];
    const segB = [c.x, c.y];
    for (const other of cycles) {
      const isSelf = other === c;
      const segs = other.segments;
      const limit = isSelf ? Math.max(0, segs.length - 1) : segs.length;
      for (let i = 0; i < limit; i++) {
        if (segIntersect(segA, segB, segs[i].a, segs[i].b)) {
          return lineIntersect(segA, segB, segs[i].a, segs[i].b) ||
            [(segA[0] + segB[0]) / 2, (segA[1] + segB[1]) / 2];
        }
      }
      if (!isSelf && other.alive) {
        if (segIntersect(segA, segB, other.lastTurn, [other.x, other.y])) {
          return lineIntersect(segA, segB, other.lastTurn, [other.x, other.y]) ||
            [(segA[0] + segB[0]) / 2, (segA[1] + segB[1]) / 2];
        }
      }
    }
    for (const d of debris) {
      for (const s of d.segments) {
        if (segIntersect(segA, segB, s.a, s.b)) {
          return lineIntersect(segA, segB, s.a, s.b) ||
            [(segA[0] + segB[0]) / 2, (segA[1] + segB[1]) / 2];
        }
      }
    }
    return null;
  };

  /* ----------------------------------------------------------------------
     AI
     ---------------------------------------------------------------------- */
  const pathCrossesTurn = (c, d, e) => {
    const mult = 1 + c.boost * BOOST_GAIN;
    const nx = c.x + d[0] * SPEED_K * mult * LOOK_T;
    const ny = c.y + d[1] * SPEED_T * T_SCALE * mult * LOOK_T;
    const tx = e.x + e.newDir[0] * SPEED_K * mult * LOOK_T;
    const ty = e.y + e.newDir[1] * SPEED_T * T_SCALE * mult * LOOK_T;
    return segIntersect([c.x, c.y], [nx, ny], [e.x, e.y], [tx, ty]);
  };

  const findNearestOpponent = (c) => {
    let best = null, bestD = Infinity;
    for (const o of cycles) {
      if (o === c || !o.alive) continue;
      const d = Math.hypot(o.x - c.x, o.y - c.y);
      if (d < bestD) { bestD = d; best = o; }
    }
    return best;
  };

  const think = (c, nowS) => {
    if (!c.alive) return;
    const g = buildGrid(c);
    const [dx, dy] = c.dir;
    const cands = [[dx, dy], [dy, -dx], [-dy, dx]];
    const prey = findNearestOpponent(c);
    const recentTurns = turnEvents.filter((e) => e.time > nowS - 1.0 && e.cycle !== c);

    /* choose objective: defend target quadrant if invaded, otherwise race to center */
    const tz = CORNERS[c.targetCorner];
    let invader = null, invaderD = Infinity;
    for (const o of cycles) {
      if (o === c || !o.alive) continue;
      if (inZone(o.x, o.y, tz)) {
        const d = Math.hypot(o.x - c.x, o.y - c.y);
        if (d < invaderD) { invaderD = d; invader = o; }
      }
    }
    const target = invader || CENTER;
    const tx = (target.xMin + target.xMax) / 2;
    const ty = (target.yMin + target.yMax) / 2;

    let best = null, bestScore = -Infinity;
    for (const d of cands) {
      const mult = 1 + c.boost * BOOST_GAIN;
      const nx = c.x + d[0] * SPEED_K * mult * LOOK_T;
      const ny = c.y + d[1] * SPEED_T * T_SCALE * mult * LOOK_T;
      if (outOfBounds(nx, ny) || rayBlocked(g, c.x, c.y, nx, ny)) continue;
      const idx = cellOf(nx, ny);
      const space = idx < 0 ? 0 : flood(g, idx, FLOOD_CAP);
      if (space < MIN_SPACE) continue;

      const before = Math.hypot(tx - c.x, ty - c.y);
      const after = Math.hypot(tx - nx, ty - ny);
      let score = (before - after) * (invader ? 18 : 15);
      score += space * 0.5;
      if (d[0] === dx && d[1] === dy) score += 8;
      if (space < 15) score -= 20;

      /* once near the center, prefer staying there */
      if (!invader && inZone(nx, ny, CENTER)) score += 15;

      for (const e of recentTurns) {
        if (pathCrossesTurn(c, d, e)) score -= 30;
      }

      if (c.aggression > 0.5 && prey) {
        score += Math.max(0, 8 - Math.hypot(nx - prey.x, ny - prey.y)) * 2;
      }

      score += Math.random() * 2;
      if (score > bestScore) { bestScore = score; best = d; }
    }

    if (!best) {
      let fallback = null, fallbackSpace = -1;
      for (const d of cands) {
        const nx = c.x + d[0] * SPEED_K * LOOK_T;
        const ny = c.y + d[1] * SPEED_T * T_SCALE * LOOK_T;
        if (outOfBounds(nx, ny) || rayBlocked(g, c.x, c.y, nx, ny)) continue;
        const idx = cellOf(nx, ny);
        const space = idx < 0 ? 0 : flood(g, idx, FLOOD_CAP);
        if (space > fallbackSpace) { fallbackSpace = space; fallback = d; }
      }
      best = fallback;
    }

    if (!best) return;

    if (best[0] !== dx || best[1] !== dy) {
      commitSegment(c, nowS);
      const oldDir = c.dir.slice();
      c.dir = best.slice();
      c.boost = 0;
      turnEvents.push({
        cycle: c,
        x: c.x,
        y: c.y,
        oldDir,
        newDir: c.dir.slice(),
        time: nowS
      });
    }
  };

  /* ----------------------------------------------------------------------
     update loop
     ---------------------------------------------------------------------- */
  const update = (dt, nowS) => {
    while (turnEvents.length && turnEvents[0].time < nowS - 1.5) turnEvents.shift();
    while (debris.length && debris[0].born < nowS - DEBRIS_LIFE) debris.shift();

    for (const c of cycles) {
      if (!c.alive) {
        c.respawnIn -= dt;
        if (c.respawnIn <= 0) respawn(c);
        continue;
      }
      c.score += SCORE_SURVIVAL_PER_S * dt;
      if (inZone(c.x, c.y, CENTER)) c.score += SCORE_CENTER_PER_S * dt;
      const tz = CORNERS[c.targetCorner];
      for (const o of cycles) {
        if (o !== c && o.alive && inZone(o.x, o.y, tz)) {
          c.score += SCORE_INVASION_PER_S * dt;
        }
      }
      trimTail(c, nowS);
      c.boost = Math.min(BOOST_MAX, c.boost + dt);
      const mult = 1 + c.boost * BOOST_GAIN;
      const px = c.x, py = c.y;
      c.x += c.dir[0] * SPEED_K * mult * dt;
      c.y += c.dir[1] * SPEED_T * T_SCALE * mult * dt;

      if (outOfBounds(c.x, c.y)) {
        c.x = clamp(c.x, X_MIN, X_MAX);
        c.y = clamp(c.y, Y_MIN, Y_MAX);
        crash(c, nowS);
        continue;
      }

      const hit = checkCollision(c, px, py);
      if (hit) {
        c.x = hit[0];
        c.y = hit[1];
        crash(c, nowS);
      }
    }
  };

  /* ----------------------------------------------------------------------
     rendering
     ---------------------------------------------------------------------- */
  const heightAt = (y) => {
    const t = y / T_SCALE;
    return (9 + 24 * t) * scale;
  };

  const drawCornerGlow = (z, color, now) => {
    const [x1, y1] = project(z.xMin, z.yMin / T_SCALE);
    const [x2, y2] = project(z.xMax, z.yMax / T_SCALE);
    const pulse = 0.35 + 0.15 * Math.sin(now / 800);
    ctx.strokeStyle = color;
    ctx.globalAlpha = pulse;
    ctx.lineWidth = 1.5;
    ctx.strokeRect(x1, y2, x2 - x1, y1 - y2);
    ctx.globalAlpha = 1;
  };

  const draw = (now) => {
    ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
    ctx.clearRect(0, 0, vw, vh);
    ctx.lineJoin = 'miter';
    const nowS = now / 1000;

    /* target corner glows and shared center glow */
    drawCornerGlow(CENTER, '#8ffff2', now);
    for (const c of cycles) {
      if (!c.alive) continue;
      drawCornerGlow(CORNERS[c.targetCorner], c.color, now);
    }

    const segs = [];
    for (const d of debris) {
      const fade = Math.max(0, 1 - (nowS - d.born) / DEBRIS_LIFE);
      for (const s of d.segments) {
        segs.push({ a: s.a, b: s.b, color: d.color, alpha: 0.65 * fade });
      }
    }
    for (const c of cycles) {
      for (const s of c.segments) {
        const fade = Math.max(0, 1 - (nowS - s.born) / TAIL_LIFE);
        segs.push({ a: s.a, b: s.b, color: c.color, alpha: fade });
      }
      if (c.alive) {
        segs.push({ a: c.lastTurn, b: [c.x, c.y], color: c.color, alpha: 1 });
      }
    }
    segs.sort((s1, s2) => (s1.a[1] + s1.b[1]) - (s2.a[1] + s2.b[1]));

    for (const s of segs) {
      const [x1, y1] = project(s.a[0], s.a[1] / T_SCALE);
      const [x2, y2] = project(s.b[0], s.b[1] / T_SCALE);
      const h1 = heightAt(s.a[1]);
      const h2 = heightAt(s.b[1]);
      ctx.beginPath();
      ctx.moveTo(x1, y1);
      ctx.lineTo(x2, y2);
      ctx.lineTo(x2, y2 - h2);
      ctx.lineTo(x1, y1 - h1);
      ctx.closePath();
      ctx.fillStyle = s.color;
      ctx.globalAlpha = 0.15 * s.alpha;
      ctx.fill();
      ctx.beginPath();
      ctx.moveTo(x1, y1 - h1);
      ctx.lineTo(x2, y2 - h2);
      ctx.strokeStyle = s.color;
      ctx.globalAlpha = 0.2 * s.alpha;
      ctx.lineWidth = 8;
      ctx.stroke();
      ctx.globalAlpha = 0.8 * s.alpha;
      ctx.lineWidth = 2;
      ctx.stroke();
    }

    /* heads */
    cycles.forEach((c, ci) => {
      if (!c.alive) return;
      const [hx, hy] = project(c.x, c.y / T_SCALE);
      const top = hy - heightAt(c.y);
      const phase = ((now / 1000) * 0.9 + ci * 0.25) % 1;
      ctx.strokeStyle = c.color;
      ctx.globalAlpha = 0.55 * (1 - phase);
      ctx.lineWidth = 1.2;
      ctx.beginPath();
      ctx.arc(hx, top, 4 + phase * 15, 0, Math.PI * 2);
      ctx.stroke();
      ctx.globalAlpha = 1;
      ctx.shadowColor = c.color;
      ctx.shadowBlur = 14;
      ctx.fillStyle = '#eafcff';
      ctx.beginPath();
      ctx.arc(hx, top, 3.2, 0, Math.PI * 2);
      ctx.fill();
      ctx.shadowBlur = 0;
    });
    ctx.globalAlpha = 1;
  };

  /* ----------------------------------------------------------------------
     round lifecycle
     ---------------------------------------------------------------------- */
  const startRound = () => {
    shuffleCorners();
    const colorOrder = [0, 1, 2, 3].sort(() => Math.random() - 0.5);
    debris.length = 0;
    turnEvents.length = 0;
    cycles.forEach((c, i) => {
      c.colorIndex = colorOrder[i];
      c.color = COLORS[c.colorIndex];
      placeInCorner(c);
      c.score = 0;
    });
  };

  startRound();

  /* expose state for the UI */
  window.__haraGame = { cycles, corners: CORNERS };

  let last = performance.now();
  let aiAcc = 0, winAcc = 0;

  const frame = (now) => {
    const dt = Math.min(0.05, (now - last) / 1000);
    last = now;
    const nowS = now / 1000;
    update(dt, nowS);
    aiAcc += dt * 1000;
    while (aiAcc > AI_MS) {
      cycles.forEach((c) => c.alive && think(c, nowS));
      aiAcc -= AI_MS;
    }
    const alive = cycles.filter((c) => c.alive).length;
    if (alive <= 1) {
      winAcc += dt;
      if (winAcc > 1.5) {
        startRound();
        winAcc = 0;
      }
    } else {
      winAcc = 0;
    }
    draw(now);
    window.requestAnimationFrame(frame);
  };
  window.requestAnimationFrame(frame);
})();

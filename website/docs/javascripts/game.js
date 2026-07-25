/* Hara grid game — ambient light cycles riding the perspective floor of the
   fixed background (.hara-bg). Cycles move slowly, use real segment
   intersection collision, navigate toward waypoints, and react when rivals
   turn. Drifting in a straight line still builds a small speed boost; turning
   resets it. Positions are kept in a normalized arena space (k, t*T_SCALE) and
   projected with a fan geometry (vanishing point 720,461; lateral spacing 36 at
   the horizon growing to 260 at the bottom). */
(() => {
  const canvas = document.querySelector('[data-hara-component="game"]');
  if (!canvas) return;
  if (window.matchMedia('(prefers-reduced-motion: reduce)').matches) return;
  const ctx = canvas.getContext('2d');
  if (!ctx) return;

  /* cover-fit of the 1440x810 scene viewBox (preserveAspectRatio slice) */
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

  /* arena in normalized units: x = k (lateral), y = t * T_SCALE (depth). */
  const T_SCALE = 28;
  const X_MIN = -2.2, X_MAX = 2.2;
  const Y_MIN = 0.30 * T_SCALE, Y_MAX = 0.90 * T_SCALE;
  const SPEED_K = 0.55;         // k units / s  (lateral)
  const SPEED_T = 0.085;        // t units / s  (depth, multiplied by T_SCALE for y/s)
  const BOOST_MAX = 2.4;        // seconds of straight drift that build boost
  const BOOST_GAIN = 0.25;      // speed multiplier gained per boosted second
  const LOOK_T = 0.55;          // seconds of travel the AI looks ahead
  const AI_MS = 40;             // milliseconds between AI ticks
  const ROUND_MS = 60000;
  const COLORS = ['#41f5e4', '#ff2e88', '#9c7bff'];
  const DIRS = [[1, 0], [-1, 0], [0, 1], [0, -1]];

  const rand = (a, b) => a + Math.random() * (b - a);
  const clamp = (v, lo, hi) => Math.max(lo, Math.min(hi, v));
  const outOfBounds = (x, y) => x < X_MIN || x > X_MAX || y < Y_MIN || y > Y_MAX;

  /* ----------------------------------------------------------------------
     geometry: segment intersection for real collision detection
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

  /* ----------------------------------------------------------------------
     coarse occupancy grid for pathfinding (AI only)
     ---------------------------------------------------------------------- */
  const CELL = 0.15;
  const GX = Math.ceil((X_MAX - X_MIN) / CELL);
  const GY = Math.ceil((Y_MAX - Y_MIN) / CELL);
  const FLOOD_CAP = 600;
  const MIN_SPACE = 8;

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
        if (Math.abs(dx) + Math.abs(dy) > 1) continue; /* cross, not corners */
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

  const markWall = (g, w, hx, hy) => {
    for (let i = 0; i < w.length - 1; i++) {
      markSeg(g, w[i][0], w[i][1], w[i + 1][0], w[i + 1][1]);
    }
    if (hx !== null && w.length) {
      markSeg(g, w[w.length - 1][0], w[w.length - 1][1], hx, hy);
    }
  };

  const buildGrid = (self) => {
    const g = new Uint8Array(GX * GY);
    for (const c of cycles) {
      if (!c.alive && c !== self) continue;
      if (c === self) {
        /* ignore the tail we are currently extending */
        const w = c.wall;
        for (let i = 0; i < w.length - 2; i++) {
          markSeg(g, w[i][0], w[i][1], w[i + 1][0], w[i + 1][1]);
        }
      } else {
        markWall(g, c.wall, c.x, c.y);
      }
    }
    for (const d of debris) {
      markWall(g, d.wall, null, null);
    }
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
     cycles, debris, turn events
     ---------------------------------------------------------------------- */
  const debris = [];
  const DEBRIS_CAP = 2400;
  const turnEvents = []; /* { cycle, x, y, oldDir, newDir, time } */

  const findNearestOpponent = (c) => {
    let best = null, bestD = Infinity;
    for (const o of cycles) {
      if (o === c || !o.alive) continue;
      const d = Math.hypot(o.x - c.x, o.y - c.y);
      if (d < bestD) { bestD = d; best = o; }
    }
    return best;
  };

  const pickInitialDir = (x, y, g) => {
    let best = DIRS[0], bestSpace = -1;
    for (const d of DIRS) {
      const nx = x + d[0] * SPEED_K * 0.8;
      const ny = y + d[1] * SPEED_T * T_SCALE * 0.8;
      if (outOfBounds(nx, ny)) continue;
      const idx = cellOf(nx, ny);
      const space = idx < 0 ? 0 : flood(g, idx, 200);
      if (space > bestSpace) { bestSpace = space; best = d; }
    }
    return best.slice();
  };

  const spawn = (color) => ({
    color,
    x: rand(X_MIN * 0.6, X_MAX * 0.6),
    y: rand(Y_MIN + 2, Y_MAX - 2),
    dir: DIRS[Math.floor(Math.random() * 4)].slice(),
    wall: [],
    boost: 0,
    alive: true,
    respawnIn: 0,
    target: { x: 0, y: 0 },
    aggression: Math.random() < 0.5 ? 0.85 : 0.15,
  });

  const cycles = COLORS.map(spawn);

  const pickTarget = (c) => {
    const prey = findNearestOpponent(c);
    const candidates = [];
    for (let i = 0; i < 10; i++) {
      candidates.push({
        x: rand(X_MIN * 0.85, X_MAX * 0.85),
        y: rand(Y_MIN + 1.5, Y_MAX - 1.5)
      });
    }
    if (prey) {
      const dx = c.x - prey.x;
      const dy = c.y - prey.y;
      const d = Math.hypot(dx, dy) || 1;
      if (c.aggression > 0.5) {
        /* intercept where the prey is heading */
        candidates.push({
          x: clamp(prey.x + prey.dir[0] * SPEED_K * 2.5, X_MIN, X_MAX),
          y: clamp(prey.y + prey.dir[1] * SPEED_T * T_SCALE * 2.5, Y_MIN, Y_MAX)
        });
      } else {
        /* flee to the opposite side */
        candidates.push({
          x: clamp(c.x + dx / d * 4, X_MIN, X_MAX),
          y: clamp(c.y + dy / d * 4, Y_MIN, Y_MAX)
        });
      }
    }

    const g = buildGrid(c);
    let best = null, bestScore = -Infinity;
    for (const t of candidates) {
      const tx = clamp(t.x, X_MIN, X_MAX);
      const ty = clamp(t.y, Y_MIN, Y_MAX);
      const idx = cellOf(tx, ty);
      const space = idx < 0 ? 0 : flood(g, idx, FLOOD_CAP);
      let score = space * 2;
      score -= Math.hypot(tx - c.x, ty - c.y) * 0.6;
      if (c.aggression > 0.5 && prey) {
        score += Math.max(0, 10 - Math.hypot(tx - prey.x, ty - prey.y)) * 4;
      } else if (c.aggression < 0.5 && prey) {
        score += Math.hypot(tx - prey.x, ty - prey.y) * 0.4;
      }
      if (rayBlocked(g, c.x, c.y, tx, ty)) score -= 30;
      if (score > bestScore) { bestScore = score; best = { x: tx, y: ty }; }
    }
    c.target = best || { x: rand(X_MIN * 0.6, X_MAX * 0.6), y: rand(Y_MIN + 2, Y_MAX - 2) };
  };

  /* initialize positions and directions safely */
  cycles.forEach((c) => {
    c.wall.push([c.x, c.y]);
    const g = buildGrid(null);
    c.dir = pickInitialDir(c.x, c.y, g);
    pickTarget(c);
  });

  const crash = (c) => {
    c.alive = false;
    c.respawnIn = rand(1.2, 2.6);
    const lastP = c.wall[c.wall.length - 1];
    if (!lastP || Math.hypot(c.x - lastP[0], c.y - lastP[1]) > 0.01) {
      c.wall.push([c.x, c.y]);
    }
    if (c.wall.length > 1) debris.push({ color: c.color, wall: c.wall });
    let total = 0;
    for (const d of debris) total += d.wall.length;
    while (total > DEBRIS_CAP && debris.length) {
      total -= debris.shift().wall.length;
    }
  };

  const respawn = (c) => {
    let bestSpot = null, bestScore = -Infinity;
    for (let i = 0; i < 40; i++) {
      const x = rand(X_MIN * 0.7, X_MAX * 0.7);
      const y = rand(Y_MIN + 2, Y_MAX - 2);
      const idx = cellOf(x, y);
      if (idx < 0) continue;
      const g = buildGrid(null);
      if (g[idx]) continue;
      const space = flood(g, idx, FLOOD_CAP);
      /* check that at least one direction is immediately clear */
      let bestDirSpace = 0;
      for (const d of DIRS) {
        const nx = x + d[0] * SPEED_K * 0.8;
        const ny = y + d[1] * SPEED_T * T_SCALE * 0.8;
        if (outOfBounds(nx, ny)) continue;
        const nidx = cellOf(nx, ny);
        const nspace = nidx < 0 ? 0 : flood(g, nidx, 200);
        if (nspace > bestDirSpace) bestDirSpace = nspace;
      }
      let score = space * 0.6 + bestDirSpace;
      score += Math.min(x - X_MIN, X_MAX - x);
      score += Math.min(y - Y_MIN, Y_MAX - y);
      if (score > bestScore) { bestScore = score; bestSpot = { x, y }; }
      if (space >= FLOOD_CAP && bestDirSpace >= 100) break;
    }
    if (!bestSpot || bestScore < 50) {
      c.respawnIn = 0.5;
      return;
    }
    const fresh = spawn(c.color);
    fresh.x = bestSpot.x;
    fresh.y = bestSpot.y;
    fresh.wall = [[bestSpot.x, bestSpot.y]];
    const g = buildGrid(null);
    fresh.dir = pickInitialDir(bestSpot.x, bestSpot.y, g);
    Object.assign(c, fresh);
    pickTarget(c);
  };

  /* ----------------------------------------------------------------------
     continuous collision detection
     ---------------------------------------------------------------------- */
  const checkCollision = (c, px, py) => {
    const segA = [px, py];
    const segB = [c.x, c.y];
    for (const other of cycles) {
      const w = other.wall;
      const isSelf = other === c;
      const segCount = w.length - 1;
      const limit = isSelf ? Math.max(0, segCount - 1) : segCount;
      for (let i = 0; i < limit; i++) {
        if (segIntersect(segA, segB, w[i], w[i + 1])) {
          return lineIntersect(segA, segB, w[i], w[i + 1]) ||
            [(segA[0] + segB[0]) / 2, (segA[1] + segB[1]) / 2];
        }
      }
      if (!isSelf && other.alive && w.length) {
        if (segIntersect(segA, segB, w[w.length - 1], [other.x, other.y])) {
          return lineIntersect(segA, segB, w[w.length - 1], [other.x, other.y]) ||
            [(segA[0] + segB[0]) / 2, (segA[1] + segB[1]) / 2];
        }
      }
    }
    for (const d of debris) {
      const w = d.wall;
      for (let i = 0; i < w.length - 1; i++) {
        if (segIntersect(segA, segB, w[i], w[i + 1])) {
          return lineIntersect(segA, segB, w[i], w[i + 1]) ||
            [(segA[0] + segB[0]) / 2, (segA[1] + segB[1]) / 2];
        }
      }
    }
    return null;
  };

  /* ----------------------------------------------------------------------
     AI: waypoint routing with turn-event reaction and aggression
     ---------------------------------------------------------------------- */
  const pathCrossesTurn = (c, d, e) => {
    const mult = 1 + c.boost * BOOST_GAIN;
    const nx = c.x + d[0] * SPEED_K * mult * LOOK_T;
    const ny = c.y + d[1] * SPEED_T * T_SCALE * mult * LOOK_T;
    const tx = e.x + e.newDir[0] * SPEED_K * mult * LOOK_T;
    const ty = e.y + e.newDir[1] * SPEED_T * T_SCALE * mult * LOOK_T;
    return segIntersect([c.x, c.y], [nx, ny], [e.x, e.y], [tx, ty]);
  };

  const think = (c, nowS) => {
    if (!c.alive) return;
    const g = buildGrid(c);
    const [dx, dy] = c.dir;
    const cands = [[dx, dy], [dy, -dx], [-dy, dx]]; /* straight, left, right */
    const prey = findNearestOpponent(c);
    const recentTurns = turnEvents.filter((e) => e.time > nowS - 1.2 && e.cycle !== c);

    let best = null, bestScore = -Infinity;
    for (const d of cands) {
      const mult = 1 + c.boost * BOOST_GAIN;
      const nx = c.x + d[0] * SPEED_K * mult * LOOK_T;
      const ny = c.y + d[1] * SPEED_T * T_SCALE * mult * LOOK_T;
      if (outOfBounds(nx, ny) || rayBlocked(g, c.x, c.y, nx, ny)) continue;
      const idx = cellOf(nx, ny);
      const space = idx < 0 ? 0 : flood(g, idx, FLOOD_CAP);
      if (space < MIN_SPACE) continue;

      const before = Math.hypot(c.target.x - c.x, c.target.y - c.y);
      const after = Math.hypot(c.target.x - nx, c.target.y - ny);
      let score = (before - after) * 12;
      score += space * 0.4;
      if (d[0] === dx && d[1] === dy) score += 10;     /* keep boost */
      if (space < 20) score -= 25;                     /* avoid tight pockets */

      /* react to rivals that just turned across our path */
      for (const e of recentTurns) {
        if (pathCrossesTurn(c, d, e)) score -= 35;
      }

      /* aggression tweaks */
      if (c.aggression > 0.5 && prey) {
        score += Math.max(0, 8 - Math.hypot(nx - prey.x, ny - prey.y)) * 3;
      } else if (c.aggression < 0.5 && prey) {
        score += Math.hypot(nx - prey.x, ny - prey.y) * 0.3;
      }

      score += Math.random() * 2;
      if (score > bestScore) { bestScore = score; best = d; }
    }

    if (!best) {
      /* emergency fallback: any direction that is not immediately blocked */
      for (const d of cands) {
        const nx = c.x + d[0] * SPEED_K * LOOK_T;
        const ny = c.y + d[1] * SPEED_T * T_SCALE * LOOK_T;
        if (!outOfBounds(nx, ny) && !rayBlocked(g, c.x, c.y, nx, ny)) {
          best = d; break;
        }
      }
    }

    if (!best) {
      crash(c);
      return;
    }

    if (best[0] !== dx || best[1] !== dy) {
      c.wall.push([c.x, c.y]);
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

    if (Math.hypot(c.target.x - c.x, c.target.y - c.y) < 1.0) {
      pickTarget(c);
    }
  };

  const update = (dt, nowS) => {
    /* prune old turn events */
    while (turnEvents.length && turnEvents[0].time < nowS - 2.0) turnEvents.shift();

    for (const c of cycles) {
      if (!c.alive) {
        c.respawnIn -= dt;
        if (c.respawnIn <= 0) respawn(c);
        continue;
      }
      c.boost = Math.min(BOOST_MAX, c.boost + dt);
      const mult = 1 + c.boost * BOOST_GAIN;
      const px = c.x, py = c.y;
      c.x += c.dir[0] * SPEED_K * mult * dt;
      c.y += c.dir[1] * SPEED_T * T_SCALE * mult * dt;

      if (outOfBounds(c.x, c.y)) {
        c.x = clamp(c.x, X_MIN, X_MAX);
        c.y = clamp(c.y, Y_MIN, Y_MAX);
        c.wall.push([c.x, c.y]);
        crash(c);
        continue;
      }

      const hit = checkCollision(c, px, py);
      if (hit) {
        c.x = hit[0];
        c.y = hit[1];
        c.wall.push([c.x, c.y]);
        crash(c);
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

  const draw = (now) => {
    ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
    ctx.clearRect(0, 0, vw, vh);
    ctx.lineJoin = 'miter';

    const segs = [];
    for (const d of debris) {
      const w = d.wall;
      for (let i = 0; i < w.length - 1; i++) {
        segs.push({ a: w[i], b: w[i + 1], color: d.color, alpha: 0.72 });
      }
    }
    for (const c of cycles) {
      const w = c.wall;
      for (let i = 0; i < w.length - 1; i++) {
        segs.push({ a: w[i], b: w[i + 1], color: c.color, alpha: 1 });
      }
      if (c.alive && w.length) {
        segs.push({ a: w[w.length - 1], b: [c.x, c.y], color: c.color, alpha: 1 });
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

    /* target markers */
    cycles.forEach((c) => {
      if (!c.alive) return;
      const [tx, ty] = project(c.target.x, c.target.y / T_SCALE);
      ctx.strokeStyle = c.color;
      ctx.globalAlpha = 0.22;
      ctx.lineWidth = 1;
      ctx.beginPath();
      ctx.arc(tx, ty, 5, 0, Math.PI * 2);
      ctx.stroke();
    });

    /* heads */
    cycles.forEach((c, ci) => {
      if (!c.alive) return;
      const [hx, hy] = project(c.x, c.y / T_SCALE);
      const top = hy - heightAt(c.y);
      const phase = ((now / 1000) * 0.9 + ci * 0.33) % 1;
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

  let last = performance.now();
  let aiAcc = 0, roundAcc = 0, winAcc = 0;

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
    roundAcc += dt * 1000;
    if (roundAcc > ROUND_MS) {
      cycles.forEach((c) => c.alive && crash(c));
      debris.length = 0;
      roundAcc = 0;
    }
    const alive = cycles.filter((c) => c.alive).length;
    if (alive === 0) {
      debris.length = 0;
      cycles.forEach(respawn);
      roundAcc = 0;
      winAcc = 0;
    } else if (alive === 1) {
      winAcc += dt;
      if (winAcc > 2.5) {
        debris.length = 0;
        cycles.forEach(respawn);
        roundAcc = 0;
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

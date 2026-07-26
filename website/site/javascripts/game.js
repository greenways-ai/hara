/* Hara grid game — zone-control Tron on a square isometric arena.
   Four cycles start in corner bases, paint 1×1 cells as they move, claim
   objective zones, and eliminate each other until one remains. */
(() => {
  const canvas = document.querySelector('[data-hara-component="game"]');
  if (!canvas) return;
  if (window.matchMedia('(prefers-reduced-motion: reduce)').matches) return;
  const ctx = canvas.getContext('2d');
  if (!ctx) return;

  let vw = 0, vh = 0, dpr = 1, isoScale = 1, cx = 0, cy = 0;

  const resize = () => {
    vw = window.innerWidth;
    vh = window.innerHeight;
    dpr = Math.min(2, window.devicePixelRatio || 1);
    canvas.width = Math.round(vw * dpr);
    canvas.height = Math.round(vh * dpr);
    /* isometric: 30x30 arena, zoomed in ~2x so the diamond fills the upper
       half of the hero while keeping all four corner bases on screen. */
    isoScale = Math.min(vw / 75, vh / 47) * dpr;
    cx = (vw * dpr) * 0.45;
    cy = (vh * dpr) * 0.30;
  };
  window.addEventListener('resize', resize);
  resize();

  /* ----------------------------------------------------------------------
     arena and movement
     ---------------------------------------------------------------------- */
  const X_MIN = -15, X_MAX = 15;
  const Y_MIN = -15, Y_MAX = 15;
  const SPEED = 2.0;
  const BOOST_MAX = 1.5;
  const BOOST_GAIN = 0.25;
  const LOOK_T = 0.6;
  const AI_MS = 40;
  const WALL_H = 1.0;
  const TAIL_CAP = 20;
  const MAX_TAIL_LENGTH = 60;
  const TAIL_LIFE = 8;
  const CELL = 1.0;
  const GRID_W = Math.ceil((X_MAX - X_MIN) / CELL);
  const GRID_H = Math.ceil((Y_MAX - Y_MIN) / CELL);

  const COLORS = ['#41f5e4', '#ff2e88', '#9c7bff', '#f5d742'];
  const DIRS = [[1, 0], [-1, 0], [0, 1], [0, -1]];

  const BASE_SIZE = 6;
  const ZONE_SIZE = 8;
  const BASES = [
    { xMin: X_MIN, xMax: X_MIN + BASE_SIZE, yMin: Y_MIN, yMax: Y_MIN + BASE_SIZE },
    { xMin: X_MAX - BASE_SIZE, xMax: X_MAX, yMin: Y_MIN, yMax: Y_MIN + BASE_SIZE },
    { xMin: X_MIN, xMax: X_MIN + BASE_SIZE, yMax: Y_MAX, yMin: Y_MAX - BASE_SIZE },
    { xMin: X_MAX - BASE_SIZE, xMax: X_MAX, yMax: Y_MAX, yMin: Y_MAX - BASE_SIZE }
  ];

  const ZONES = [
    /* home zones — index matches base/corner; cannot be claimed by home player */
    { name: 'TL home', xMin: -12, xMax: -8, yMin: -12, yMax: -8, home: 0 },
    { name: 'TR home', xMin: 8, xMax: 12, yMin: -12, yMax: -8, home: 1 },
    { name: 'BL home', xMin: -12, xMax: -8, yMin: 8, yMax: 12, home: 2 },
    { name: 'BR home', xMin: 8, xMax: 12, yMin: 8, yMax: 12, home: 3 },
    /* edge zones */
    { name: 'top edge', xMin: -3, xMax: 3, yMin: -14, yMax: -11 },
    { name: 'bottom edge', xMin: -3, xMax: 3, yMin: 11, yMax: 14 },
    { name: 'left edge', xMin: -14, xMax: -11, yMin: -3, yMax: 3 },
    { name: 'right edge', xMin: 11, xMax: 14, yMin: -3, yMax: 3 },
    /* center zone */
    { name: 'center', xMin: -3, xMax: 3, yMin: -3, yMax: 3 }
  ];

  const SCORE_CLAIM = 50;
  const SCORE_STEAL = 100;
  const SCORE_HOLD_PER_S = 3;
  const SCORE_HOME_LOSS_PER_S = -5;
  const SCORE_NEW_CELL = 1;
  const SCORE_STEAL_CELL = 2;

  const rand = (a, b) => a + Math.random() * (b - a);
  const clamp = (v, lo, hi) => Math.max(lo, Math.min(hi, v));
  const inZone = (x, y, z) => x >= z.xMin && x <= z.xMax && y >= z.yMin && y <= z.yMax;

  /* ----------------------------------------------------------------------
     isometric projection
     ---------------------------------------------------------------------- */
  const project = (x, y, z = 0) => {
    return [
      cx + (x - y) * isoScale,
      cy + (x + y) * 0.5 * isoScale - z * isoScale
    ];
  };

  /* ----------------------------------------------------------------------
     geometry
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
     territory grid and zones
     ---------------------------------------------------------------------- */
  const territory = new Int8Array(GRID_W * GRID_H).fill(-1);
  const cellIndex = (x, y) => {
    const ix = Math.floor((x - X_MIN) / CELL);
    const iy = Math.floor((y - Y_MIN) / CELL);
    if (ix < 0 || ix >= GRID_W || iy < 0 || iy >= GRID_H) return -1;
    return iy * GRID_W + ix;
  };

  const claimCell = (c, x, y, nowS) => {
    const idx = cellIndex(x, y);
    if (idx < 0) return;
    const owner = territory[idx];
    if (owner === c.colorIndex) return;
    if (owner < 0) {
      territory[idx] = c.colorIndex;
      c.score += SCORE_NEW_CELL;
    } else {
      territory[idx] = c.colorIndex;
      c.score += SCORE_STEAL_CELL;
    }
    /* update zone counts */
    for (const z of ZONES) {
      if (x >= z.xMin && x <= z.xMax && y >= z.yMin && y <= z.yMax) {
        if (!z.counts) z.counts = new Int32Array(COLORS.length);
        if (owner >= 0) z.counts[owner]--;
        z.counts[c.colorIndex]++;
      }
    }
  };

  const markSeg = (c, ax, ay, bx, by, nowS) => {
    const len = Math.hypot(bx - ax, by - ay);
    const steps = Math.max(1, Math.ceil(len / (CELL * 0.4)));
    for (let i = 0; i <= steps; i++) {
      claimCell(c, ax + ((bx - ax) * i) / steps, ay + ((by - ay) * i) / steps, nowS);
    }
  };

  const updateZones = (nowS) => {
    for (const z of ZONES) {
      if (!z.counts) continue;
      let best = -1, bestCount = -1;
      for (let i = 0; i < COLORS.length; i++) {
        if (z.counts[i] > bestCount) { bestCount = z.counts[i]; best = i; }
      }
      if (bestCount < 3) { /* need at least a few cells to claim */
        best = -1;
      }
      /* home zone cannot be claimed by its owner */
      if (z.home !== undefined && best === z.home) {
        best = -1;
      }
      if (best !== z.owner) {
        if (best >= 0 && z.owner >= 0) {
          cycles[best].score += SCORE_STEAL;
        } else if (best >= 0 && z.owner < 0) {
          cycles[best].score += SCORE_CLAIM;
        }
        z.owner = best;
        z.ownerTime = nowS;
      }
    }
  };

  /* ----------------------------------------------------------------------
     cycles
     ---------------------------------------------------------------------- */
  const cycles = COLORS.map((_, i) => ({
    color: COLORS[i],
    colorIndex: i,
    corner: i,
    x: 0, y: 0,
    lastTurn: [0, 0],
    dir: [1, 0],
    segments: [],
    boost: 0,
    alive: true,
    score: 0
  }));

  const shuffleCorners = () => {
    const order = [0, 1, 2, 3].sort(() => Math.random() - 0.5);
    cycles.forEach((c, i) => { c.corner = order[i]; });
  };

  const placeInBase = (c) => {
    const b = BASES[c.corner];
    c.x = rand(b.xMin + 0.5, b.xMax - 0.5);
    c.y = rand(b.yMin + 0.5, b.yMax - 0.5);
    c.lastTurn = [c.x, c.y];
    c.segments = [];
    c.boost = 0;
    c.alive = true;
    /* pick direction away from corner center */
    const cx = (b.xMin + b.xMax) / 2;
    const cy = (b.yMin + b.yMax) / 2;
    const dx = c.x - cx, dy = c.y - cy;
    if (Math.abs(dx) >= Math.abs(dy)) {
      c.dir = [dx >= 0 ? 1 : -1, 0];
    } else {
      c.dir = [0, dy >= 0 ? 1 : -1];
    }
  };

  const tailLength = (c) => {
    let len = 0;
    for (const s of c.segments) len += segLength(s);
    return len;
  };

  const trimTail = (c, nowS) => {
    while (c.segments.length > 1 && tailLength(c) > MAX_TAIL_LENGTH) c.segments.shift();
    while (c.segments.length > TAIL_CAP) c.segments.shift();
    while (c.segments.length && c.segments[0].born < nowS - TAIL_LIFE) c.segments.shift();
  };

  const commitSegment = (c, nowS) => {
    const seg = { a: [c.lastTurn[0], c.lastTurn[1]], b: [c.x, c.y], born: nowS };
    c.segments.push(seg);
    trimTail(c, nowS);
    c.lastTurn = [c.x, c.y];
  };

  const crash = (c, nowS) => {
    c.alive = false;
    c.segments = [];
  };

  /* ----------------------------------------------------------------------
     collision
     ---------------------------------------------------------------------- */
  const checkCollision = (c, px, py) => {
    const segA = [px, py];
    const segB = [c.x, c.y];
    /* arena bounds */
    if (c.x < X_MIN || c.x > X_MAX || c.y < Y_MIN || c.y > Y_MAX) {
      return [clamp(c.x, X_MIN, X_MAX), clamp(c.y, Y_MIN, Y_MAX)];
    }
    /* trails */
    for (const other of cycles) {
      const segs = other.segments;
      const isSelf = other === c;
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
    return null;
  };

  /* ----------------------------------------------------------------------
     AI
     ---------------------------------------------------------------------- */
  const rayBlocked = (c, d, dist) => {
    const steps = Math.max(1, Math.ceil(dist / 0.5));
    for (let i = 1; i <= steps; i++) {
      const x = c.x + d[0] * dist * i / steps;
      const y = c.y + d[1] * dist * i / steps;
      if (x < X_MIN + 0.2 || x > X_MAX - 0.2 || y < Y_MIN + 0.2 || y > Y_MAX - 0.2) return true;
      /* check against trails */
      for (const other of cycles) {
        if (other === c && i === steps) continue; /* allow reaching head */
        const segs = other.segments;
        const limit = other === c ? Math.max(0, segs.length - 1) : segs.length;
        for (let j = 0; j < limit; j++) {
          if (segIntersect([c.x, c.y], [x, y], segs[j].a, segs[j].b)) return true;
        }
        if (other !== c && other.alive) {
          if (segIntersect([c.x, c.y], [x, y], other.lastTurn, [other.x, other.y])) return true;
        }
      }
    }
    return false;
  };

  const floodSpace = (c, d, dist) => {
    /* rough open-space estimate: count free cells around the lookahead point */
    const nx = c.x + d[0] * dist;
    const ny = c.y + d[1] * dist;
    const idx = cellIndex(nx, ny);
    if (idx < 0) return 0;
    let count = 0;
    const r = 2;
    for (let dy = -r; dy <= r; dy++) {
      for (let dx = -r; dx <= r; dx++) {
        const ix = idx % GRID_W + dx;
        const iy = Math.floor(idx / GRID_W) + dy;
        if (ix < 0 || ix >= GRID_W || iy < 0 || iy >= GRID_H) continue;
        if (territory[iy * GRID_W + ix] === c.colorIndex) count += 0.5;
        else count += 1;
      }
    }
    return count;
  };

  const cellsInZone = (c, d, dist, z) => {
    let count = 0, rival = 0, empty = 0;
    const steps = Math.max(1, Math.ceil(dist / 0.5));
    for (let i = 0; i <= steps; i++) {
      const x = c.x + d[0] * dist * i / steps;
      const y = c.y + d[1] * dist * i / steps;
      if (x < z.xMin || x > z.xMax || y < z.yMin || y > z.yMax) continue;
      const idx = cellIndex(x, y);
      if (idx < 0) continue;
      count++;
      const owner = territory[idx];
      if (owner < 0) empty++;
      else if (owner !== c.colorIndex) rival++;
    }
    return { count, rival, empty };
  };

  const think = (c, nowS) => {
    if (!c.alive) return;
    const [dx, dy] = c.dir;
    const cands = [[dx, dy], [dy, -dx], [-dy, dx]];
    const mult = 1 + c.boost * BOOST_GAIN;
    const dist = SPEED * mult * LOOK_T;

    let best = null, bestScore = -Infinity;
    for (const d of cands) {
      if (rayBlocked(c, d, dist)) continue;
      const space = floodSpace(c, d, dist);
      if (space < 8) continue;

      let score = space * 0.3;
      if (d[0] === dx && d[1] === dy) score += 5;

      /* zone scoring */
      for (const z of ZONES) {
        /* skip home zone of this cycle */
        if (z.home === c.colorIndex) continue;
        const cells = cellsInZone(c, d, dist, z);
        if (cells.count === 0) continue;
        const value = (z.home !== undefined ? 2 : 1) + (z.name === 'center' ? 1.5 : 0);
        score += cells.empty * value;
        score += cells.rival * value * 3;
        /* already-owned zone: small bonus to stay and hold */
        if (z.owner === c.colorIndex) score += 8 * value;
      }

      score += Math.random() * 2;
      if (score > bestScore) { bestScore = score; best = d; }
    }

    if (!best) {
      /* emergency: any unblocked direction */
      for (const d of cands) {
        if (!rayBlocked(c, d, dist * 0.5)) { best = d; break; }
      }
    }
    if (!best) return;

    if (best[0] !== dx || best[1] !== dy) {
      commitSegment(c, nowS);
      c.dir = best.slice();
      c.boost = 0;
    }
  };

  /* ----------------------------------------------------------------------
     update loop
     ---------------------------------------------------------------------- */
  const update = (dt, nowS) => {
    for (const c of cycles) {
      if (!c.alive) continue;
      c.boost = Math.min(BOOST_MAX, c.boost + dt);
      const mult = 1 + c.boost * BOOST_GAIN;
      const px = c.x, py = c.y;
      c.x += c.dir[0] * SPEED * mult * dt;
      c.y += c.dir[1] * SPEED * mult * dt;

      /* paint trail cells */
      markSeg(c, px, py, c.x, c.y, nowS);
      trimTail(c, nowS);

      const hit = checkCollision(c, px, py);
      if (hit) {
        c.x = hit[0]; c.y = hit[1];
        crash(c, nowS);
      }
    }
    updateZones(nowS);

    /* hold / invasion scoring */
    for (const c of cycles) {
      if (!c.alive) continue;
      for (const z of ZONES) {
        if (z.owner === c.colorIndex) c.score += SCORE_HOLD_PER_S * dt;
        if (z.home === c.colorIndex && z.owner >= 0 && z.owner !== c.colorIndex) {
          c.score += SCORE_HOME_LOSS_PER_S * dt;
        }
      }
    }
  };

  /* ----------------------------------------------------------------------
     rendering
     ---------------------------------------------------------------------- */
  const isoPoly = (pts) => {
    ctx.beginPath();
    const [x0, y0] = project(pts[0][0], pts[0][1], pts[0][2] || 0);
    ctx.moveTo(x0, y0);
    for (let i = 1; i < pts.length; i++) {
      const [xi, yi] = project(pts[i][0], pts[i][1], pts[i][2] || 0);
      ctx.lineTo(xi, yi);
    }
    ctx.closePath();
  };

  const drawZone = (z, color, alpha, now) => {
    const pts = [
      [z.xMin, z.yMin, 0],
      [z.xMax, z.yMin, 0],
      [z.xMax, z.yMax, 0],
      [z.xMin, z.yMax, 0]
    ];
    isoPoly(pts);
    ctx.fillStyle = color;
    ctx.globalAlpha = alpha;
    ctx.fill();
    ctx.strokeStyle = color;
    ctx.globalAlpha = Math.min(1, alpha + 0.25);
    ctx.lineWidth = 2;
    ctx.stroke();
    ctx.globalAlpha = 1;
  };

  const draw = (now) => {
    ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
    ctx.clearRect(0, 0, vw * dpr, vh * dpr);

    const nowS = now / 1000;
    const prims = [];

    /* arena floor — dark, low-contrast so the hero text stays legible */
    const floor = [
      [X_MIN, Y_MIN, 0],
      [X_MAX, Y_MIN, 0],
      [X_MAX, Y_MAX, 0],
      [X_MIN, Y_MAX, 0]
    ];
    isoPoly(floor);
    ctx.fillStyle = 'rgba(16, 38, 56, .72)';
    ctx.fill();
    ctx.strokeStyle = 'rgba(65, 245, 228, .45)';
    ctx.lineWidth = 2;
    ctx.stroke();

    /* subtle isometric grid on the floor */
    ctx.save();
    ctx.strokeStyle = 'rgba(65, 245, 228, .22)';
    ctx.lineWidth = 1;
    ctx.globalAlpha = 0.9;
    for (let x = X_MIN; x <= X_MAX; x += 5) {
      ctx.beginPath();
      const [x0, y0] = project(x, Y_MIN, 0);
      const [x1, y1] = project(x, Y_MAX, 0);
      ctx.moveTo(x0, y0); ctx.lineTo(x1, y1);
      ctx.stroke();
    }
    for (let y = Y_MIN; y <= Y_MAX; y += 5) {
      ctx.beginPath();
      const [x0, y0] = project(X_MIN, y, 0);
      const [x1, y1] = project(X_MAX, y, 0);
      ctx.moveTo(x0, y0); ctx.lineTo(x1, y1);
      ctx.stroke();
    }
    ctx.restore();

    /* zones — dark, muted fills; owned zones glow softly with the cycle color */
    for (const z of ZONES) {
      const color = z.owner >= 0 ? COLORS[z.owner] : (z.home !== undefined ? 'rgba(30, 55, 72, .25)' : 'rgba(22, 45, 60, .18)');
      const alpha = z.owner >= 0 ? 0.12 : 0.08;
      drawZone(z, color, alpha, now);
    }

    /* corner bases — faint outline so all four starting areas are readable */
    for (const b of BASES) {
      drawZone(b, 'rgba(65, 245, 228, .12)', 0.10, now);
    }

    /* collect wall primitives */
    for (const c of cycles) {
      for (const s of c.segments) {
        const fade = Math.max(0, 1 - (nowS - s.born) / TAIL_LIFE);
        prims.push({
          type: 'wall',
          a: s.a, b: s.b,
          color: c.color,
          alpha: fade,
          depth: (s.a[0] + s.a[1] + s.b[0] + s.b[1]) / 4
        });
      }
      if (c.alive) {
        prims.push({
          type: 'wall',
          a: c.lastTurn, b: [c.x, c.y],
          color: c.color,
          alpha: 1,
          depth: (c.lastTurn[0] + c.lastTurn[1] + c.x + c.y) / 4
        });
        prims.push({
          type: 'head',
          x: c.x, y: c.y,
          color: c.color,
          depth: c.x + c.y
        });
      }
    }

    /* painter's sort: larger depth = closer to viewer = draw later */
    prims.sort((p1, p2) => p1.depth - p2.depth);

    for (const p of prims) {
      if (p.type === 'wall') {
        const [ax, ay] = p.a;
        const [bx, by] = p.b;
        /* front face */
        isoPoly([[ax, ay, 0], [bx, by, 0], [bx, by, WALL_H], [ax, ay, WALL_H]]);
        ctx.fillStyle = p.color;
        ctx.globalAlpha = 0.45 * p.alpha;
        ctx.fill();
        /* top edge */
        ctx.beginPath();
        const [t1x, t1y] = project(ax, ay, WALL_H);
        const [t2x, t2y] = project(bx, by, WALL_H);
        ctx.moveTo(t1x, t1y);
        ctx.lineTo(t2x, t2y);
        ctx.strokeStyle = p.color;
        ctx.globalAlpha = p.alpha;
        ctx.lineWidth = 6;
        ctx.stroke();
        ctx.shadowColor = p.color;
        ctx.shadowBlur = 8;
        ctx.stroke();
        ctx.shadowBlur = 0;
        /* inner bright core */
        ctx.globalAlpha = 0.65 * p.alpha;
        ctx.lineWidth = 2;
        ctx.stroke();
      } else if (p.type === 'head') {
        const [hx, hy] = project(p.x, p.y, WALL_H);
        ctx.shadowColor = p.color;
        ctx.shadowBlur = 14;
        ctx.fillStyle = '#eafcff';
        ctx.beginPath();
        ctx.arc(hx, hy, 3.5, 0, Math.PI * 2);
        ctx.fill();
        ctx.shadowBlur = 0;
      }
    }
    ctx.globalAlpha = 1;
  };

  /* ----------------------------------------------------------------------
     round lifecycle
     ---------------------------------------------------------------------- */
  const startRound = () => {
    shuffleCorners();
    territory.fill(-1);
    ZONES.forEach((z) => { z.owner = -1; z.counts = new Int32Array(COLORS.length); z.ownerTime = 0; });
    cycles.forEach((c, i) => {
      /* colors stay fixed to each cycle across rounds */
      c.colorIndex = i;
      c.color = COLORS[i];
      placeInBase(c);
      c.score = 0;
    });
  };

  startRound();
  window.__haraGame = { cycles, zones: ZONES };

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
      if (winAcc > 3.0) {
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

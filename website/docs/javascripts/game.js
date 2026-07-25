/* Hara grid game — ambient light cycles riding the perspective floor of the
   fixed background (.hara-bg). Cycles move in the floor plane, turn at right
   angles, leave persistent walls, and steer by flood-fill pathfinding: they
   turn when a wall or trail blocks them and hunt each other, cutting across
   rival paths to force kills. Drifting in a straight line builds speed;
   turning resets it. Positions are kept in a normalized arena space
   (k, t*T_SCALE) and projected with a fan geometry (vanishing point 720,461;
   lateral spacing 36 at the horizon growing to 260 at the bottom). */
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

  /* contained arena in normalized units: x = k (lateral), y = t * T_SCALE
     (depth). Sized to read as a square on screen; the boundary is an
     invisible wall the cycles crash into */
  const T_SCALE = 28;
  const X_MIN = -2.2, X_MAX = 2.2, Y_MIN = 0.30 * T_SCALE, Y_MAX = 0.90 * T_SCALE;
  /* per-axis speeds tuned so screen speed is equal in both directions
     (lateral ~165 viewBox px/s at mid depth, depth the same) */
  const SPEED_K = 0.75;         // k units / s
  const SPEED_T = 0.17;         // t units / s
  const BOOST_MAX = 2.4;        // seconds of straight drift that build boost
  const BOOST_GAIN = 0.55;      // speed multiplier gained per boosted second
  const LOOK_T = 0.55;          // seconds of travel the AI looks ahead
  const DOOM_SPACE = 50;        // a pocket this small is not worth circling
  const ROUND_MS = 40000;
  const COLORS = ['#41f5e4', '#ff2e88', '#9c7bff'];
  const DIRS = [[1, 0], [-1, 0], [0, 1], [0, -1]];

  const rand = (a, b) => a + Math.random() * (b - a);

  const spawn = (color) => ({
    color,
    x: rand(X_MIN * 0.6, X_MAX * 0.6),
    y: rand(Y_MIN + 2, Y_MAX - 2),
    dir: DIRS[Math.floor(Math.random() * 4)].slice(),
    wall: [],
    boost: 0,
    alive: true,
    respawnIn: 0,
  });

  const cycles = COLORS.map(spawn);
  cycles.forEach((c) => c.wall.push([c.x, c.y]));

  /* walls of dead cycles: they stay on the field (drawn dimmer, still
     lethal) until the round reset, so the arena fills up as kills land */
  const debris = [];
  const DEBRIS_CAP = 2400; /* max debris points kept */

  const outOfBounds = (x, y) => x < X_MIN || x > X_MAX || y < Y_MIN || y > Y_MAX;

  /* ---- pathfinding: coarse occupancy grid + flood fill ----------------
     The arena is rasterized into cells (wall points plus a one-cell margin
     for collision distance). A cycle evaluates straight/left/right by flood
     filling the free space reachable from a step in that direction, and
     steers toward the roomiest option — turning only when a wall or trail
     genuinely blocks it. */

  const CELL = 0.15;
  const GX = Math.ceil((X_MAX - X_MIN) / CELL);
  const GY = Math.ceil((Y_MAX - Y_MIN) / CELL);
  const FLOOD_CAP = 400;
  const MIN_SPACE = 25; /* candidates with less room than this are suicide */

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

  /* walls are polylines with points only at turns, so mark along the
     segments, not just the points */
  const markSeg = (g, ax, ay, bx, by) => {
    const len = Math.hypot(bx - ax, by - ay);
    const steps = Math.max(1, Math.ceil(len / (CELL * 0.5)));
    for (let i = 0; i <= steps; i++) {
      mark(g, ax + ((bx - ax) * i) / steps, ay + ((by - ay) * i) / steps);
    }
  };

  /* mark a whole wall polyline */
  const markWall = (g, w, hx, hy) => {
    for (let i = 0; i < w.length - 1; i++) {
      markSeg(g, w[i][0], w[i][1], w[i + 1][0], w[i + 1][1]);
    }
    /* live cycles: the current run from the last turn to the head */
    if (hx !== null && w.length) {
      markSeg(g, w[w.length - 1][0], w[w.length - 1][1], hx, hy);
    }
  };

  const buildGrid = (self) => {
    const g = new Uint8Array(GX * GY);
    for (const c of cycles) {
      if (!c.alive) continue;
      /* ignore the fresh end of our own wall (last turn + current run) */
      if (c === self) {
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

  /* free cells reachable from start, capped */
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

  /* walk the grid from (x1,y1) to (x2,y2); true if the path hits a marked
     cell or leaves the arena */
  const rayBlocked = (g, x1, y1, x2, y2) => {
    const len = Math.hypot(x2 - x1, y2 - y1);
    const steps = Math.max(1, Math.ceil(len / (CELL * 0.5)));
    for (let i = 1; i <= steps; i++) {
      const idx = cellOf(x1 + ((x2 - x1) * i) / steps, y1 + ((y2 - y1) * i) / steps);
      if (idx < 0 || g[idx]) return true;
    }
    return false;
  };

  const think = (c) => {
    const g = buildGrid(c);
    const [dx, dy] = c.dir;
    const cands = [[dx, dy], [dy, -dx], [-dy, dx]]; /* straight, left, right */

    /* hunt: aim to cut across the nearest opponent's projected path */
    let prey = null, preyD = Infinity;
    for (const o of cycles) {
      if (o === c || !o.alive) continue;
      const d = Math.hypot(o.x - c.x, o.y - c.y);
      if (d < preyD) { preyD = d; prey = o; }
    }
    const hunting = prey && preyD < 9;
    const ix = hunting ? prey.x + prey.dir[0] * SPEED_K * 3 : 0;
    const iy = hunting ? prey.y + prey.dir[1] * SPEED_T * T_SCALE * 3 : 0;

    /* lookahead scales with actual speed on each axis, so reaction time is
       constant no matter how fast the cycle is drifting */
    const mult = 1 + c.boost * BOOST_GAIN;
    let best = null, bestScore = -1, bestRoom = 0;
    for (const d of cands) {
      const nx = c.x + d[0] * SPEED_K * mult * LOOK_T;
      const ny = c.y + d[1] * SPEED_T * T_SCALE * mult * LOOK_T;
      if (outOfBounds(nx, ny) || rayBlocked(g, c.x, c.y, nx, ny)) continue;
      const space = flood(g, cellOf(nx, ny), FLOOD_CAP);
      if (space < MIN_SPACE) continue;
      let score = space;
      /* keep the line: straight drift keeps the speed boost */
      if (d[0] === dx && d[1] === dy) score += 25;
      if (hunting) score += Math.max(0, 70 - Math.hypot(nx - ix, ny - iy) * 10);
      score += Math.random() * 10;
      if (score > bestScore) { bestScore = score; bestRoom = space; best = d; }
    }
    /* doomed: boxed in with nowhere worth going — die now, don't circle */
    if (!best || bestRoom < DOOM_SPACE) {
      crash(c);
      return;
    }
    if (best[0] !== dx || best[1] !== dy) {
      /* wall points are recorded at turns only */
      c.wall.push([c.x, c.y]);
      c.dir = best.slice();
      c.boost = 0;
    }
  };

  const crash = (c) => {
    c.alive = false;
    c.respawnIn = rand(1.2, 2.6);
    /* close the polyline at the death point, then keep the wall on the
       field as debris */
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

  const clamp = (v, lo, hi) => Math.max(lo, Math.min(hi, v));

  /* respawn at the roomiest spot on the field; if the whole field is
     walled in, stay dead and wait for the round reset */
  const respawn = (c) => {
    const g = buildGrid(null);
    let bestSpace = -1, bx = 0, by = 0;
    for (let i = 0; i < 12; i++) {
      const x = rand(X_MIN * 0.6, X_MAX * 0.6);
      const y = rand(Y_MIN + 2, Y_MAX - 2);
      const idx = cellOf(x, y);
      const space = idx < 0 ? 0 : flood(g, idx, FLOOD_CAP);
      if (space > bestSpace) { bestSpace = space; bx = x; by = y; }
      if (space >= FLOOD_CAP) break;
    }
    if (bestSpace < 150) {
      c.respawnIn = 1.2;
      return;
    }
    const fresh = spawn(c.color);
    fresh.x = bx;
    fresh.y = by;
    fresh.wall.push([bx, by]);
    Object.assign(c, fresh);
  };

  const update = (dt) => {
    for (const c of cycles) {
      if (!c.alive) {
        c.respawnIn -= dt;
        if (c.respawnIn <= 0) respawn(c);
        continue;
      }
      /* tron rule: drifting in a straight line builds speed; turning resets */
      c.boost = Math.min(BOOST_MAX, c.boost + dt);
      const mult = 1 + c.boost * BOOST_GAIN;
      c.x += c.dir[0] * SPEED_K * mult * dt;
      c.y += c.dir[1] * SPEED_T * T_SCALE * mult * dt;
      if (outOfBounds(c.x, c.y)) {
        /* ran into the arena wall: the trail ends at the boundary */
        c.x = clamp(c.x, X_MIN, X_MAX);
        c.y = clamp(c.y, Y_MIN, Y_MAX);
        c.wall.push([c.x, c.y]);
        crash(c);
        continue;
      }
    }
  };

  /* wall height in screen px for a floor point at normalized depth y;
     walls rise off the grid and grow as they come toward the viewer */
  const heightAt = (y) => {
    const t = y / T_SCALE;
    return (9 + 24 * t) * scale;
  };

  const draw = (now) => {
    ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
    ctx.clearRect(0, 0, vw, vh);
    ctx.lineJoin = 'miter';

    /* collect wall segments (live cycles + debris), far first for
       painter's order; the arena boundary stays invisible */
    const segs = [];
    for (const d of debris) {
      const w = d.wall;
      for (let i = 0; i < w.length - 1; i++) {
        segs.push({ a: w[i], b: w[i + 1], color: d.color, alpha: 0.45 });
      }
    }
    for (const c of cycles) {
      const w = c.wall;
      for (let i = 0; i < w.length - 1; i++) {
        segs.push({ a: w[i], b: w[i + 1], color: c.color, alpha: 1 });
      }
      /* the current run: from the last turn to the head */
      if (c.alive && w.length) {
        segs.push({ a: w[w.length - 1], b: [c.x, c.y], color: c.color, alpha: 1 });
      }
    }
    segs.sort((s1, s2) => (s1.a[1] + s1.b[1]) - (s2.a[1] + s2.b[1]));

    /* 3D extrusion: each segment is a quad from the floor up to a top edge */
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
      ctx.globalAlpha = 0.13 * s.alpha;
      ctx.lineWidth = 6;
      ctx.stroke();
      ctx.globalAlpha = 0.8 * s.alpha;
      ctx.lineWidth = 2;
      ctx.stroke();
    }

    /* heads: bright tracking dot with a sonar pulse on top of the wall */
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
    update(dt);
    aiAcc += dt;
    if (aiAcc > 0.066) {
      cycles.forEach((c) => c.alive && think(c));
      aiAcc = 0;
    }
    roundAcc += dt * 1000;
    if (roundAcc > ROUND_MS) {
      cycles.forEach((c) => c.alive && crash(c));
      debris.length = 0;
      roundAcc = 0;
    }
    /* round over when everyone is dead, or when only one survivor remains —
       give the winner a short lap, then clear the field and start fresh */
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

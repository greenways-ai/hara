/* Hara grid game — ambient light cycles riding the perspective floor of the
   fixed background (.hara-bg). Cycles move in the floor plane, turn at right
   angles, leave persistent walls, avoid collisions, crash and respawn.
   Positions are kept in a normalized arena space (k, t*T_SCALE) and projected
   with the same fan geometry as grid-scene.html (vanishing point 720,461;
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
     (depth); the boundary is drawn as visible walls the cycles run into */
  const T_SCALE = 28;
  const X_MIN = -8, X_MAX = 8, Y_MIN = 0.28 * T_SCALE, Y_MAX = 0.92 * T_SCALE;
  const SPEED = 2.1;            // normalized units / s
  const EPS = 0.38;             // collision distance
  const LOOKAHEAD = 1.7;
  const ROUND_MS = 55000;
  const COLORS = ['#41f5e4', '#ff2e88', '#9c7bff'];
  const DIRS = [[1, 0], [-1, 0], [0, 1], [0, -1]];

  const rand = (a, b) => a + Math.random() * (b - a);

  const spawn = (color) => ({
    color,
    x: rand(X_MIN + 2.5, X_MAX - 2.5),
    y: rand(Y_MIN + 2.5, Y_MAX - 2.5),
    dir: DIRS[Math.floor(Math.random() * 4)].slice(),
    wall: [],
    alive: true,
    fade: 1,
    respawnIn: 0,
    fearless: false,
  });

  const cycles = COLORS.map(spawn);
  cycles.forEach((c) => c.wall.push([c.x, c.y]));

  /* distance from point p to segment ab */
  const segDist = (px, py, ax, ay, bx, by) => {
    const dx = bx - ax, dy = by - ay;
    const len2 = dx * dx + dy * dy;
    let u = len2 ? ((px - ax) * dx + (py - ay) * dy) / len2 : 0;
    u = Math.max(0, Math.min(1, u));
    const cx = ax + u * dx, cy = ay + u * dy;
    return Math.hypot(px - cx, py - cy);
  };

  const outOfBounds = (x, y) => x < X_MIN || x > X_MAX || y < Y_MIN || y > Y_MAX;

  /* cycle walls only; the arena boundary is separate so a fearless cycle
     can choose to run straight into it */
  const hitWall = (x, y, self) => {
    for (const c of cycles) {
      const w = c.wall;
      /* ignore the fresh end of our own wall */
      const end = c === self ? w.length - 8 : w.length - 1;
      for (let i = 0; i < end; i++) {
        const a = w[i], b = w[i + 1];
        if (!b) break;
        if (segDist(x, y, a[0], a[1], b[0], b[1]) < EPS) return true;
      }
    }
    return false;
  };

  const blockedAt = (x, y, self, fearless) =>
    (!fearless && outOfBounds(x, y)) || hitWall(x, y, self);

  const clearance = (x, y, dir, fearless) => {
    for (let d = 0.5; d <= 9; d += 0.5) {
      if (blockedAt(x + dir[0] * d, y + dir[1] * d, null, fearless)) return d;
    }
    return 9;
  };

  const think = (c) => {
    if (Math.random() < 0.03) c.fearless = !c.fearless;
    const [dx, dy] = c.dir;
    const blocked = blockedAt(c.x + dx * LOOKAHEAD, c.y + dy * LOOKAHEAD, c, c.fearless);
    const whim = Math.random() < 0.22;
    if (!blocked && !whim) return;
    const left = [dy, -dx];
    const right = [-dy, dx];
    const cl = clearance(c.x, c.y, left, c.fearless);
    const cr = clearance(c.x, c.y, right, c.fearless);
    if (blocked && cl < 0.8 && cr < 0.8) {
      crash(c);
      return;
    }
    c.dir = (cl >= cr ? left : right).slice();
  };

  const crash = (c) => {
    c.alive = false;
    c.respawnIn = rand(1.2, 2.6);
  };

  const clamp = (v, lo, hi) => Math.max(lo, Math.min(hi, v));

  const update = (dt) => {
    for (const c of cycles) {
      if (!c.alive) {
        c.fade = Math.max(0, c.fade - dt / 1.6);
        c.respawnIn -= dt;
        if (c.respawnIn <= 0) {
          const fresh = spawn(c.color);
          fresh.wall.push([fresh.x, fresh.y]);
          Object.assign(c, fresh);
        }
        continue;
      }
      c.x += c.dir[0] * SPEED * dt;
      c.y += c.dir[1] * SPEED * dt;
      if (outOfBounds(c.x, c.y)) {
        /* ran into the arena wall: the trail ends at the boundary */
        c.x = clamp(c.x, X_MIN, X_MAX);
        c.y = clamp(c.y, Y_MIN, Y_MAX);
        c.wall.push([c.x, c.y]);
        crash(c);
        continue;
      }
      const w = c.wall;
      const last = w[w.length - 1];
      if (Math.hypot(c.x - last[0], c.y - last[1]) > 0.08) w.push([c.x, c.y]);
      if (w.length > 1600) w.splice(0, 200);
    }
  };

  const draw = () => {
    ctx.setTransform(dpr, 0, 0, dpr, 0, 0);
    ctx.clearRect(0, 0, vw, vh);
    ctx.lineJoin = 'miter';
    /* arena boundary walls, under the cycle trails */
    ctx.beginPath();
    [[X_MIN, Y_MIN], [X_MAX, Y_MIN], [X_MAX, Y_MAX], [X_MIN, Y_MAX], [X_MIN, Y_MIN]]
      .forEach(([x, y], i) => {
        const [sx, sy] = project(x, y / T_SCALE);
        i ? ctx.lineTo(sx, sy) : ctx.moveTo(sx, sy);
      });
    ctx.strokeStyle = '#8ffff2';
    ctx.globalAlpha = 0.08;
    ctx.lineWidth = 5;
    ctx.stroke();
    ctx.globalAlpha = 0.4;
    ctx.lineWidth = 1.4;
    ctx.stroke();
    for (const c of cycles) {
      const alpha = c.fade;
      if (alpha <= 0 || c.wall.length < 2) continue;
      ctx.beginPath();
      c.wall.forEach(([x, y], i) => {
        const [sx, sy] = project(x, y / T_SCALE);
        i ? ctx.lineTo(sx, sy) : ctx.moveTo(sx, sy);
      });
      ctx.strokeStyle = c.color;
      ctx.globalAlpha = 0.07 * alpha;
      ctx.lineWidth = 4.5;
      ctx.stroke();
      ctx.globalAlpha = 0.45 * alpha;
      ctx.lineWidth = 1.3;
      ctx.stroke();
      if (c.alive) {
        const [hx, hy] = project(c.x, c.y / T_SCALE);
        ctx.globalAlpha = 0.9;
        ctx.shadowColor = c.color;
        ctx.shadowBlur = 8;
        ctx.fillStyle = c.color;
        ctx.beginPath();
        ctx.arc(hx, hy, 2.6, 0, Math.PI * 2);
        ctx.fill();
        ctx.shadowBlur = 0;
      }
    }
    ctx.globalAlpha = 1;
  };

  let last = performance.now();
  let aiAcc = 0, roundAcc = 0;

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
      roundAcc = 0;
    }
    draw();
    window.requestAnimationFrame(frame);
  };
  window.requestAnimationFrame(frame);
})();

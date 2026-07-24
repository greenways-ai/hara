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
    const x = 720 + k * (36 + 224 * t);
    const y = 461 + 349 * t;
    return [offX + x * scale, offY + y * scale];
  };

  /* arena in normalized units: x = k (lateral), y = t * T_SCALE (depth) */
  const T_SCALE = 28;
  const X_MIN = -13, X_MAX = 13, Y_MIN = 0.06 * T_SCALE, Y_MAX = 0.98 * T_SCALE;
  const SPEED = 2.6;            // normalized units / s
  const EPS = 0.38;             // collision distance
  const LOOKAHEAD = 1.7;
  const ROUND_MS = 55000;
  const COLORS = ['#41f5e4', '#ff2e88', '#9c7bff'];
  const DIRS = [[1, 0], [-1, 0], [0, 1], [0, -1]];

  const rand = (a, b) => a + Math.random() * (b - a);

  const spawn = (color) => ({
    color,
    x: rand(X_MIN + 3, X_MAX - 3),
    y: rand(Y_MIN + 3, Y_MAX - 3),
    dir: DIRS[Math.floor(Math.random() * 4)].slice(),
    wall: [],
    alive: true,
    fade: 1,
    respawnIn: 0,
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

  const hitWall = (x, y, self) => {
    if (x < X_MIN || x > X_MAX || y < Y_MIN || y > Y_MAX) return true;
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

  const clearance = (x, y, dir) => {
    for (let d = 0.5; d <= 9; d += 0.5) {
      if (hitWall(x + dir[0] * d, y + dir[1] * d, null)) return d;
    }
    return 9;
  };

  const think = (c) => {
    const [dx, dy] = c.dir;
    const blocked = hitWall(c.x + dx * LOOKAHEAD, c.y + dy * LOOKAHEAD, c);
    const whim = Math.random() < 0.22;
    if (!blocked && !whim) return;
    const left = [dy, -dx];
    const right = [-dy, dx];
    const cl = clearance(c.x, c.y, left);
    const cr = clearance(c.x, c.y, right);
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
    for (const c of cycles) {
      const alpha = c.fade;
      if (alpha <= 0 || c.wall.length < 2) continue;
      ctx.beginPath();
      c.wall.forEach(([x, y], i) => {
        const [sx, sy] = project(x, y / T_SCALE);
        i ? ctx.lineTo(sx, sy) : ctx.moveTo(sx, sy);
      });
      ctx.strokeStyle = c.color;
      ctx.globalAlpha = 0.15 * alpha;
      ctx.lineWidth = 5.5;
      ctx.stroke();
      ctx.globalAlpha = 0.85 * alpha;
      ctx.lineWidth = 1.6;
      ctx.stroke();
      if (c.alive) {
        const [hx, hy] = project(c.x, c.y / T_SCALE);
        ctx.globalAlpha = 1;
        ctx.shadowColor = c.color;
        ctx.shadowBlur = 14;
        ctx.fillStyle = '#eafcff';
        ctx.beginPath();
        ctx.arc(hx, hy, 3.2, 0, Math.PI * 2);
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

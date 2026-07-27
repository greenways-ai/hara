const COLORS = ["#41f5e4", "#ff2e88", "#9c7bff", "#f5d742"];
const DIRECTIONS = [[1, 0], [0, 1], [-1, 0], [0, -1]];
const GRID = 36;

function choose(options) {
  return options[Math.floor(Math.random() * options.length)];
}

function createCycle(index) {
  const corners = [
    [3, 3, 0],
    [GRID - 3, 3, 2],
    [3, GRID - 3, 0],
    [GRID - 3, GRID - 3, 2]
  ];
  const [x, y, direction] = corners[index];
  return {
    x,
    y,
    direction,
    color: COLORS[index],
    trail: [[x, y]],
    nextTurn: 12 + Math.random() * 20
  };
}

export function startTron(canvas) {
  const context = canvas?.getContext("2d");
  if (!context) return () => {};

  let width = 0;
  let height = 0;
  let ratio = 1;
  let step = 0;
  let raf = 0;
  let last = performance.now();
  let accumulator = 0;
  let cycles = COLORS.map((_, index) => createCycle(index));

  function resize() {
    width = innerWidth;
    height = innerHeight;
    ratio = Math.min(2, devicePixelRatio || 1);
    canvas.width = Math.round(width * ratio);
    canvas.height = Math.round(height * ratio);
  }

  function occupied(x, y, except) {
    return cycles.some((cycle) =>
      cycle !== except && cycle.trail.some(([trailX, trailY]) =>
        Math.abs(trailX - x) < .35 && Math.abs(trailY - y) < .35
      )
    );
  }

  function optionsFor(cycle) {
    return DIRECTIONS
      .map((_, direction) => direction)
      .filter((direction) => (direction + 2) % 4 !== cycle.direction)
      .filter((direction) => {
        const [dx, dy] = DIRECTIONS[direction];
        const x = cycle.x + dx * 2;
        const y = cycle.y + dy * 2;
        return x > 1 && x < GRID - 1 && y > 1 && y < GRID - 1 && !occupied(x, y, cycle);
      });
  }

  function update() {
    step += 1;
    for (const cycle of cycles) {
      cycle.nextTurn -= 1;
      const [dx, dy] = DIRECTIONS[cycle.direction];
      const lookX = cycle.x + dx * 3;
      const lookY = cycle.y + dy * 3;
      const blocked = lookX < 1 || lookX > GRID - 1 || lookY < 1 || lookY > GRID - 1 ||
        occupied(lookX, lookY, cycle);
      if (blocked || cycle.nextTurn <= 0) {
        const options = optionsFor(cycle);
        if (options.length) cycle.direction = choose(options);
        cycle.nextTurn = 9 + Math.random() * 24;
        cycle.trail.push([cycle.x, cycle.y]);
      }
      const [nextX, nextY] = DIRECTIONS[cycle.direction];
      cycle.x += nextX * .28;
      cycle.y += nextY * .28;
      if (cycle.x < 0 || cycle.x > GRID || cycle.y < 0 || cycle.y > GRID) {
        Object.assign(cycle, createCycle(COLORS.indexOf(cycle.color)));
      }
      const head = cycle.trail[cycle.trail.length - 1];
      if (Math.hypot(cycle.x - head[0], cycle.y - head[1]) > 4) {
        cycle.trail.push([cycle.x, cycle.y]);
      }
      if (cycle.trail.length > 26) cycle.trail.shift();
    }
    if (step % 1200 === 0) cycles = COLORS.map((_, index) => createCycle(index));
  }

  function project(x, y) {
    const scale = Math.min(width / 58, height / 34);
    return [
      width * .5 + (x - y) * scale,
      height * .12 + (x + y) * scale * .42
    ];
  }

  function drawGrid() {
    context.strokeStyle = "rgba(65,245,228,.055)";
    context.lineWidth = 1;
    for (let value = 0; value <= GRID; value += 3) {
      context.beginPath();
      context.moveTo(...project(value, 0));
      context.lineTo(...project(value, GRID));
      context.stroke();
      context.beginPath();
      context.moveTo(...project(0, value));
      context.lineTo(...project(GRID, value));
      context.stroke();
    }
  }

  function drawCycle(cycle) {
    const points = [...cycle.trail, [cycle.x, cycle.y]].map(([x, y]) => project(x, y));
    if (points.length < 2) return;
    context.beginPath();
    context.moveTo(...points[0]);
    for (const point of points.slice(1)) context.lineTo(...point);
    context.strokeStyle = cycle.color;
    context.globalAlpha = .18;
    context.lineWidth = 11;
    context.stroke();
    context.globalAlpha = .82;
    context.lineWidth = 2;
    context.stroke();
    const [headX, headY] = points[points.length - 1];
    context.shadowColor = cycle.color;
    context.shadowBlur = 18;
    context.fillStyle = "#efffff";
    context.beginPath();
    context.arc(headX, headY, 3, 0, Math.PI * 2);
    context.fill();
    context.shadowBlur = 0;
  }

  function frame(now) {
    const elapsed = Math.min(60, now - last);
    last = now;
    accumulator += elapsed;
    while (accumulator >= 30) {
      update();
      accumulator -= 30;
    }
    context.setTransform(ratio, 0, 0, ratio, 0, 0);
    context.clearRect(0, 0, width, height);
    const gradient = context.createLinearGradient(0, 0, 0, height);
    gradient.addColorStop(0, "#010307");
    gradient.addColorStop(.65, "#040914");
    gradient.addColorStop(1, "#07111d");
    context.fillStyle = gradient;
    context.fillRect(0, 0, width, height);
    drawGrid();
    for (const cycle of cycles) drawCycle(cycle);
    context.globalAlpha = 1;
    raf = requestAnimationFrame(frame);
  }

  resize();
  addEventListener("resize", resize);
  if (!matchMedia("(prefers-reduced-motion: reduce)").matches) {
    raf = requestAnimationFrame(frame);
  } else {
    context.setTransform(ratio, 0, 0, ratio, 0, 0);
    context.fillStyle = "#020408";
    context.fillRect(0, 0, width, height);
    drawGrid();
  }

  return () => {
    cancelAnimationFrame(raf);
    removeEventListener("resize", resize);
  };
}

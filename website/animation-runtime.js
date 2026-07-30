export class AnimationRuntime {
  constructor(canvas, { onFrame = () => {}, onAction = () => {} } = {}) {
    this.canvas = canvas;
    this.context = canvas.getContext("2d");
    this.onFrame = onFrame;
    this.onAction = onAction;
    this.plan = null;
    this.running = false;
    this.started = 0;
    this.frame = 0;
    this.handle = 0;
    this.resize();
  }

  setPlan(plan) {
    this.plan = structuredClone(plan);
    this.started = performance.now();
    this.frame = 0;
    this.draw(this.started);
  }

  play() {
    if (!this.plan) return;
    this.running = true;
    this.started = performance.now();
    cancelAnimationFrame(this.handle);
    this.handle = requestAnimationFrame((time) => this.tick(time));
  }

  pause() {
    this.running = false;
    cancelAnimationFrame(this.handle);
  }

  tick(time) {
    this.draw(time);
    if (this.running) this.handle = requestAnimationFrame((next) => this.tick(next));
  }

  draw(time) {
    if (!this.plan) return;
    this.resize();
    const elapsed = Math.max(0, time - this.started);
    const duration = 1100;
    const index = Math.floor(elapsed / duration) % this.plan.actions.length;
    const progress = (elapsed % duration) / duration;
    const action = this.plan.actions[index];
    const ctx = this.context;
    const width = this.canvas.width;
    const height = this.canvas.height;
    ctx.clearRect(0, 0, width, height);
    ctx.fillStyle = "#030811";
    ctx.fillRect(0, 0, width, height);
    grid(ctx, width, height);
    drawCharacter(ctx, this.plan.selected, action, progress, width / 2, height * 0.64);
    ctx.fillStyle = "#8ca4af";
    ctx.font = `${Math.max(12, width / 55)}px ui-monospace`;
    ctx.fillText(`${this.plan.selected.toUpperCase()} / ${action.toUpperCase()}`, 22, 32);
    this.frame += 1;
    this.onFrame({ frame: this.frame, action, index, progress });
    this.onAction({ action, index });
  }

  resize() {
    const ratio = devicePixelRatio || 1;
    const width = Math.max(320, this.canvas.clientWidth || 800);
    const height = Math.max(260, this.canvas.clientHeight || 480);
    if (this.canvas.width !== Math.round(width * ratio) ||
        this.canvas.height !== Math.round(height * ratio)) {
      this.canvas.width = Math.round(width * ratio);
      this.canvas.height = Math.round(height * ratio);
      this.context.setTransform(ratio, 0, 0, ratio, 0, 0);
    }
  }
}

function grid(ctx, width, height) {
  ctx.strokeStyle = "rgba(65,245,228,.08)";
  ctx.lineWidth = 1;
  for (let x = 0; x < width; x += 32) {
    ctx.beginPath(); ctx.moveTo(x, 0); ctx.lineTo(x, height); ctx.stroke();
  }
  for (let y = 0; y < height; y += 32) {
    ctx.beginPath(); ctx.moveTo(0, y); ctx.lineTo(width, y); ctx.stroke();
  }
  ctx.strokeStyle = "rgba(65,245,228,.35)";
  ctx.beginPath(); ctx.moveTo(0, height * .82); ctx.lineTo(width, height * .82); ctx.stroke();
}

function drawCharacter(ctx, character, action, progress, x, y) {
  const pulse = Math.sin(progress * Math.PI * 2);
  const jump = action === "jump" ? Math.sin(progress * Math.PI) * 90 : 0;
  const travel = action === "walk" ? (progress - .5) * 130 : 0;
  const spin = action === "spin" ? progress * Math.PI * 2 : 0;
  const bow = action === "bow" ? Math.sin(progress * Math.PI) * .55 : 0;
  ctx.save();
  ctx.translate(x + travel, y - jump);
  ctx.rotate(spin + bow);
  ctx.strokeStyle = character === "mage" ? "#9c7bff" : character === "fox" ? "#f2ce4e" : "#41f5e4";
  ctx.fillStyle = "#071721";
  ctx.lineWidth = 7;
  ctx.lineCap = "round";
  ctx.beginPath(); ctx.arc(0, -92, 31, 0, Math.PI * 2); ctx.fill(); ctx.stroke();
  ctx.beginPath(); ctx.moveTo(0, -60); ctx.lineTo(0, 20); ctx.stroke();
  const wave = action === "wave" ? pulse * .8 : 0;
  limb(ctx, 0, -42, -42, -10, -.35);
  limb(ctx, 0, -42, 45, -34, wave - .5);
  limb(ctx, 0, 20, -30, 74, action === "walk" ? pulse * .45 : 0);
  limb(ctx, 0, 20, 30, 74, action === "walk" ? -pulse * .45 : 0);
  if (character === "mage") {
    ctx.beginPath(); ctx.moveTo(-38, -119); ctx.lineTo(0, -168); ctx.lineTo(38, -119); ctx.closePath(); ctx.fill(); ctx.stroke();
  } else if (character === "fox") {
    ctx.beginPath(); ctx.moveTo(-25, -116); ctx.lineTo(-8, -151); ctx.lineTo(0, -119);
    ctx.moveTo(25, -116); ctx.lineTo(8, -151); ctx.lineTo(0, -119); ctx.stroke();
  } else {
    ctx.fillStyle = ctx.strokeStyle; ctx.fillRect(-15, -99, 9, 9); ctx.fillRect(7, -99, 9, 9);
  }
  ctx.restore();
}

function limb(ctx, x, y, dx, dy, angle) {
  ctx.save(); ctx.translate(x, y); ctx.rotate(angle);
  ctx.beginPath(); ctx.moveTo(0, 0); ctx.lineTo(dx, dy); ctx.stroke(); ctx.restore();
}

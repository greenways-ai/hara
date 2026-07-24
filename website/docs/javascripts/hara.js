document.addEventListener('DOMContentLoaded', () => {
  document.querySelectorAll('pre code').forEach((block) => {
    block.addEventListener('click', () => block.classList.add('hara-code-touched'), {once: true});
  });
});

// Parallax for the moon scenescape hero: layers carry --hara-depth (larger =
// farther = moves less); this component writes --hara-mx/--hara-my (pointer,
// -1..1) and --hara-scroll (0..1) on the container and CSS does the rest.
(() => {
  const scene = document.querySelector('[data-hara-component="parallax"]');
  if (!scene) return;
  if (window.matchMedia('(prefers-reduced-motion: reduce)').matches) return;

  const coarse = window.matchMedia('(pointer: coarse)').matches;
  let targetX = 0, targetY = 0, targetScroll = 0;
  let currentX = 0, currentY = 0, currentScroll = 0;
  let raf = null;

  const tick = () => {
    currentX += (targetX - currentX) * 0.08;
    currentY += (targetY - currentY) * 0.08;
    currentScroll += (targetScroll - currentScroll) * 0.08;
    scene.style.setProperty('--hara-mx', currentX.toFixed(4));
    scene.style.setProperty('--hara-my', currentY.toFixed(4));
    scene.style.setProperty('--hara-scroll', currentScroll.toFixed(4));
    const settled =
      Math.abs(targetX - currentX) < 0.001 &&
      Math.abs(targetY - currentY) < 0.001 &&
      Math.abs(targetScroll - currentScroll) < 0.001;
    raf = settled ? null : window.requestAnimationFrame(tick);
  };

  const kick = () => {
    if (raf === null) raf = window.requestAnimationFrame(tick);
  };

  if (!coarse) {
    window.addEventListener('pointermove', (event) => {
      targetX = (event.clientX / window.innerWidth - 0.5) * 2;
      targetY = (event.clientY / window.innerHeight - 0.5) * 2;
      kick();
    }, {passive: true});
  }

  const readScroll = () => {
    const rect = scene.getBoundingClientRect();
    targetScroll = Math.min(1, Math.max(0, -rect.top / (rect.height || 1)));
    kick();
  };
  window.addEventListener('scroll', readScroll, {passive: true});
  readScroll();
})();

// Home-page demos — two wasm sound demos, both started by a user gesture
// (browser autoplay policy) and torn down on STOP.
//
//   [data-hara-demo="synth"]  — our own demo-synth.wasm, self-hosted under
//                               assets/wasm/ (built by scripts/build-demo-synth-wasm).
//                               Pre-renders a loop into an AudioBuffer and draws
//                               waveform + spectrum from an AnalyserNode.
//   [data-hara-demo="csound"] — @csound/browser imported from the jsdelivr CDN
//                               ("a wasm from the internet"). A JS scheduler
//                               feeds it a pentatonic pattern; the same event
//                               stream drives the canvas.
//
// Panel contract (see index.md):
//   [data-demo-run]            run/stop button
//   [data-demo-canvas]         visualization canvas
//   <b data-status="...">      status strip slots

(() => {
  const CYAN = '#41f5e4';
  const GREEN = '#3dff9e';
  const FAINT = 'rgba(91, 107, 124, .35)';

  // A-minor pentatonic, 16 eighth-note steps at 120 BPM — the same pattern the
  // demo-synth wasm plays, so both demos agree.
  const PATTERN = [0, 7, 12, 15, 19, 15, 12, 7, 0, 7, 12, 17, 19, 22, 19, 15];
  const STEP_MS = 250;

  const freq = (base, semis) => base * Math.pow(2, semis / 12);

  const setStatus = (root, key, value) => {
    const slot = root.querySelector(`[data-status="${key}"]`);
    if (slot) slot.textContent = value;
  };

  const fitCanvas = (canvas) => {
    const dpr = window.devicePixelRatio || 1;
    const w = canvas.clientWidth, h = canvas.clientHeight;
    if (canvas.width !== w * dpr || canvas.height !== h * dpr) {
      canvas.width = w * dpr;
      canvas.height = h * dpr;
    }
    const g = canvas.getContext('2d');
    g.setTransform(dpr, 0, 0, dpr, 0, 0);
    return [g, w, h];
  };

  const frameGrid = (g, w, h) => {
    g.clearRect(0, 0, w, h);
    g.strokeStyle = FAINT;
    g.lineWidth = 1;
    g.beginPath();
    g.moveTo(0, h / 2 + .5);
    g.lineTo(w, h / 2 + .5);
    g.stroke();
  };

  /* ------------------------------------------------------- demo 1: synth */

  const startSynth = async (root) => {
    const canvas = root.querySelector('[data-demo-canvas]');
    setStatus(root, 'wasm', 'LOADING');
    const res = await fetch('assets/wasm/demo-synth.wasm');
    if (!res.ok) throw new Error(`fetch demo-synth.wasm: HTTP ${res.status}`);
    const { instance } = await WebAssembly.instantiate(await res.arrayBuffer(), {});
    const x = instance.exports;

    const ctx = new AudioContext();
    const sr = ctx.sampleRate;
    // Two 4 s cycles; any whole number of seconds is loop-seamless and keeps
    // the buffer an integer number of samples at any sample rate.
    const frames = Math.round(sr * 8);
    const buffer = ctx.createBuffer(1, frames, sr);
    const data = buffer.getChannelData(0);
    const cap = x.synth_capacity();
    const view = new Float32Array(x.memory.buffer, x.synth_buffer(), cap);
    for (let off = 0; off < frames; off += cap) {
      const n = Math.min(x.synth_fill(BigInt(off), cap, sr), frames - off);
      data.set(view.subarray(0, n), off);
    }
    setStatus(root, 'wasm', 'LOADED');

    const analyser = ctx.createAnalyser();
    analyser.fftSize = 2048;
    const src = ctx.createBufferSource();
    src.buffer = buffer;
    src.loop = true;
    src.connect(analyser);
    analyser.connect(ctx.destination);
    src.start();
    setStatus(root, 'audio', 'RUNNING');
    setStatus(root, 'state', 'LOOP · 8S');

    const wave = new Uint8Array(analyser.fftSize);
    const spec = new Uint8Array(analyser.frequencyBinCount);
    let raf = null;
    const draw = () => {
      const [g, w, h] = fitCanvas(canvas);
      frameGrid(g, w, h);

      analyser.getByteFrequencyData(spec);
      const bars = 56, bw = w / bars;
      g.fillStyle = 'rgba(65, 245, 228, .28)';
      for (let i = 0; i < bars; i++) {
        const v = spec[Math.floor(i * spec.length / bars / 2)] / 255;
        const bh = v * h * .45;
        g.fillRect(i * bw + 1, h - bh, bw - 2, bh);
      }

      analyser.getByteTimeDomainData(wave);
      g.strokeStyle = CYAN;
      g.lineWidth = 1.6;
      g.shadowColor = CYAN;
      g.shadowBlur = 12;
      g.beginPath();
      for (let i = 0; i < wave.length; i++) {
        const px = i / (wave.length - 1) * w;
        const py = h / 2 + (wave[i] - 128) / 128 * h * .42;
        i ? g.lineTo(px, py) : g.moveTo(px, py);
      }
      g.stroke();
      g.shadowBlur = 0;
      raf = requestAnimationFrame(draw);
    };
    draw();

    return async () => {
      cancelAnimationFrame(raf);
      try { src.stop(); } catch (_) { /* already stopped */ }
      await ctx.close();
      const [g, w, h] = fitCanvas(canvas);
      frameGrid(g, w, h);
    };
  };

  /* ----------------------------------------------------- demo 2: csound */

  const CSOUND_URL = 'https://cdn.jsdelivr.net/npm/@csound/browser@6.18.7/dist/csound.js';

  const CSD = `
<CsoundSynthesizer>
<CsOptions>
-odac -d
</CsOptions>
<CsInstruments>
sr = 44100
ksmps = 32
nchnls = 2
0dbfs = 1

instr 1
  aenv = expon(1, p3, 0.001)
  asig = poscil(aenv * 0.22, p4)
  outs(asig, asig)
endin

instr 2
  aenv = linen(0.28, 0.02, p3, 0.2)
  asig = poscil(aenv, p4)
  outs(asig, asig)
endin
</CsInstruments>
<CsScore>
f 1 0 16384 10 1
f 0 3600
</CsScore>
</CsoundSynthesizer>
`;

  const startCsound = async (root) => {
    const canvas = root.querySelector('[data-demo-canvas]');
    setStatus(root, 'wasm', 'CDN · LOADING');
    const { Csound } = await import(CSOUND_URL);
    const cs = await Csound();
    await cs.compileCsdText(CSD);
    setStatus(root, 'wasm', 'CDN · LOADED');
    await cs.start();
    setStatus(root, 'audio', 'RUNNING');

    const send = (msg) =>
      cs.inputMessageAsync ? cs.inputMessageAsync(msg) : cs.inputMessage(msg);

    // event-driven visualization: every scheduled note lands here
    const events = [];
    let step = 0;
    const tick = () => {
      const semi = PATTERN[step % PATTERN.length];
      send(`i 1 0 0.28 ${freq(440, semi).toFixed(2)}`);
      events.push({ kind: 'arp', step: step % 16, semi, at: performance.now() });
      if (step % 8 === 0) {
        send('i 2 0 1.9 110');
        events.push({ kind: 'bass', step: step % 16, semi: -12, at: performance.now() });
      }
      if (events.length > 64) events.splice(0, events.length - 64);
      step++;
    };
    tick();
    const timer = setInterval(tick, STEP_MS);
    setStatus(root, 'state', 'PATTERN · 16 STEPS');

    let raf = null;
    const draw = () => {
      const [g, w, h] = fitCanvas(canvas);
      g.clearRect(0, 0, w, h);
      const colW = w / 16;
      const now = performance.now();

      g.strokeStyle = FAINT;
      g.lineWidth = 1;
      for (let i = 1; i < 16; i++) {
        g.beginPath();
        g.moveTo(i * colW + .5, 0);
        g.lineTo(i * colW + .5, h);
        g.stroke();
      }
      // playhead over the current step
      g.fillStyle = 'rgba(65, 245, 228, .08)';
      g.fillRect((step % 16) * colW, 0, colW, h);

      for (const ev of events) {
        const age = (now - ev.at) / 700;
        if (age > 1) continue;
        const y = ev.kind === 'bass'
          ? h - 14
          : h - 24 - (ev.semi / 24) * (h - 48);
        const r = (ev.kind === 'bass' ? 5 : 3.5) * (1 - age * .4);
        g.globalAlpha = 1 - age;
        g.fillStyle = ev.kind === 'bass' ? GREEN : CYAN;
        g.shadowColor = g.fillStyle;
        g.shadowBlur = 14;
        g.beginPath();
        g.arc(ev.step * colW + colW / 2, y, r, 0, Math.PI * 2);
        g.fill();
      }
      g.globalAlpha = 1;
      g.shadowBlur = 0;
      raf = requestAnimationFrame(draw);
    };
    draw();

    return async () => {
      clearInterval(timer);
      cancelAnimationFrame(raf);
      try { await cs.stop(); } catch (_) { /* engine already down */ }
      try { if (cs.terminate) await cs.terminate(); } catch (_) { /* ignore */ }
      const [g, w, h] = fitCanvas(canvas);
      g.clearRect(0, 0, w, h);
    };
  };

  /* ------------------------------------------------------------ wiring */

  const ENGINES = { synth: startSynth, csound: startCsound };

  document.querySelectorAll('[data-hara-demo]').forEach((root) => {
    const start = ENGINES[root.dataset.haraDemo];
    const button = root.querySelector('[data-demo-run]');
    if (!start || !button) return;

    let stop = null;
    let busy = false;

    button.addEventListener('click', async () => {
      if (busy) return;
      busy = true;
      try {
        if (stop) {
          await stop();
          stop = null;
          button.textContent = 'Run';
          setStatus(root, 'audio', 'IDLE');
          setStatus(root, 'state', 'IDLE');
        } else {
          stop = await start(root);
          button.textContent = 'Stop';
        }
      } catch (err) {
        console.error('[hara demo]', err);
        setStatus(root, 'state', 'ERROR');
        stop = null;
        button.textContent = 'Run';
      } finally {
        busy = false;
      }
    });
  });
})();

//! hara demo synth — standalone wasm module for the website home-page demo.
//!
//! Plain C ABI, no hta machinery: JS fetches the .wasm, pre-renders a loop by
//! calling `synth_fill` in chunks, and plays it through WebAudio. The piece is
//! a deterministic function of the sample index: a minor-pentatonic arpeggio
//! over a low drone, periodic in `STEP * PATTERN` seconds so the loop is
//! seamless when the JS-side buffer length is a multiple of the cycle.
//!
//! All math is f64 internally: absolute sample times grow large, and f32
//! phase arithmetic drifts audibly across the loop boundary.

const CAPACITY: usize = 4096;

static mut BUFFER: [f32; CAPACITY] = [0.0; CAPACITY];

/// Eighth note at 120 BPM.
const STEP: f64 = 0.25;

/// A-minor pentatonic around A3, 16 steps = one 4 s cycle.
const PATTERN: [i32; 16] = [
    0, 7, 12, 15, 19, 15, 12, 7,
    0, 7, 12, 17, 19, 22, 19, 15,
];

fn freq(semitones_above_a3: i32) -> f64 {
    220.0 * 2f64.powf(semitones_above_a3 as f64 / 12.0)
}

/// Naive sawtooth in [-1, 1); aliasing is part of the charm here.
fn saw(phase: f64) -> f64 {
    2.0 * (phase - (phase + 0.5).floor())
}

/// One arpeggio voice at time `t`: detuned dual saw with a decay envelope.
/// `step_offset` > 0 gives a stateless echo `step_offset` steps back. Notes
/// before t=0 wrap around the pattern (the loop runs forever), keeping the
/// piece periodic everywhere instead of fading in over the first cycle.
fn voice(t: f64, step_offset: i64, gain: f64) -> f64 {
    let steps_back = (t / STEP).floor() as i64 - step_offset;
    let idx = steps_back.rem_euclid(PATTERN.len() as i64) as usize;
    let since = t - (steps_back + step_offset) as f64 * STEP;
    let f = freq(PATTERN[idx]);
    let env = (-since * 6.0).exp();
    // phase is relative to note onset so every 16-step cycle is identical
    let body = (saw(f * since) + saw(f * 1.007 * since)) * 0.5;
    body * env * gain
}

fn sample_at(n: u64, sample_rate: f64) -> f32 {
    let t = n as f64 / sample_rate;
    // lead voice + a quieter echo three steps back (periodic, stateless)
    let arp = voice(t, 0, 0.6) + voice(t, 3, 0.22);
    // drone: A2 + A1 with a slow tremolo (110/55/0.5 Hz all divide the 4 s cycle)
    let tau = 2.0 * std::f64::consts::PI;
    let trem = 0.75 + 0.25 * (tau * 0.5 * t).sin();
    let drone = ((tau * 110.0 * t).sin() * 0.18 + (tau * 55.0 * t).sin() * 0.12) * trem;
    ((arp + drone).tanh() * 0.85) as f32
}

#[no_mangle]
pub extern "C" fn synth_buffer() -> *mut f32 {
    std::ptr::addr_of_mut!(BUFFER) as *mut f32
}

#[no_mangle]
pub extern "C" fn synth_capacity() -> usize {
    CAPACITY
}

/// Fill BUFFER with `frames` samples starting at absolute sample index
/// `start_sample`. `frames` is clamped to `synth_capacity()`; returns the
/// number of frames written.
#[no_mangle]
pub extern "C" fn synth_fill(start_sample: u64, frames: usize, sample_rate: f32) -> usize {
    let count = frames.min(CAPACITY);
    let buffer = unsafe { &mut *std::ptr::addr_of_mut!(BUFFER) };
    for (i, slot) in buffer.iter_mut().enumerate().take(count) {
        *slot = sample_at(start_sample + i as u64, sample_rate as f64);
    }
    count
}

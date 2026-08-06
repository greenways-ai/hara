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

use std::sync::{
    atomic::{AtomicU32, Ordering},
    Mutex,
};

const CAPACITY: usize = 4096;
const MAX_STEPS: usize = 64;

static mut BUFFER: [f32; CAPACITY] = [0.0; CAPACITY];
static WAVEFORM: AtomicU32 = AtomicU32::new(2);

#[derive(Clone, Copy)]
struct Tune {
    tempo: f64,
    root: i32,
    gate: f64,
    steps_per_beat: f64,
    len: usize,
    steps: [i32; MAX_STEPS],
    rests: [bool; MAX_STEPS],
}

const DEFAULT_STEPS: [i32; MAX_STEPS] = {
    let mut values = [0; MAX_STEPS];
    let pattern = [0, 7, 12, 15, 19, 15, 12, 7, 0, 7, 12, 17, 19, 22, 19, 15];
    let mut index = 0;
    while index < pattern.len() {
        values[index] = pattern[index];
        index += 1;
    }
    values
};

const DEFAULT_TUNE: Tune = Tune {
    tempo: 120.0,
    root: 57,
    gate: 0.72,
    steps_per_beat: 2.0,
    len: 16,
    steps: DEFAULT_STEPS,
    rests: [false; MAX_STEPS],
};

static ACTIVE_TUNE: Mutex<Tune> = Mutex::new(DEFAULT_TUNE);
static STAGED_TUNE: Mutex<Tune> = Mutex::new(DEFAULT_TUNE);

fn midi_freq(note: i32) -> f64 {
    440.0 * 2f64.powf((note as f64 - 69.0) / 12.0)
}

/// Naive sawtooth in [-1, 1); aliasing is part of the charm here.
fn saw(phase: f64) -> f64 {
    2.0 * (phase - (phase + 0.5).floor())
}

fn oscillator(phase: f64) -> f64 {
    let tau = 2.0 * std::f64::consts::PI;
    match WAVEFORM.load(Ordering::Relaxed) {
        0 => (tau * phase).sin(),
        1 => {
            if (tau * phase).sin() >= 0.0 {
                1.0
            } else {
                -1.0
            }
        }
        _ => saw(phase),
    }
}

/// One arpeggio voice at time `t`: detuned dual saw with a decay envelope.
/// `step_offset` > 0 gives a stateless echo `step_offset` steps back. Notes
/// before t=0 wrap around the pattern (the loop runs forever), keeping the
/// piece periodic everywhere instead of fading in over the first cycle.
fn voice(t: f64, step_offset: i64, gain: f64, tune: Tune) -> f64 {
    let step_seconds = 60.0 / tune.tempo / tune.steps_per_beat;
    let steps_back = (t / step_seconds).floor() as i64 - step_offset;
    let idx = steps_back.rem_euclid(tune.len as i64) as usize;
    if tune.rests[idx] {
        return 0.0;
    }
    let since = t - (steps_back + step_offset) as f64 * step_seconds;
    let f = midi_freq(tune.root + tune.steps[idx]);
    let gate_seconds = step_seconds * tune.gate;
    let env = if since <= gate_seconds {
        (-since * 6.0).exp()
    } else {
        0.0
    };
    // phase is relative to note onset so every 16-step cycle is identical
    let body = (oscillator(f * since) + oscillator(f * 1.007 * since)) * 0.5;
    body * env * gain
}

fn sample_at(n: u64, sample_rate: f64) -> f32 {
    let t = n as f64 / sample_rate;
    let tune = *ACTIVE_TUNE.lock().expect("active tune lock");
    // lead voice + a quieter echo three steps back (periodic, stateless)
    let arp = voice(t, 0, 0.6, tune) + voice(t, 3, 0.22, tune);
    // Drone follows the configured root at one and two octaves below.
    let tau = 2.0 * std::f64::consts::PI;
    let trem = 0.75 + 0.25 * (tau * 0.5 * t).sin();
    let drone = ((tau * midi_freq(tune.root - 12) * t).sin() * 0.18
        + (tau * midi_freq(tune.root - 24) * t).sin() * 0.12)
        * trem;
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

/// Select the oscillator used by subsequent fills: 0 sine, 1 square, 2 saw.
#[no_mangle]
pub extern "C" fn synth_set_waveform(waveform: u32) -> bool {
    if waveform > 2 {
        return false;
    }
    WAVEFORM.store(waveform, Ordering::Relaxed);
    true
}

#[no_mangle]
pub extern "C" fn synth_tune_begin() {
    let active = *ACTIVE_TUNE.lock().expect("active tune lock");
    *STAGED_TUNE.lock().expect("staged tune lock") = active;
}

#[no_mangle]
pub extern "C" fn synth_tune_set_tempo(tempo: f32) -> bool {
    if !(30.0..=300.0).contains(&tempo) {
        return false;
    }
    STAGED_TUNE.lock().expect("staged tune lock").tempo = tempo as f64;
    true
}

#[no_mangle]
pub extern "C" fn synth_tune_set_root(root: i32) -> bool {
    if !(0..=127).contains(&root) {
        return false;
    }
    STAGED_TUNE.lock().expect("staged tune lock").root = root;
    true
}

#[no_mangle]
pub extern "C" fn synth_tune_set_gate(gate: f32) -> bool {
    if !(0.0..=1.0).contains(&gate) {
        return false;
    }
    STAGED_TUNE.lock().expect("staged tune lock").gate = gate as f64;
    true
}

#[no_mangle]
pub extern "C" fn synth_tune_set_steps_per_beat(value: f32) -> bool {
    if !value.is_finite() || value <= 0.0 || value > 16.0 {
        return false;
    }
    STAGED_TUNE.lock().expect("staged tune lock").steps_per_beat = value as f64;
    true
}

#[no_mangle]
pub extern "C" fn synth_tune_set_length(length: usize) -> bool {
    if length == 0 || length > MAX_STEPS {
        return false;
    }
    STAGED_TUNE.lock().expect("staged tune lock").len = length;
    true
}

#[no_mangle]
pub extern "C" fn synth_tune_set_step(index: usize, note: i32, rest: bool) -> bool {
    if index >= MAX_STEPS || !(-48..=48).contains(&note) {
        return false;
    }
    let mut staged = STAGED_TUNE.lock().expect("staged tune lock");
    staged.steps[index] = note;
    staged.rests[index] = rest;
    true
}

#[no_mangle]
pub extern "C" fn synth_tune_commit() {
    let staged = *STAGED_TUNE.lock().expect("staged tune lock");
    *ACTIVE_TUNE.lock().expect("active tune lock") = staged;
}

#[no_mangle]
pub extern "C" fn synth_tune_revision_length() -> usize {
    ACTIVE_TUNE.lock().expect("active tune lock").len
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn waveform_changes_samples_and_rejects_unknown_values() {
        assert!(synth_set_waveform(0));
        let sine = sample_at(137, 48_000.0);
        assert!(synth_set_waveform(2));
        let saw = sample_at(137, 48_000.0);
        assert_ne!(sine, saw);
        assert!(!synth_set_waveform(3));
        assert_eq!(WAVEFORM.load(Ordering::Relaxed), 2);
    }

    #[test]
    fn tune_is_staged_and_committed_atomically() {
        synth_tune_begin();
        assert!(synth_tune_set_root(60));
        assert!(synth_tune_set_tempo(96.0));
        assert!(synth_tune_set_length(4));
        assert!(synth_tune_set_step(0, 0, false));
        assert!(synth_tune_set_step(1, 0, true));
        assert!(synth_tune_set_step(2, 7, false));
        assert!(synth_tune_set_step(3, 12, false));
        assert_eq!(ACTIVE_TUNE.lock().unwrap().root, 57);
        synth_tune_commit();
        assert_eq!(ACTIVE_TUNE.lock().unwrap().root, 60);
        assert_eq!(synth_tune_revision_length(), 4);
        assert!(ACTIVE_TUNE.lock().unwrap().rests[1]);
    }

    #[test]
    fn tune_rejects_out_of_range_values() {
        assert!(!synth_tune_set_root(128));
        assert!(!synth_tune_set_tempo(20.0));
        assert!(!synth_tune_set_length(65));
        assert!(!synth_tune_set_step(64, 0, false));
        assert!(!synth_tune_set_step(0, 49, false));
    }
}

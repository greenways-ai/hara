//! Deterministic browser FFT transform for the Hara Amp demonstration.
//!
//! This intentionally has a tiny C ABI and no allocator. The browser copies
//! mono PCM into INPUT, calls `fft_compute`, then reads magnitudes from OUTPUT.
//! A Hann window and a direct real DFT keep the reference implementation small
//! and auditable; the demo uses 1024 samples and 96 display bins.

const INPUT_CAPACITY: usize = 2048;
const OUTPUT_CAPACITY: usize = 512;

static mut INPUT: [f32; INPUT_CAPACITY] = [0.0; INPUT_CAPACITY];
static mut OUTPUT: [f32; OUTPUT_CAPACITY] = [0.0; OUTPUT_CAPACITY];

#[no_mangle]
pub extern "C" fn fft_input() -> *mut f32 {
    std::ptr::addr_of_mut!(INPUT) as *mut f32
}

#[no_mangle]
pub extern "C" fn fft_output() -> *mut f32 {
    std::ptr::addr_of_mut!(OUTPUT) as *mut f32
}

#[no_mangle]
pub extern "C" fn fft_input_capacity() -> usize {
    INPUT_CAPACITY
}

#[no_mangle]
pub extern "C" fn fft_output_capacity() -> usize {
    OUTPUT_CAPACITY
}

#[no_mangle]
pub extern "C" fn fft_compute(frames: usize, bins: usize) -> usize {
    let frames = frames.clamp(2, INPUT_CAPACITY);
    let bins = bins.min(OUTPUT_CAPACITY).min(frames / 2);
    let input = unsafe { &*std::ptr::addr_of!(INPUT) };
    let output = unsafe { &mut *std::ptr::addr_of_mut!(OUTPUT) };
    let tau = std::f32::consts::TAU;
    let scale = 2.0 / frames as f32;

    for (bin, slot) in output.iter_mut().enumerate().take(bins) {
        let mut real = 0.0;
        let mut imaginary = 0.0;
        for (index, sample) in input.iter().copied().enumerate().take(frames) {
            let window = 0.5 - 0.5 * (tau * index as f32 / (frames - 1) as f32).cos();
            let phase = tau * bin as f32 * index as f32 / frames as f32;
            real += sample * window * phase.cos();
            imaginary -= sample * window * phase.sin();
        }
        *slot = (real.mul_add(real, imaginary * imaginary)).sqrt() * scale;
    }
    for slot in output.iter_mut().take(OUTPUT_CAPACITY).skip(bins) {
        *slot = 0.0;
    }
    bins
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn identifies_a_bin_centred_sine() {
        let frames = 256;
        let target = 12;
        let input = unsafe { &mut *std::ptr::addr_of_mut!(INPUT) };
        for (index, slot) in input.iter_mut().enumerate().take(frames) {
            *slot = (std::f32::consts::TAU * target as f32 * index as f32 / frames as f32).sin();
        }
        assert_eq!(fft_compute(frames, 64), 64);
        let output = unsafe { &*std::ptr::addr_of!(OUTPUT) };
        let peak = output[..64]
            .iter()
            .enumerate()
            .max_by(|left, right| left.1.total_cmp(right.1))
            .map(|(index, _)| index);
        assert_eq!(peak, Some(target));
    }
}

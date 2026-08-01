use hara_wasm::Runtime;
use std::{hint::black_box, time::Instant};

fn main() {
    let samples = std::env::args()
        .nth(1)
        .and_then(|value| value.parse::<usize>().ok())
        .unwrap_or(25);
    let mut times = Vec::with_capacity(samples);
    for _ in 0..samples {
        let started = Instant::now();
        black_box(Runtime::new());
        times.push(started.elapsed().as_nanos());
    }
    times.sort_unstable();
    println!(
        "{{\"mode\":\"{}\",\"samples\":{},\"median_ns\":{},\"min_ns\":{}}}",
        if cfg!(feature = "bytecode-vm") {
            "bytecode"
        } else {
            "source"
        },
        samples,
        times[times.len() / 2],
        times[0]
    );
}

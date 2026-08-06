use hara_wasm::Runtime;
use std::hint::black_box;
use std::time::Instant;

const CASES: &[(&str, &str)] = &[
    ("arithmetic", "(+ 19 23)"),
    ("control", "(if (< 19 20) 42 0)"),
    ("functions", "((fn [x] (+ x 23)) 19)"),
    (
        "collections",
        "(get (assoc {:answer 41} :answer 42) :answer)",
    ),
    ("iterators", "(iter-next (iter-map (fn [x] (+ x 1)) [41]))"),
    (
        "loops",
        "(loop [x 0 acc 0] (if (< x 16) (recur (+ x 1) (+ acc x)) acc))",
    ),
    ("bitops", "(bit-xor (bit-or 19 42) (bit-and 19 42))"),
    ("error", "(/ 1 0)"),
];

fn main() {
    let iterations = std::env::args()
        .nth(1)
        .and_then(|value| value.parse().ok())
        .unwrap_or(1000usize);
    let warmup = 100usize;
    println!("case,iterations,warm_ns_per_call,status");
    for (name, source) in CASES {
        let mut runtime = Runtime::new();
        for _ in 0..warmup {
            let _ = black_box(runtime.eval_native(source));
        }
        let start = Instant::now();
        let mut failed = false;
        for _ in 0..iterations {
            let result = black_box(runtime.eval_native(source));
            if result.is_err() {
                failed = true;
            }
        }
        let nanos = start.elapsed().as_nanos() / iterations as u128;
        println!(
            "{name},{iterations},{nanos},{}",
            if failed { "error" } else { "value" }
        );
    }
}

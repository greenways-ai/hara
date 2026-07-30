use std::env;
use std::fs;
use std::sync::{Arc, Barrier, Mutex};
use std::time::{Instant, SystemTime, UNIX_EPOCH};

use hara_wasm::service::{FabricService, ServiceConfig};

#[derive(Clone, Copy)]
struct Options {
    rooms: usize,
    tasks_per_room: usize,
    payload_bytes: usize,
    clients: usize,
    shards: usize,
}

fn main() {
    let options = options().unwrap_or_else(|error| {
        eprintln!("hara-fabric-benchmark: {error}");
        std::process::exit(2);
    });
    let json = run(options).unwrap_or_else(|error| {
        eprintln!("hara-fabric-benchmark: {error}");
        std::process::exit(1);
    });
    println!("{json}");
}

fn options() -> Result<Options, String> {
    let mut options = Options {
        rooms: 100,
        tasks_per_room: 100,
        payload_bytes: 4096,
        clients: std::thread::available_parallelism()
            .map(usize::from)
            .unwrap_or(1),
        shards: std::thread::available_parallelism()
            .map(usize::from)
            .unwrap_or(1),
    };
    let mut arguments = env::args().skip(1);
    while let Some(argument) = arguments.next() {
        let mut value = || {
            arguments
                .next()
                .ok_or_else(|| format!("{argument} requires a value"))
        };
        match argument.as_str() {
            "--rooms" => options.rooms = positive(&value()?, "--rooms")?,
            "--tasks" => options.tasks_per_room = positive(&value()?, "--tasks")?,
            "--payload" => options.payload_bytes = positive(&value()?, "--payload")?,
            "--clients" => options.clients = positive(&value()?, "--clients")?,
            "--shards" => options.shards = positive(&value()?, "--shards")?,
            "--help" | "-h" => {
                println!("hara-fabric-benchmark [--rooms N] [--tasks N] [--payload BYTES] [--clients N] [--shards N]");
                std::process::exit(0);
            }
            _ => return Err(format!("unknown option: {argument}")),
        }
    }
    if options.payload_bytes > hara_wasm::service::MAX_REPORT_BYTES {
        return Err(format!(
            "--payload exceeds {} bytes",
            hara_wasm::service::MAX_REPORT_BYTES
        ));
    }
    options.clients = options.clients.min(options.rooms);
    Ok(options)
}

fn positive(value: &str, option: &str) -> Result<usize, String> {
    value
        .parse::<usize>()
        .ok()
        .filter(|value| *value > 0)
        .ok_or_else(|| format!("{option} must be a positive integer"))
}

fn run(options: Options) -> Result<String, String> {
    let service = FabricService::start(ServiceConfig {
        shards: options.shards,
        event_capacity: 1,
        ..ServiceConfig::default()
    })?;
    let subscriptions = prepare(&service, options.rooms)?;
    let payload = Arc::new(vec![42_u8; options.payload_bytes]);
    let barrier = Arc::new(Barrier::new(options.clients + 1));
    let latencies = Arc::new(Mutex::new(Vec::with_capacity(
        options.rooms * options.tasks_per_room,
    )));
    let mut workers = Vec::new();
    for client in 0..options.clients {
        let service = service.clone();
        let payload = payload.clone();
        let barrier = barrier.clone();
        let latencies = latencies.clone();
        let assigned = (client..options.rooms)
            .step_by(options.clients)
            .collect::<Vec<_>>();
        let subscriptions = assigned
            .iter()
            .map(|room| (*room, subscriptions[*room].clone()))
            .collect::<Vec<_>>();
        workers.push(std::thread::spawn(move || -> Result<(), String> {
            barrier.wait();
            for (room, subscriptions) in subscriptions {
                let space = format!("room-{room}");
                for _ in 0..options.tasks_per_room {
                    let started = Instant::now();
                    service.send_report(&space, "coordinator", "researcher-a", "task", &payload)?;
                    service.send_report(&space, "coordinator", "researcher-b", "task", &payload)?;
                    require_report(&service, &subscriptions.researcher_a)?;
                    require_report(&service, &subscriptions.researcher_b)?;
                    service.send_report(&space, "researcher-a", "reviewer", "finding", &payload)?;
                    service.send_report(&space, "researcher-b", "reviewer", "finding", &payload)?;
                    require_report(&service, &subscriptions.reviewer)?;
                    require_report(&service, &subscriptions.reviewer)?;
                    service.send_report(&space, "reviewer", "coordinator", "final", &payload)?;
                    require_report(&service, &subscriptions.coordinator)?;
                    latencies
                        .lock()
                        .map_err(|_| "benchmark latency lock poisoned".to_owned())?
                        .push(started.elapsed().as_nanos() as u64);
                }
            }
            Ok(())
        }));
    }
    barrier.wait();
    let started = Instant::now();
    for worker in workers {
        worker
            .join()
            .map_err(|_| "benchmark worker panicked".to_owned())??;
    }
    let elapsed = started.elapsed();
    let mut latencies = Arc::try_unwrap(latencies)
        .map_err(|_| "benchmark latency owners remain".to_owned())?
        .into_inner()
        .map_err(|_| "benchmark latency lock poisoned".to_owned())?;
    latencies.sort_unstable();
    let tasks = latencies.len();
    let metrics = service.metrics()?;
    let timestamp = SystemTime::now()
        .duration_since(UNIX_EPOCH)
        .unwrap_or_default()
        .as_secs();
    Ok(format!(
        concat!(
            "{{\"schema\":\"hara.fabric.benchmark.v1\",\"timestamp\":{},",
            "\"config\":{{\"rooms\":{},\"tasks_per_room\":{},\"payload_bytes\":{},",
            "\"clients\":{},\"shards\":{}}},",
            "\"result\":{{\"tasks\":{},\"elapsed_ns\":{},\"tasks_per_second\":{:.3},",
            "\"latency_ns\":{{\"p50\":{},\"p95\":{},\"p99\":{},\"p999\":{}}},",
            "\"peak_rss_kib\":{},\"service_metrics\":{}}}}}"
        ),
        timestamp,
        options.rooms,
        options.tasks_per_room,
        options.payload_bytes,
        options.clients,
        options.shards,
        tasks,
        elapsed.as_nanos(),
        tasks as f64 / elapsed.as_secs_f64(),
        percentile(&latencies, 0.50),
        percentile(&latencies, 0.95),
        percentile(&latencies, 0.99),
        percentile(&latencies, 0.999),
        peak_rss_kib().unwrap_or(0),
        metrics.json()
    ))
}

#[derive(Clone)]
struct RoomSubscriptions {
    researcher_a: String,
    researcher_b: String,
    reviewer: String,
    coordinator: String,
}

fn prepare(service: &FabricService, rooms: usize) -> Result<Vec<RoomSubscriptions>, String> {
    let mut result = Vec::with_capacity(rooms);
    for room in 0..rooms {
        let space = format!("room-{room}");
        service.create_space(&space)?;
        for session in ["coordinator", "researcher-a", "researcher-b", "reviewer"] {
            service.create_session(&space, session)?;
        }
        service.retain(&space, "coordinator", "final", 10_000, 86_400_000)?;
        result.push(RoomSubscriptions {
            researcher_a: service.subscribe(&space, "researcher-a", "task")?,
            researcher_b: service.subscribe(&space, "researcher-b", "task")?,
            reviewer: service.subscribe(&space, "reviewer", "finding")?,
            coordinator: service.subscribe(&space, "coordinator", "final")?,
        });
    }
    Ok(result)
}

fn require_report(service: &FabricService, subscription: &str) -> Result<(), String> {
    service
        .next_report(subscription)?
        .ok_or_else(|| format!("benchmark/report-missing: {subscription}"))
        .map(|_| ())
}

fn percentile(values: &[u64], quantile: f64) -> u64 {
    if values.is_empty() {
        return 0;
    }
    let index = ((values.len() - 1) as f64 * quantile).ceil() as usize;
    values[index]
}

fn peak_rss_kib() -> Option<u64> {
    let status = fs::read_to_string("/proc/self/status").ok()?;
    status
        .lines()
        .find_map(|line| line.strip_prefix("VmHWM:"))
        .and_then(|value| value.split_whitespace().next())
        .and_then(|value| value.parse().ok())
}

#[cfg(test)]
mod tests {
    use super::{percentile, Options};

    #[test]
    fn percentile_uses_nearest_rank_without_interpolation() {
        assert_eq!(percentile(&[1, 2, 3, 4, 5], 0.50), 3);
        assert_eq!(percentile(&[1, 2, 3, 4, 5], 0.99), 5);
    }

    #[test]
    fn options_are_copy_for_worker_configuration() {
        let options = Options {
            rooms: 1,
            tasks_per_room: 1,
            payload_bytes: 1,
            clients: 1,
            shards: 1,
        };
        assert_eq!(options.rooms, 1);
    }
}

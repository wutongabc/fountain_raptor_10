//! Test Raptor-10 systematic encode/decode over a K range with a fixed worker pool.
//!
//! Run with:
//! `cargo run -p raptor_10 --release --example test_raptor_10_multi_threads`
//!
//! Optional quick smoke test:
//! `cargo run -p raptor_10 --release --example test_raptor_10_multi_threads -- 4 20 2`

use fountain_engine::*;
use fountain_raptor_10::Raptor10SysCode;
use fountain_utility::VecDataOperater;
use std::collections::HashMap;
use std::env;
use std::io::{self, Write};
use std::panic::{self, AssertUnwindSafe};
use std::process;
use std::sync::{
    Arc,
    atomic::{AtomicUsize, Ordering},
    mpsc,
};
use std::thread;
use std::time::{Duration, Instant};

const DEFAULT_START_K: usize = 4;
const DEFAULT_END_K: usize = 2000;
const DEFAULT_NUMBER_OF_THREADS: usize = 4;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum TestStatus {
    Solvable,
    Unsolvable,
    Panicked,
}

#[derive(Clone, Copy, Debug)]
struct WorkerResult {
    worker_id: usize,
    k: usize,
    status: TestStatus,
    duration: Duration,
}

fn test_single_k(k: usize) -> bool {
    let symbol_size = 4;

    let config = Raptor10SysCode::new_with_default_setting(k);
    let params = config.get_params();
    let k_prime = params.k;

    let mut message_vectors = vec![vec![0u8; symbol_size]; k_prime];
    for (i, vector) in message_vectors.iter_mut().enumerate() {
        for (j, byte) in vector.iter_mut().enumerate() {
            *byte = ((i * j + i) % 256) as u8;
        }
    }

    let mut encode_operator = VecDataOperater::new(symbol_size);
    for (i, vector) in message_vectors.iter().enumerate() {
        encode_operator.insert_vector(vector, i);
    }

    let mut encoder = Encoder::new_with_operator(&config, Box::new(encode_operator));
    let mut coded_id_to_data_id = HashMap::new();

    for coded_id in 0..k_prime {
        if let Some(data_id) = encoder.encode_coded_vector(coded_id) {
            coded_id_to_data_id.insert(coded_id, data_id);
        }
    }

    let total_num = params.num_total();
    let num_repair = k_prime / 2;
    for coded_id in total_num..total_num + num_repair {
        if let Some(data_id) = encoder.encode_coded_vector(coded_id) {
            coded_id_to_data_id.insert(coded_id, data_id);
        }
    }

    let encoder_operator = encoder.manager.move_operator();
    let mut decoder =
        Decoder::new_with_operator(&config, Box::new(VecDataOperater::new(symbol_size)));

    let mut decoded = false;
    for coded_id in 0..k_prime {
        if let Some(data_id) = coded_id_to_data_id.get(&coded_id) {
            let status = decoder.add_coded_vector(coded_id, encoder_operator.get_vector(*data_id));
            if matches!(status, DecodeStatus::Decoded) {
                decoded = true;
                break;
            }
        }
    }

    if !decoded {
        for coded_id in total_num..total_num + num_repair {
            if let Some(data_id) = coded_id_to_data_id.get(&coded_id) {
                let status =
                    decoder.add_coded_vector(coded_id, encoder_operator.get_vector(*data_id));
                if matches!(status, DecodeStatus::Decoded) {
                    decoded = true;
                    break;
                }
            }
        }
    }

    if !decoded {
        return false;
    }

    let decoder_operator = decoder.manager.move_operator();
    for (i, expected) in message_vectors.iter().enumerate() {
        if decoder_operator.get_vector(i) != expected {
            return false;
        }
    }

    true
}

fn main() {
    println!("=== Testing Raptor-10 K Values with Multi-Threaded Workers ===\n");

    let config = RunConfig::from_args();
    let all_k_values = (config.start_k..=config.end_k).collect::<Vec<_>>();
    if all_k_values.is_empty() {
        eprintln!("No k values configured for testing");
        process::exit(1);
    }

    let worker_count = config.num_threads.min(all_k_values.len());
    if worker_count == 0 {
        eprintln!("number of worker threads must be greater than 0");
        process::exit(1);
    }

    println!("K range: {}..={}", config.start_k, config.end_k);
    println!("Total k values to test: {}", all_k_values.len());
    println!("Worker threads: {}", worker_count);
    println!("Testing progress:");
    println!("{}", "-".repeat(90));

    let shared_k_values = Arc::new(all_k_values);
    let next_index = Arc::new(AtomicUsize::new(0));
    let (sender, receiver) = mpsc::channel::<WorkerResult>();
    let mut worker_handles = Vec::with_capacity(worker_count);
    let start_time = Instant::now();

    for worker_id in 0..worker_count {
        let shared_k_values = Arc::clone(&shared_k_values);
        let next_index = Arc::clone(&next_index);
        let sender = sender.clone();

        let handle = thread::Builder::new()
            .name(format!("raptor10-k-worker-{worker_id}"))
            .spawn(move || {
                loop {
                    let index = next_index.fetch_add(1, Ordering::Relaxed);
                    if index >= shared_k_values.len() {
                        break;
                    }

                    let k = shared_k_values[index];
                    let test_started = Instant::now();
                    let status = match panic::catch_unwind(AssertUnwindSafe(|| test_single_k(k))) {
                        Ok(true) => TestStatus::Solvable,
                        Ok(false) => TestStatus::Unsolvable,
                        Err(_) => TestStatus::Panicked,
                    };

                    let result = WorkerResult {
                        worker_id,
                        k,
                        status,
                        duration: test_started.elapsed(),
                    };

                    if sender.send(result).is_err() {
                        break;
                    }
                }
            })
            .unwrap_or_else(|error| panic!("failed to spawn worker thread {worker_id}: {error}"));

        worker_handles.push(handle);
    }
    drop(sender);

    let total = shared_k_values.len();
    let mut completed = 0usize;
    let mut solvable_k = Vec::new();
    let mut unsolvable_k = Vec::new();
    let mut panicked_k = Vec::new();

    while completed < total {
        let result = receiver.recv().unwrap_or_else(|error| {
            panic!("result channel closed after {completed} / {total} results: {error}");
        });

        completed += 1;
        match result.status {
            TestStatus::Solvable => solvable_k.push(result.k),
            TestStatus::Unsolvable => unsolvable_k.push(result.k),
            TestStatus::Panicked => panicked_k.push(result.k),
        }

        print_progress(
            completed,
            total,
            result,
            solvable_k.len(),
            unsolvable_k.len(),
            panicked_k.len(),
            start_time,
        );
    }

    println!();

    let mut worker_panics = 0usize;
    for handle in worker_handles {
        if handle.join().is_err() {
            worker_panics += 1;
        }
    }

    solvable_k.sort_unstable();
    unsolvable_k.sort_unstable();
    panicked_k.sort_unstable();

    println!("{}", "-".repeat(90));
    println!("\n=== Test Results Summary ===");
    println!("K range tested: {}..={}", config.start_k, config.end_k);
    println!("Total k values tested: {}", total);
    println!("Worker threads used: {}", worker_count);
    println!(
        "Solvable: {} ({:.1}%)",
        solvable_k.len(),
        percentage(solvable_k.len(), total)
    );
    println!(
        "Unsolvable: {} ({:.1}%)",
        unsolvable_k.len(),
        percentage(unsolvable_k.len(), total)
    );
    println!(
        "Panicked: {} ({:.1}%)",
        panicked_k.len(),
        percentage(panicked_k.len(), total)
    );

    if worker_panics > 0 {
        println!(
            "Worker thread panics after task isolation: {}",
            worker_panics
        );
    }

    println!("\n=== Unsolvable K Values ({}) ===", unsolvable_k.len());
    if unsolvable_k.is_empty() {
        println!("None");
    } else {
        print_k_list(&unsolvable_k);
    }

    println!("\n=== Panicked K Values ({}) ===", panicked_k.len());
    if panicked_k.is_empty() {
        println!("None");
    } else {
        print_k_list(&panicked_k);
    }

    println!("\n=== Test Complete ===");
}

#[derive(Clone, Copy, Debug)]
struct RunConfig {
    start_k: usize,
    end_k: usize,
    num_threads: usize,
}

impl RunConfig {
    fn from_args() -> Self {
        let args = env::args().skip(1).collect::<Vec<_>>();
        if args.is_empty() {
            return Self {
                start_k: DEFAULT_START_K,
                end_k: DEFAULT_END_K,
                num_threads: DEFAULT_NUMBER_OF_THREADS,
            };
        }

        if args.len() != 3 {
            eprintln!("Usage: test_raptor_10_multi_threads [start_k end_k num_threads]");
            process::exit(2);
        }

        let start_k = parse_arg(&args[0], "start_k");
        let end_k = parse_arg(&args[1], "end_k");
        let num_threads = parse_arg(&args[2], "num_threads");

        if start_k > end_k {
            eprintln!("start_k must be <= end_k");
            process::exit(2);
        }

        Self {
            start_k,
            end_k,
            num_threads,
        }
    }
}

fn parse_arg(value: &str, name: &str) -> usize {
    value.parse::<usize>().unwrap_or_else(|error| {
        eprintln!("invalid {name} '{value}': {error}");
        process::exit(2);
    })
}

fn print_k_list(k_values: &[usize]) {
    for (i, &k) in k_values.iter().enumerate() {
        if i > 0 && i % 10 == 0 {
            println!();
        }
        print!("{:5}", k);
        if (i + 1) % 10 != 0 && i + 1 < k_values.len() {
            print!(", ");
        }
    }
    println!();
}

fn print_progress(
    current: usize,
    total: usize,
    result: WorkerResult,
    solvable: usize,
    unsolvable: usize,
    panicked: usize,
    start_time: Instant,
) {
    const BAR_WIDTH: usize = 40;

    let filled = if total == 0 {
        BAR_WIDTH
    } else {
        current * BAR_WIDTH / total
    };
    let bar = format!(
        "{}{}",
        "=".repeat(filled),
        "-".repeat(BAR_WIDTH.saturating_sub(filled))
    );
    let progress = percentage(current, total);
    let elapsed = start_time.elapsed();
    let eta = if current == 0 {
        Duration::ZERO
    } else {
        Duration::from_secs_f64(elapsed.as_secs_f64() * (total - current) as f64 / current as f64)
    };

    print!(
        "\r[{bar}] {current:>4}/{total:<4} {progress:>5.1}% | worker={:>2} | k={:>5} | {} | ok={solvable:>4} fail={unsolvable:>4} panic={panicked:>4} | last {} | elapsed {} | eta {}",
        result.worker_id + 1,
        result.k,
        status_label(result.status),
        format_duration(result.duration),
        format_duration(elapsed),
        format_duration(eta),
    );
    io::stdout().flush().unwrap();
}

fn status_label(status: TestStatus) -> &'static str {
    match status {
        TestStatus::Solvable => "ok",
        TestStatus::Unsolvable => "fail",
        TestStatus::Panicked => "panic",
    }
}

fn percentage(count: usize, total: usize) -> f64 {
    if total == 0 {
        0.0
    } else {
        100.0 * count as f64 / total as f64
    }
}

fn format_duration(duration: Duration) -> String {
    let secs = duration.as_secs();
    let hours = secs / 3600;
    let minutes = (secs % 3600) / 60;
    let seconds = secs % 60;

    if hours > 0 {
        format!("{hours:02}:{minutes:02}:{seconds:02}")
    } else {
        format!("{minutes:02}:{seconds:02}")
    }
}

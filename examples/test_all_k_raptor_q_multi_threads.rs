//! Test all k values in raptor_q_para.csv with RaptorQSysCodeRFC6330 using
//! a fixed-size worker pool.
//!
//! This example keeps all mutable shared state on the main thread to avoid the
//! usual multi-threading pitfalls: workers only claim the next k index, run an
//! isolated test, and send the result back through a channel.
//!
//! Run with:
//! `cargo run --example test_all_k_raptor_q_multi_threads --release`

use fountain_engine::*;
use fountain_utility::VecDataOperater;
use fountain_raptor_q::{LDPCType, raptor_q_main::raptor_q_main};
use std::io::{self, Write};
use std::panic::{self, AssertUnwindSafe};
use std::process;
use std::sync::{
    atomic::{AtomicUsize, Ordering},
    mpsc, Arc,
};
use std::thread;
use std::time::{Duration, Instant};

const NUMBER_OF_K_TO_TEST: usize = 477;
const NUMBER_OF_THREADS: usize = 4;

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

/// Test a single k value with systematic encoding/decoding.
fn test_single_k(k: usize) -> bool {
    let symbol_size = 4;

    let config = raptor_q_main::new(k, 30, LDPCType::RQLDPC);
    let params = config.get_params();
    let k_prime = params.k;

    let mut message_vectors = vec![vec![0u8; symbol_size]; k_prime];
    for i in 0..k_prime {
        for j in 0..symbol_size {
            message_vectors[i][j] = ((i * j + i) % 256) as u8;
        }
    }

    let mut vec_data_operater = VecDataOperater::new(symbol_size);
    for (i, vector) in message_vectors.iter().enumerate() {
        vec_data_operater.insert_vector(vector, i);
    }

    let encoder_result = panic::catch_unwind(AssertUnwindSafe(|| {
    let mut encoder = config.new_encoder_with_operator(Box::new(vec_data_operater));

        for coded_id in 0..k_prime {
            encoder.encode_coded_vector(coded_id);
        }

        let total_num = params.num_total();
        let num_repair = k_prime / 2;
        for coded_id in total_num..total_num + num_repair {
            encoder.encode_coded_vector(coded_id);
        }

        (encoder, total_num, num_repair)
    }));

    let (encoder, total_num, num_repair) = match encoder_result {
        Ok(result) => result,
        Err(_) => {
            return false;
        }
    };

    let mut decoder = config.new_decoder_with_operator(Box::new(VecDataOperater::new(symbol_size)));

    let mut decoded = false;
    for coded_id in 0..k_prime {
        let coded_vector = encoder.manager.get_coded_vector(coded_id);
        let status = decoder.add_coded_vector(coded_id, &coded_vector);

        if let DecodeStatus::Decoded = status {
            decoded = true;
            break;
        }
    }

    if !decoded {
        for coded_id in total_num..total_num + num_repair {
            let coded_vector = encoder.manager.get_coded_vector(coded_id);
            let status = decoder.add_coded_vector(coded_id, &coded_vector);

            if let DecodeStatus::Decoded = status {
                decoded = true;
                break;
            }
        }
    }

    if decoded {
        for i in 0..k_prime {
            let decoded_vec = decoder.manager.get_data_vector(i);
            if decoded_vec != message_vectors[i] {
                return false;
            }
        }
        return true;
    }

    false
}

fn main() {
    println!("=== Testing All K Values with Multi-Threaded RaptorQ ===\n");

    let all_k_values = read_k_values(NUMBER_OF_K_TO_TEST);
    if all_k_values.is_empty() {
        eprintln!("No k values found in raptor_q_para.csv");
        process::exit(1);
    }

    if NUMBER_OF_THREADS == 0 {
        eprintln!("NUMBER_OF_THREADS must be greater than 0");
        process::exit(1);
    }

    let worker_count = NUMBER_OF_THREADS.min(all_k_values.len());

    println!("Found {} k values to test", all_k_values.len());
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
            .name(format!("k-worker-{worker_id}"))
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
            .unwrap_or_else(|error| {
                panic!("failed to spawn worker thread {worker_id}: {error}");
            });

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
            panic!(
                "result channel closed after {completed} / {total} results: {error}"
            );
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

    println!("\n=== Solvable K Values ({}) ===", solvable_k.len());
    if solvable_k.is_empty() {
        println!("None");
    } else {
        print_k_list(&solvable_k);
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

fn read_k_values(limit: usize) -> Vec<usize> {
    let csv_data = include_str!("../src/raptor_q_para.csv");
    let mut all_k_values = Vec::new();

    for line in csv_data.lines().skip(1) {
        if line.trim().is_empty() {
            continue;
        }

        let parts: Vec<&str> = line.split(',').collect();
        if let Some(k_str) = parts.first() {
            if let Ok(k) = k_str.trim().parse::<usize>() {
                all_k_values.push(k);
            }
        }
    }

    all_k_values.into_iter().take(limit).collect()
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
        "\r[{bar}] {current:>3}/{total:<3} {progress:>5.1}% | worker={:>2} | k={:>5} | {} | ok={solvable:>3} fail={unsolvable:>3} panic={panicked:>3} | last {} | elapsed {} | eta {}",
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

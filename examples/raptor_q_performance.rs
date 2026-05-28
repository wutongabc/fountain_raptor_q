// Copyright (c) 2025.
// All rights reserved.

//! Performance Test for RaptorQ Main Implementation
//!
//! This example runs performance tests on Raptor_q_main and saves the results to a file.

//! run with: cargo run --bin raptor_q_performance --release

use fountain_utility::*;
use fountain_raptor_q::{PARAMS_CSV, raptor_q_main::raptor_q_main};
use std::fs::File;
use std::io::{BufRead, Write};
use std::panic::{self, AssertUnwindSafe};
use std::time::SystemTime;

const TEST_ALL_K: bool = true; // Set to true to test all K values from csv, false for selected K values
const NUMBER_OF_K_TO_TEST: usize = 477; // the number of k values to test, max is 477
const NUM_RUNS: usize = 20;
const NUM_TO_IGNORE: usize = 300; // ignore the first NUM_TO_IGNORE results for each k value

struct ExperimentStats {
    success_rate: f64,
    overhead_stats: Statistics,
    encoding_operation_stats: AverageComputation,
    decoding_operation_stats: AverageComputation,
    avg_encoding_time_ms: f64,
    avg_decoding_time_ms: f64,
}

impl ExperimentStats {
    fn from_results(k: usize, results: &[TestResult]) -> Self {
        let (_, _, success_rate) = TestStatistics::success_rate(results);
        let overhead_stats = TestStatistics::overhead_stats(k, results);
        let (_, encoding_avg, decoding_avg) = TestStatistics::avg_computation_costs(k, results);
        let time_stats = TestStatistics::avg_time_costs(results);

        ExperimentStats {
            success_rate,
            overhead_stats,
            encoding_operation_stats: encoding_avg,
            decoding_operation_stats: decoding_avg,
            avg_encoding_time_ms: time_stats.encoding,
            avg_decoding_time_ms: time_stats.decoding,
        }
    }
}

fn main() -> std::io::Result<()> {
    let timestamp = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_secs();

    // Ensure results directory exists
    std::fs::create_dir_all("results")?;

    // Define test parameters
    let ks = if TEST_ALL_K {
        // Parse K values from PARAMS_CSV
        let mut all_ks = Vec::new();
        for line in PARAMS_CSV.lines().skip(1) {
            // Skip header
            if let Some(k_str) = line.split(',').next() {
                if let Ok(k) = k_str.trim().parse::<usize>() {
                    all_ks.push(k);
                }
            }
        }
        // Take only the first NUMBER_OF_K_TO_TEST values
        all_ks = all_ks.into_iter().take(NUMBER_OF_K_TO_TEST).collect();
        // all_ks = all_ks.into_iter().skip(NUM_TO_IGNORE).collect();
        all_ks
    } else {
        vec![10, 30, 60, 100, 200, 500] // Different K values to test
    };

    // Header for console and file
    let header = format!(
        "{:<10} {:<10} {:<15} {:<15} {:<15} {:<15} {:<15} {:<15}",
        "K",
        "Coded",
        "Success Rate",
        "Overhead",
        "Enc Time(ms)",
        "Dec Time(ms)",
        "Enc Ops",
        "Dec Ops"
    );

    println!("=== RaptorQ Performance Test ===");
    println!("Testing All K: {}", TEST_ALL_K);
    println!("\n{}", header);
    println!("{}", "-".repeat(110));

    let mut all_results: Vec<TestResult> = Vec::new();
    let mut failed_ks: Vec<usize> = Vec::new();

    for &k in &ks {
        let num_coded_vectors = k + k / 2; // 1.5x overhead buffer

        // Catch panics to skip failed K values
        let result = panic::catch_unwind(AssertUnwindSafe(|| {
            let code = raptor_q_main::new_with_default_setting(k);
            let results = test_code_scheme_multiple(NUM_RUNS, &code, k, num_coded_vectors);
            results
        }));

        match result {
            Ok(results) => {
                let stats = ExperimentStats::from_results(k, &results);
                all_results.extend(results);

                let line = format!(
                    "{:<10} {:<10} {:<15.2} {:<15.4} {:<15.4} {:<15.4} {:<15.2} {:<15.2}",
                    k,
                    num_coded_vectors,
                    stats.success_rate * 100.0,
                    stats.overhead_stats.mean,
                    stats.avg_encoding_time_ms,
                    stats.avg_decoding_time_ms,
                    stats.encoding_operation_stats.vector_add,
                    stats.decoding_operation_stats.vector_add
                );

                println!("{}", line);
            }
            Err(_) => {
                println!(
                    "{:<10} {:<10} {:<15} {:<15} {:<15} {:<15} {:<15} {:<15}",
                    k, "N/A", "FAILED", "N/A", "N/A", "N/A", "N/A", "N/A"
                );
                failed_ks.push(k);
            }
        }
    }

    // Save detailed results to JSON
    let json_output_path = format!("results/raptor_q_performance_{}.jsonl", timestamp);
    save_test_results(&all_results, &json_output_path).expect("Failed to save JSON results");
    println!("Detailed JSON results saved to: {}", json_output_path);

    if !failed_ks.is_empty() {
        println!("\nFailed K values: {:?}", failed_ks);
        let failed_path = format!("results/raptor_q_failed_ks_{}.txt", timestamp);
        let mut failed_file = File::create(&failed_path)?;
        writeln!(failed_file, "Failed K values: {:?}", failed_ks)?;
        println!("Failed K values saved to: {}", failed_path);
    }

    println!("\nTest complete.");
    Ok(())
}

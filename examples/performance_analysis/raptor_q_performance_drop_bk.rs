// Copyright (c) 2025.
// All rights reserved.

//! Performance Test for RaptorQ Main Implementation with Packet Dropping (No Data)
//! 
//! This example runs performance tests on Raptor_q_main WITHOUT actual data vectors (symbolic simulation),
//! specifically testing decoding performance when the first N packets are dropped.
//! 
//! Rust Concept Explanation:
//! - `//!`: Module-level documentation.
//! - `//`: Implementation comments.

// Run with: $env:RUSTFLAGS="-A warnings"; cargo run --bin raptor_q_performance_drop_bk --release

#![allow(warnings)]

use fountain_utility::*;
use fountain_raptor_q::{raptor_q_main::raptor_q_main, PARAMS_CSV};
use std::time::{SystemTime, Instant};
use std::fs::File;
use std::io::Write;
use std::panic::{self, AssertUnwindSafe};
use rand::prelude::SliceRandom;
use std::collections::HashMap;
use fountain_engine::*;
use serde::{Serialize, Deserialize};

// Configuration constants
const TEST_ALL_K: bool = true; // Set to true to test all K values from csv
const NUMBER_OF_K_TO_TEST: usize = 3; // Max number of K values to test
const NUM_RUNS: usize = 10; // Number of repetitions for statistical significance
const K_TO_SKIP: usize = 433; // Skip the first K_TO_SKIP packets
const MAX_PACKETS_TO_SKIP: usize = 0; // Test skipping 0 to this number of packets
const PACKETS_TO_SKIP_STEP: usize = 0; // Step size for skipping packets

// Extended TestResult that includes dropped_packets info
#[derive(Debug, Clone, Serialize, Deserialize)]
struct DropTestResult {
    #[serde(flatten)]
    base: TestResult,
    packets_dropped: usize,
}

// Rust Concept: Struct definition
// Holds aggregated statistics for an experiment
struct ExperimentStats {
    success_rate: f64,
    overhead_stats: Statistics,
    encoding_operation_stats: AverageComputation,
    decoding_operation_stats: AverageComputation,
    avg_encoding_time_ms: f64,
    avg_decoding_time_ms: f64,
}

// Rust Concept: Implementation block (`impl`)
impl ExperimentStats {
    fn from_results(k: usize, results: &[DropTestResult]) -> Self {
        // Convert back to basic TestResult slice for utility functions
        let base_results: Vec<TestResult> = results.iter().map(|r| r.base.clone()).collect();
        
        let (_, _, success_rate) = TestStatistics::success_rate(&base_results);
        let overhead_stats = TestStatistics::overhead_stats(k, &base_results);
        let (_, encoding_avg, decoding_avg) = TestStatistics::avg_computation_costs(k, &base_results);
        let time_stats = TestStatistics::avg_time_costs(&base_results);

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

// Custom testing function that supports skipping packets WITHOUT actual data
fn test_code_scheme_with_packet_skip_no_data(
    code_scheme: &raptor_q_main, 
    k: usize, 
    num_coded_vectors: usize,
    packets_to_skip: usize
) -> DropTestResult {    
    // Encoding - Determine which symbols to generate
    let params = code_scheme.get_params();
    let mut coded_ids : Vec<usize> = if code_scheme.code_type() == CodeType::Systematic {
        let mut ids: Vec<usize> = (0..k).collect();
        ids.extend(params.num_total()..params.num_total() + num_coded_vectors-k);
        ids
    } else {
        (params.num_total()..params.num_total() + num_coded_vectors).collect()
    };

    // Precoding (Symbolic)
    let precoding_time = Instant::now();
    let mut encoder = code_scheme.new_encoder();
    let precoding_time = precoding_time.elapsed();
    let precoding_metrics = PerformanceMetrics
        ::from_operations(&encoder.manager.move_new_operations(), params.k);

    // LT Encoding (Symbolic)
    let encoding_time = Instant::now();
    for coded_id in &coded_ids {
        encoder.encode_coded_vector(*coded_id);
    }
    let encoding_time = encoding_time.elapsed();
    let encoding_metrics = PerformanceMetrics
        ::from_operations(&encoder.manager.move_new_operations(), params.num_total());
    
    // Do not shuffle packets to test sequential packet loss
    // let mut rng = rand::thread_rng();
    // coded_ids.shuffle(&mut rng);
    
    // Decoding - SKIP the first `packets_to_skip` packets
    let effective_coded_ids: Vec<&usize> = coded_ids.iter().skip(packets_to_skip).collect();

    let mut decoded_successfully = false;
    let decoding_time = Instant::now();
    let mut decoder = code_scheme.new_decoder();
    
    // Feed packets to decoder (Symbolic - only IDs)
    for coded_id in effective_coded_ids {
        let status = decoder.add_coded_id(*coded_id);
        if matches!(status, DecodeStatus::Decoded) {
            decoded_successfully = true;
            break;
        }
    }
    let decoding_time = decoding_time.elapsed();
    let decoding_metrics = PerformanceMetrics
        ::from_operations(&decoder.manager.move_new_operations(), decoder.manager.coded_vector_inserted);

    // In symbolic testing, if decoding status is Decoded, we assume 0 mismatches.
    let num_mismatches = if decoded_successfully {
        0
    } else {
        k
    };
    
    let base_result = TestResult {
        k,
        num_mismatches,
        precoding_metrics,
        encoding_metrics,
        decoding_metrics,
        precoding_time_ms: precoding_time.as_secs_f64() * 1000.0,
        encoding_time_ms: encoding_time.as_secs_f64() * 1000.0,
        decoding_time_ms: decoding_time.as_secs_f64() * 1000.0,
    };

    DropTestResult {
        base: base_result,
        packets_dropped: packets_to_skip,
    }
}

// Custom save function for DropTestResult
pub fn save_drop_test_results(results: &[DropTestResult], filename: &str) -> Result<(), Box<dyn std::error::Error>> {
    let file = File::create(filename)?;
    let mut writer = std::io::BufWriter::new(file);
    for result in results {
        let json = serde_json::to_string(result)?;
        writeln!(writer, "{}", json)?;
    }
    writer.flush()?;
    Ok(())
}

fn main() -> std::io::Result<()> {
    let timestamp = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    
    std::fs::create_dir_all("results/drop")?;
    
    let ks = if TEST_ALL_K {
        let mut all_ks = Vec::new();
        for line in PARAMS_CSV.lines().skip(1) { 
            if let Some(k_str) = line.split(',').next() {
                if let Ok(k) = k_str.trim().parse::<usize>() {
                    all_ks.push(k);
                }
            }
        }
        all_ks.into_iter().skip(K_TO_SKIP).take(NUMBER_OF_K_TO_TEST).collect()
    
    } else {
        vec![10, 30, 60, 100, 200, 500] 
    };
    
    let header = format!(
        "{:<10} {:<10} {:<15} {:<15} {:<15} {:<15} {:<15} {:<15}",
        "K", "Skip", "Success Rate", "Overhead", "Enc Time(ms)", "Dec Time(ms)", "Enc Ops", "Dec Ops"
    );
    
    println!("=== RaptorQ Performance Test (Packet Drop Analysis - Symbolic) ===");
    println!("Testing All K: {}", TEST_ALL_K);
    println!("Max Packets to Skip: {}", MAX_PACKETS_TO_SKIP);
    println!("All K values: {:?}", ks);
    println!("\n{}", header);
    println!("{}", "-".repeat(110));
    
    let mut all_results: Vec<DropTestResult> = Vec::new();
    let mut failed_ks: Vec<usize> = Vec::new();
    
    // Save path
    let json_output_path = format!("results/drop/raptor_q_performance_drop_{}.jsonl", timestamp);

    for &k in &ks {
        for skip in (0..=MAX_PACKETS_TO_SKIP).step_by(PACKETS_TO_SKIP_STEP) {
            let num_coded_vectors = k + k/2 + skip; 
            
            let result = panic::catch_unwind(AssertUnwindSafe(|| {
                let code = raptor_q_main::new_with_default_setting(k);
                
                let results: Vec<DropTestResult> = (0..NUM_RUNS)
                    .map(|_| {
                        test_code_scheme_with_packet_skip_no_data(
                            &code, 
                            k, 
                            num_coded_vectors,
                            skip
                        )
                    })
                    .collect();
                    
                results
            }));
    
            match result {
                Ok(results) => {
                    let stats = ExperimentStats::from_results(k, &results);
                    all_results.extend(results);
                    
                    let line = format!(
                        "{:<10} {:<10} {:<15.2} {:<15.4} {:<15.4} {:<15.4} {:<15.2} {:<15.2}",
                        k, 
                        skip,
                        stats.success_rate * 100.0,
                        stats.overhead_stats.mean,
                        stats.avg_encoding_time_ms,
                        stats.avg_decoding_time_ms,
                        stats.encoding_operation_stats.vector_add,
                        stats.decoding_operation_stats.vector_add
                    );
                    
                    println!("{}", line);
                },
                Err(_) => {
                    println!("{:<10} {:<10} {:<15} {:<15} {:<15} {:<15} {:<15} {:<15}",
                        k, skip, "FAILED", "N/A", "N/A", "N/A", "N/A", "N/A");
                    if skip == 0 { failed_ks.push(k); } 
                }
            }
        }
        
        // Save results after each K iteration (Checkpoint)
        // Rust Concept: Result Handling
        // We use `.expect()` here because if saving fails, we probably want to know immediately,
        // but since this is a checkpoint, maybe logging the error is better? 
        // For simplicity in a script, panic on save failure is acceptable.
        save_drop_test_results(&all_results, &json_output_path).expect("Failed to save checkpoint results");
        println!("Checkpoint saved for K={} to: {}", k, json_output_path);
    }
    
    println!("Detailed JSON results saved to: {}", json_output_path);

    if !failed_ks.is_empty() {
        println!("\nFailed K values: {:?}", failed_ks);
        let failed_path = format!("results/drop/raptor_q_drop_failed_ks_{}.txt", timestamp);
        let mut failed_file = File::create(&failed_path)?;
        writeln!(failed_file, "Failed K values: {:?}", failed_ks)?;
        println!("Failed K values saved to: {}", failed_path);
    }

    println!("\nTest complete.");
    Ok(())
}

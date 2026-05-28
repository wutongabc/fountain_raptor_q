// Copyright (c) 2025.
// All rights reserved.

//! Performance Test for RaptorQ Main Implementation with Data Vectors
//! 
//! This example runs performance tests on Raptor_q_main with actual data vectors and saves the results to a file.
//! 
//! Rust Concept Explanation:
//! - `//!`: These are module-level documentation comments. They describe the file/module itself.
//! - `//`: Single line comments for implementation details.

//! run with: cargo run --bin raptor_q_performance_data --release

use fountain_utility::*;
use fountain_raptor_q::{raptor_q_main::raptor_q_main, PARAMS_CSV};
use std::time::SystemTime;
use std::fs::File;
use std::io::Write;
use std::panic::{self, AssertUnwindSafe};

// Configuration constants
const TEST_ALL_K: bool = true; // Set to true to test all K values from csv
const NUMBER_OF_K_TO_TEST: usize = 100; // Max number of K values to test
const NUM_RUNS: usize = 20; // Number of repetitions for statistical significance
const DATA_VECTOR_LENGTH: usize = 1024; // Size of each data symbol in bytes

// Rust Concept: Struct definition
// Structs are custom data types that group related values.
// Here we define a struct to hold aggregated statistics for an experiment.
struct ExperimentStats {
    success_rate: f64,
    overhead_stats: Statistics,
    encoding_operation_stats: AverageComputation,
    decoding_operation_stats: AverageComputation,
    avg_encoding_time_ms: f64,
    avg_decoding_time_ms: f64,
}

// Rust Concept: Implementation block (`impl`)
// This defines methods associated with the `ExperimentStats` struct.
// It's similar to a class method in C++ or Python.
impl ExperimentStats {
    // A "static method" (associated function) that creates a new instance from results.
    // `k`: input symbol count
    // `results`: a slice (view) of TestResult objects
    fn from_results(k: usize, results: &[TestResult]) -> Self {
        let (_, _, success_rate) = TestStatistics::success_rate(results);
        let overhead_stats = TestStatistics::overhead_stats(k, results);
        let (_, encoding_avg, decoding_avg) = TestStatistics::avg_computation_costs(k, results);
        let time_stats = TestStatistics::avg_time_costs(results);

        // Rust Concept: Struct instantiation
        // Creating a new instance of the struct. Field names matching variable names is a shorthand.
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
    // Rust Concept: Result<()>
    // `main` can return a Result to handle I/O errors gracefully. 
    // `?` operator will propagate errors up.

    let timestamp = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_secs();
    
    // Create directory if it doesn't exist
    // `std::fs::create_dir_all` is similar to `mkdir -p`
    std::fs::create_dir_all("results")?;
    
    // Define K values to test
    // Rust Concept: `if` expression
    // In Rust, `if` is an expression that returns a value. 
    // Here we assign the result of the if/else block to `ks`.
    let ks = if TEST_ALL_K {
        // Parse K values from PARAMS_CSV string
        let mut all_ks = Vec::new();
        // Rust Concept: Iterator chaining
        // `lines()` creates an iterator over lines
        // `skip(1)` ignores the header
        for line in PARAMS_CSV.lines().skip(1) { 
            // `split` returns an iterator over substrings
            if let Some(k_str) = line.split(',').next() {
                // `parse::<usize>()` attempts to convert string to number
                if let Ok(k) = k_str.trim().parse::<usize>() {
                    all_ks.push(k);
                }
            }
        }
        // Take only the specified number of K values
        all_ks.into_iter().take(NUMBER_OF_K_TO_TEST).collect()
    } else {
        // `vec!` macro creates a Vec (dynamic array)
        vec![10, 30, 60, 100, 200, 500] 
    };
    
    // Format header string
    let header = format!(
        "{:<10} {:<10} {:<15} {:<15} {:<15} {:<15} {:<15} {:<15}",
        "K", "Coded", "Success Rate", "Overhead", "Enc Time(ms)", "Dec Time(ms)", "Enc Ops", "Dec Ops"
    );
    
    println!("=== RaptorQ Performance Test (With Data Vectors) ===");
    println!("Testing All K: {}", TEST_ALL_K);
    println!("Data Vector Length: {} bytes", DATA_VECTOR_LENGTH);
    println!("\n{}", header);
    println!("{}", "-".repeat(110));
    
    let mut all_results: Vec<TestResult> = Vec::new();
    let mut failed_ks: Vec<usize> = Vec::new();

    // Iterate over reference to K values
    for &k in &ks {
        let num_coded_vectors = k + k/2; // 1.5x overhead buffer
        
        // Rust Concept: Panic catching
        // `panic::catch_unwind` allows catching runtime panics (crashes) to prevent the whole program from stopping.
        // `AssertUnwindSafe` is a wrapper asserting that the closure is safe to unwind.
        // Rust Concept: Closures
        // `|| { ... }` defines an anonymous function (closure) that captures context if needed.
        let result = panic::catch_unwind(AssertUnwindSafe(|| {
            let code = raptor_q_main::new_with_default_setting(k);
            
            // Rust Concept: Map and Collect
            // We run the test `NUM_RUNS` times using an iterator range `(0..NUM_RUNS)`.
            // `map` transforms each index into a test result.
            // `collect` gathers all results into a Vec.
            let results: Vec<TestResult> = (0..NUM_RUNS)
                .map(|_| {
                    test_code_scheme_with_data_vectors(
                        &code, 
                        k, 
                        DATA_VECTOR_LENGTH, 
                        num_coded_vectors
                    )
                })
                .collect();
                
            results
        }));

        // Rust Concept: Pattern Matching with `match`
        // Similar to switch-case but more powerful. We handle both Ok (success) and Err (panic) cases.
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
            },
            Err(_) => {
                println!("{:<10} {:<10} {:<15} {:<15} {:<15} {:<15} {:<15} {:<15}",
                    k, "N/A", "FAILED", "N/A", "N/A", "N/A", "N/A", "N/A");
                failed_ks.push(k);
            }
        }
    }
    
    // Save detailed results
    let json_output_path = format!("results/raptor_q_performance_data_{}.jsonl", timestamp);
    // `expect` unwraps the Result, panicking with a custom message if it's an Error.
    save_test_results(&all_results, &json_output_path).expect("Failed to save JSON results");
    println!("Detailed JSON results saved to: {}", json_output_path);

    if !failed_ks.is_empty() {
        println!("\nFailed K values: {:?}", failed_ks);
        let failed_path = format!("results/raptor_q_data_failed_ks_{}.txt", timestamp);
        let mut failed_file = File::create(&failed_path)?;
        writeln!(failed_file, "Failed K values: {:?}", failed_ks)?;
        println!("Failed K values saved to: {}", failed_path);
    }

    println!("\nTest complete.");
    Ok(())
}


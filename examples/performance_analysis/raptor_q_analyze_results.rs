//! Analysis script for processing test results
//! 
//! This script reads JSONL files from the results directory,
//! calculates statistics using fountain_utility, and saves the analysis.

//! Usage:
//! ```bash
//! cargo run --bin raptor_q_analyze_results --release
//! ```

use std::fs;
use std::path::Path;
use std::io::Write;
use std::collections::BTreeMap;
use fountain_utility::{
    load_test_results,
    TestStatistics,
    TestResult,
};
use serde::{Deserialize, Serialize};
use std::io::{BufReader, BufRead};

// Define DropTestResult structure locally to match what's in the file
#[derive(Debug, Clone, Serialize, Deserialize)]
struct DropTestResult {
    #[serde(flatten)]
    base: TestResult,
    packets_dropped: usize,
}

// Define WindowTestResult structure locally
#[derive(Debug, Clone, Serialize, Deserialize)]
struct WindowTestResult {
    #[serde(flatten)]
    base: TestResult,
    window_size: usize,
    window_start: usize,
}

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let results_dir = Path::new("examples/results");
    let results_drop_dir = Path::new("examples/results/drop");

    let results_dir = if results_dir.exists() { results_dir } else { Path::new("results") };
    let results_drop_dir = if results_drop_dir.exists() { results_drop_dir } else { Path::new("results/drop") };

    if !results_dir.exists() {
        eprintln!("Could not find results directory at {:?} or examples/results", results_dir);
        return Ok(());
    }

    println!("Scanning directory: {:?}", results_dir.canonicalize()?);

    // Process regular result files
    for entry in fs::read_dir(results_dir)? {
        let entry = entry?;
        let path = entry.path();
        
        if path.extension().and_then(|s| s.to_str()) == Some("jsonl") {
            println!("Processing file: {:?}", path.file_name().unwrap());
            process_file(&path)?;
        }
    }

    if results_drop_dir.exists() {
        println!("Scanning drop directory: {:?}", results_drop_dir.canonicalize()?);
        for entry in fs::read_dir(results_drop_dir)? {
            let entry = entry?;
            let path = entry.path();
            let file_name = path.file_name().and_then(|s| s.to_str()).unwrap_or("");
            
            if path.extension().and_then(|s| s.to_str()) == Some("jsonl") {
                if file_name.contains("window") {
                    println!("Processing window drop file: {:?}", file_name);
                    process_window_drop_file(&path)?;
                } else {
                    println!("Processing drop file: {:?}", file_name);
                    process_drop_file(&path)?;
                }
            }
        }
    }

    Ok(())
}

fn process_file(path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let path_str = path.to_str().unwrap();
    // Try to load as standard TestResult
    // Note: load_test_results returns Vec<TestResult>. 
    // If the file contains extra fields (like packets_dropped), serde will ignore them due to default behavior or we might need strict control.
    // By default serde ignores unknown fields.
    let results = load_test_results(path_str)?;
    
    if results.is_empty() {
        println!("  No results found in file.");
        return Ok(());
    }

    let mut results_by_k: BTreeMap<usize, Vec<TestResult>> = BTreeMap::new();
    for result in results {
        results_by_k.entry(result.k).or_default().push(result);
    }

    let file_stem = path.file_stem().and_then(|s| s.to_str()).unwrap();
    let output_filename = format!("{}_analysis.csv", file_stem);
    let output_path = path.parent().unwrap().join(output_filename);

    let mut csv_content = String::new();
    csv_content.push_str("K,Runs,SuccessRate,OverheadMean,OverheadMin,OverheadMax,StorageMean,StorageMin,StorageMax,TimePrecoding,TimeEncoding,TimeDecoding,CompPrecMulAlpha,CompPrecMulScalar,CompPrecVecAdd,CompPrecMulAdd,CompEncMulAlpha,CompEncMulScalar,CompEncVecAdd,CompEncMulAdd,CompDecMulAlpha,CompDecMulScalar,CompDecVecAdd,CompDecMulAdd\n");

    for (k, k_results) in results_by_k {
        let (num_runs, _, success_rate) = TestStatistics::success_rate(&k_results);
        let overhead_stats = TestStatistics::overhead_stats(k, &k_results);
        let storage_stats = TestStatistics::storage_stats(&k_results);
        let (prec_comp, enc_comp, dec_comp) = TestStatistics::avg_computation_costs(k, &k_results);
        let time_stats = TestStatistics::avg_time_costs(&k_results);

        let line = format!(
            "{},{},{:.6},{:.6},{:.6},{:.6},{:.6},{:.6},{:.6},{:.6},{:.6},{:.6},{:.6},{:.6},{:.6},{:.6},{:.6},{:.6},{:.6},{:.6},{:.6},{:.6},{:.6},{:.6}\n",
            k, num_runs, success_rate,
            overhead_stats.mean, overhead_stats.min, overhead_stats.max,
            storage_stats.mean, storage_stats.min, storage_stats.max,
            time_stats.precoding, time_stats.encoding, time_stats.decoding,
            prec_comp.multiply_alpha, prec_comp.multiply_scalar, prec_comp.vector_add, prec_comp.mul_add,
            enc_comp.multiply_alpha, enc_comp.multiply_scalar, enc_comp.vector_add, enc_comp.mul_add,
            dec_comp.multiply_alpha, dec_comp.multiply_scalar, dec_comp.vector_add, dec_comp.mul_add
        );
        csv_content.push_str(&line);
    }

    let mut file = fs::File::create(&output_path)?;
    file.write_all(csv_content.as_bytes())?;
    println!("Analysis saved to: {:?}", output_path);
    Ok(())
}

fn process_drop_file(path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let file = fs::File::open(path)?;
    let reader = BufReader::new(file);
    let mut results = Vec::new();
    
    // Manually load DropTestResult items
    for line in reader.lines() {
        let line = line?;
        if !line.trim().is_empty() {
            // Try to parse, if fails might be because of different schema, but we expect DropTestResult here
            // Note: WindowTestResult also has packets_dropped? No, it has window_size/start.
            // If we try to parse a WindowTestResult as DropTestResult, it will fail if strict, or ignore fields.
            // But main() separates them by filename.
            if let Ok(result) = serde_json::from_str::<DropTestResult>(&line) {
                results.push(result);
            }
        }
    }

    if results.is_empty() {
        println!("  No valid drop results found in file.");
        return Ok(());
    }

    // Group by (k, packets_dropped)
    let mut results_grouped: BTreeMap<(usize, usize), Vec<TestResult>> = BTreeMap::new();
    for result in results {
        results_grouped
            .entry((result.base.k, result.packets_dropped))
            .or_default()
            .push(result.base);
    }

    let file_stem = path.file_stem().and_then(|s| s.to_str()).unwrap();
    let output_filename = format!("{}_analysis.csv", file_stem);
    let output_path = path.parent().unwrap().join(output_filename);

    let mut csv_content = String::new();
    // Header includes PacketsDropped
    csv_content.push_str("K,PacketsDropped,Runs,SuccessRate,OverheadMean,OverheadMin,OverheadMax,StorageMean,StorageMin,StorageMax,TimePrecoding,TimeEncoding,TimeDecoding,CompPrecMulAlpha,CompPrecMulScalar,CompPrecVecAdd,CompPrecMulAdd,CompEncMulAlpha,CompEncMulScalar,CompEncVecAdd,CompEncMulAdd,CompDecMulAlpha,CompDecMulScalar,CompDecVecAdd,CompDecMulAdd\n");

    for ((k, dropped), k_results) in results_grouped {
        let (num_runs, _, success_rate) = TestStatistics::success_rate(&k_results);
        let overhead_stats = TestStatistics::overhead_stats(k, &k_results);
        let storage_stats = TestStatistics::storage_stats(&k_results);
        let (prec_comp, enc_comp, dec_comp) = TestStatistics::avg_computation_costs(k, &k_results);
        let time_stats = TestStatistics::avg_time_costs(&k_results);

        let line = format!(
            "{},{},{},{:.6},{:.6},{:.6},{:.6},{:.6},{:.6},{:.6},{:.6},{:.6},{:.6},{:.6},{:.6},{:.6},{:.6},{:.6},{:.6},{:.6},{:.6},{:.6},{:.6},{:.6},{:.6}\n",
            k, dropped, // New field
            num_runs, success_rate,
            overhead_stats.mean, overhead_stats.min, overhead_stats.max,
            storage_stats.mean, storage_stats.min, storage_stats.max,
            time_stats.precoding, time_stats.encoding, time_stats.decoding,
            prec_comp.multiply_alpha, prec_comp.multiply_scalar, prec_comp.vector_add, prec_comp.mul_add,
            enc_comp.multiply_alpha, enc_comp.multiply_scalar, enc_comp.vector_add, enc_comp.mul_add,
            dec_comp.multiply_alpha, dec_comp.multiply_scalar, dec_comp.vector_add, dec_comp.mul_add
        );
        csv_content.push_str(&line);
    }

    let mut file = fs::File::create(&output_path)?;
    file.write_all(csv_content.as_bytes())?;
    println!("Analysis saved to: {:?}", output_path);
    Ok(())
}

fn process_window_drop_file(path: &Path) -> Result<(), Box<dyn std::error::Error>> {
    let file = fs::File::open(path)?;
    let reader = BufReader::new(file);
    let mut results = Vec::new();
    
    // Manually load WindowTestResult items
    for line in reader.lines() {
        let line = line?;
        if !line.trim().is_empty() {
            if let Ok(result) = serde_json::from_str::<WindowTestResult>(&line) {
                results.push(result);
            }
        }
    }

    if results.is_empty() {
        println!("  No valid window drop results found in file.");
        return Ok(());
    }

    // Group by (k, window_start, window_size)
    let mut results_grouped: BTreeMap<(usize, usize, usize), Vec<TestResult>> = BTreeMap::new();
    for result in results {
        results_grouped
            .entry((result.base.k, result.window_start, result.window_size))
            .or_default()
            .push(result.base);
    }

    let file_stem = path.file_stem().and_then(|s| s.to_str()).unwrap();
    let output_filename = format!("{}_analysis.csv", file_stem);
    let output_path = path.parent().unwrap().join(output_filename);

    let mut csv_content = String::new();
    // Header includes WindowStart, WindowSize
    csv_content.push_str("K,WindowStart,WindowSize,Runs,SuccessRate,OverheadMean,OverheadMin,OverheadMax,StorageMean,StorageMin,StorageMax,TimePrecoding,TimeEncoding,TimeDecoding,CompPrecMulAlpha,CompPrecMulScalar,CompPrecVecAdd,CompPrecMulAdd,CompEncMulAlpha,CompEncMulScalar,CompEncVecAdd,CompEncMulAdd,CompDecMulAlpha,CompDecMulScalar,CompDecVecAdd,CompDecMulAdd\n");

    for ((k, win_start, win_size), k_results) in results_grouped {
        let (num_runs, _, success_rate) = TestStatistics::success_rate(&k_results);
        let overhead_stats = TestStatistics::overhead_stats(k, &k_results);
        let storage_stats = TestStatistics::storage_stats(&k_results);
        let (prec_comp, enc_comp, dec_comp) = TestStatistics::avg_computation_costs(k, &k_results);
        let time_stats = TestStatistics::avg_time_costs(&k_results);

        let line = format!(
            "{},{},{},{},{:.6},{:.6},{:.6},{:.6},{:.6},{:.6},{:.6},{:.6},{:.6},{:.6},{:.6},{:.6},{:.6},{:.6},{:.6},{:.6},{:.6},{:.6},{:.6},{:.6},{:.6},{:.6}\n",
            k, win_start, win_size, // New fields
            num_runs, success_rate,
            overhead_stats.mean, overhead_stats.min, overhead_stats.max,
            storage_stats.mean, storage_stats.min, storage_stats.max,
            time_stats.precoding, time_stats.encoding, time_stats.decoding,
            prec_comp.multiply_alpha, prec_comp.multiply_scalar, prec_comp.vector_add, prec_comp.mul_add,
            enc_comp.multiply_alpha, enc_comp.multiply_scalar, enc_comp.vector_add, enc_comp.mul_add,
            dec_comp.multiply_alpha, dec_comp.multiply_scalar, dec_comp.vector_add, dec_comp.mul_add
        );
        csv_content.push_str(&line);
    }

    let mut file = fs::File::create(&output_path)?;
    file.write_all(csv_content.as_bytes())?;
    println!("Analysis saved to: {:?}", output_path);
    Ok(())
}

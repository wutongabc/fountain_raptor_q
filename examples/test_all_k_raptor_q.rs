//! Test all k values in raptor_q_para.csv with RaptorQSysCodeRFC6330
//!
//! This script tests whether systematic encoding/decoding works for all k values
//! in the RaptorQ parameter table when using RFC 6330 degree set.
//!
//! Run with: cargo run --example test_all_k_raptor_q --release

use fountain_engine::*;
use fountain_utility::VecDataOperater;
use fountain_raptor_q::{LDPCType, raptor_q_main::raptor_q_main};
use std::io::{self, Write};
use std::time::{Duration, Instant};

/// Test a single k value with systematic encoding/decoding
fn test_single_k(k: usize) -> bool {
    let symbol_size = 4;

    // Create code configuration
    let config = raptor_q_main::new(k, 30, LDPCType::RQLDPC);
    let params = config.get_params();
    let k_prime = params.k;

    // Create message vectors
    let mut message_vectors = vec![vec![0u8; symbol_size]; k_prime];
    for i in 0..k_prime {
        for j in 0..symbol_size {
            message_vectors[i][j] = ((i * j + i) % 256) as u8;
        }
    }

    // Setup encoder
    let mut vec_data_operater = VecDataOperater::new(symbol_size);
    for (i, vector) in message_vectors.iter().enumerate() {
        vec_data_operater.insert_vector(vector, i);
    }

    // Try to create encoder - this is where systematic encoding might fail
    let encoder_result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        let mut encoder = config.new_encoder_with_operator(Box::new(vec_data_operater));

        // Encode systematic symbols (0..k_prime)
        for coded_id in 0..k_prime {
            encoder.encode_coded_vector(coded_id);
        }

        // Encode additional repair symbols
        let total_num = params.num_total();
        let num_repair = k_prime / 2; // Generate k'/2 repair symbols
        for coded_id in total_num..total_num + num_repair {
            encoder.encode_coded_vector(coded_id);
        }

        (encoder, total_num, num_repair)
    }));

    // Check if encoding succeeded
    let (encoder, total_num, num_repair) = match encoder_result {
        Ok(result) => result,
        Err(_) => {
            return false; // Encoding failed (likely due to non-triangular structure)
        }
    };

    // Now test decoding
        let mut decoder = config.new_decoder_with_operator(Box::new(VecDataOperater::new(symbol_size)));

    // Use k_prime symbols for decoding (should be sufficient)
    let mut decoded = false;
    for coded_id in 0..k_prime {
        let coded_vector = encoder.manager.get_coded_vector(coded_id);
        let status = decoder.add_coded_vector(coded_id, &coded_vector);

        if let DecodeStatus::Decoded = status {
            decoded = true;
            break;
        }
    }

    // If still not decoded, try adding repair symbols
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

    // Verify decoded vectors match original
    if decoded {
        for i in 0..k_prime {
            let decoded_vec = decoder.manager.get_data_vector(i);
            if decoded_vec != message_vectors[i] {
                return false; // Decoded data doesn't match
            }
        }
        return true; // Successfully decoded and verified
    }

    false // Failed to decode
}

fn main() {
    // define the number of k to test
    // const NUMBER_OF_K_TO_TEST: usize = 477;
    const NUMBER_OF_K_TO_TEST: usize = 477;

    // test single k of 36479
    // let is_solvable = test_single_k(36479);
    // println!("Test single k of 36479: {}", is_solvable);
    // return;

    println!("=== Testing All K Values with RaptorQSysCodeRFC6330 ===\n");

    // Read all k values from CSV
    let csv_data = include_str!("../src/raptor_q_para.csv");
    let mut all_k_values = Vec::new();

    for line in csv_data.lines().skip(1) {
        // Skip header
        if line.trim().is_empty() {
            continue;
        }
        let parts: Vec<&str> = line.split(',').collect();
        if let Some(k_str) = parts.get(0) {
            if let Ok(k) = k_str.trim().parse::<usize>() {
                all_k_values.push(k);
            }
        }
    }

    let mut solvable_k = Vec::new();
    let mut unsolvable_k = Vec::new();

    // only test the k = 18
    // let all_k_values = vec![36];
    // only test the first 20 k values
    let all_k_values: Vec<usize> = all_k_values
        .iter()
        .take(NUMBER_OF_K_TO_TEST)
        .cloned()
        .collect();

    println!("Found {} k values to test", all_k_values.len());
    println!("Testing progress:");
    println!("{}", "-".repeat(70));

    let start_time = Instant::now();

    // Test each k value
    for (idx, &k) in all_k_values.iter().enumerate() {
        let is_solvable = test_single_k(k);

        if is_solvable {
            // println!("鉁?SOLVABLE");
            solvable_k.push(k);
        } else {
            // println!("鉂?UNSOLVABLE");
            unsolvable_k.push(k);
        }

        print_progress(
            idx + 1,
            all_k_values.len(),
            k,
            solvable_k.len(),
            unsolvable_k.len(),
            start_time,
        );
    }

    println!();
    println!("{}", "-".repeat(70));
    println!("\n=== Test Results Summary ===");
    println!("Total k values tested: {}", all_k_values.len());
    println!(
        "Solvable: {} ({:.1}%)",
        solvable_k.len(),
        100.0 * solvable_k.len() as f64 / all_k_values.len() as f64
    );
    println!(
        "Unsolvable: {} ({:.1}%)",
        unsolvable_k.len(),
        100.0 * unsolvable_k.len() as f64 / all_k_values.len() as f64
    );

    println!("\n=== Solvable K Values ({}) ===", solvable_k.len());
    if !solvable_k.is_empty() {
        print_k_list(&solvable_k);
    } else {
        println!("None");
    }

    println!("\n=== Unsolvable K Values ({}) ===", unsolvable_k.len());
    if !unsolvable_k.is_empty() {
        print_k_list(&unsolvable_k);
    } else {
        println!("None");
    }

    println!("\n=== Test Complete ===");
}

/// Print k values in a formatted list (10 per line)
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
    current_k: usize,
    solvable: usize,
    unsolvable: usize,
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
    let progress = if total == 0 {
        100.0
    } else {
        current as f64 / total as f64 * 100.0
    };

    let elapsed = start_time.elapsed();
    let eta = if current == 0 {
        Duration::ZERO
    } else {
        Duration::from_secs_f64(elapsed.as_secs_f64() * (total - current) as f64 / current as f64)
    };

    print!(
        "\r[{bar}] {current:>3}/{total:<3} {progress:>5.1}% | k={current_k:>5} | ok={solvable:>3} fail={unsolvable:>3} | elapsed {} | eta {}",
        format_duration(elapsed),
        format_duration(eta),
    );
    io::stdout().flush().unwrap();
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

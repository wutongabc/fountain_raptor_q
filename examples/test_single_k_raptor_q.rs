//! Simple test for a single k value with raptor_q_main
//!
//! This test will panic if encoding or decoding fails.
//! Change `k` below to select the case, then run:
//! `cargo run --release --locked --example test_single_k_raptor_q`

use fountain_engine::*;
use fountain_utility::VecDataOperater;
use fountain_raptor_q::raptor_q_main::raptor_q_main;

fn main() {
    // Change this value to test different k values
    // After switching to DefaultHDPC, all k values should work!
    let k = 39980;
    let symbol_size = 8;

    println!("=== Testing Single K Value with RaptorQ Main ===");
    println!("k = {}", k);
    println!("symbol_size = {} bytes\n", symbol_size);

    // Create code configuration
    println!("[1/6] Creating code configuration...");
    let config = raptor_q_main::new_with_default_setting(k);
    let params = config.get_params();
    let k_prime = params.k;
    println!("  k' (padded) = {}", k_prime);
    println!("  a (active) = {}", params.a);
    println!("  l (LDPC) = {}", params.l);
    println!("  h (HDPC) = {}", params.h);

    // Create message vectors
    println!("\n[2/6] Creating message vectors...");
    let mut message_vectors = vec![vec![0u8; symbol_size]; k_prime];
    for i in 0..k_prime {
        for j in 0..symbol_size {
            message_vectors[i][j] = ((i * j + i + j) % 256) as u8;
        }
    }
    println!(
        "  Created {} message vectors of {} bytes each",
        k_prime, symbol_size
    );

    // Setup encoder
    println!("\n[3/6] Setting up encoder...");
    let mut vec_data_operater = VecDataOperater::new(symbol_size);
    for (i, vector) in message_vectors.iter().enumerate() {
        vec_data_operater.insert_vector(vector, i);
    }

    // Create encoder - this is where systematic encoding might fail
    println!("\n[4/6] Creating encoder and encoding systematic symbols...");
    let mut encoder = config.new_encoder_with_operator(Box::new(vec_data_operater));

    // Encode systematic symbols (0..k_prime)
    for coded_id in 0..k_prime {
        encoder.encode_coded_vector(coded_id);
    }
    println!("  Successfully encoded {} systematic symbols", k_prime);

    // Encode additional repair symbols
    let total_num = params.num_total();
    let num_repair = (k_prime / 2).max(10); // Generate k'/2 repair symbols, at least 10
    println!("\n[5/6] Encoding {} repair symbols...", num_repair);
    for coded_id in total_num..total_num + num_repair {
        encoder.encode_coded_vector(coded_id);
    }
    println!("  Successfully encoded {} repair symbols", num_repair);

    // Now test decoding
    println!("\n[6/6] Testing decoding...");
    let mut decoder = config.new_decoder_with_operator(Box::new(VecDataOperater::new(symbol_size)));

    // Use k_prime + some overhead for decoding
    let mut decoded = false;
    let mut symbols_used = 0;

    // First try with systematic symbols
    for coded_id in 0..k_prime {
        let coded_vector = encoder.manager.get_coded_vector(coded_id);
        let status = decoder.add_coded_vector(coded_id, &coded_vector);
        symbols_used += 1;

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
            symbols_used += 1;

            if let DecodeStatus::Decoded = status {
                decoded = true;
                break;
            }
        }
    }

    // Check if decoded
    if !decoded {
        panic!(
            "❌ DECODING FAILED! Used {} symbols but could not decode.",
            symbols_used
        );
    }

    println!(
        "  ✅ Decoding successful! Used {} symbols (overhead: {})",
        symbols_used,
        symbols_used as i32 - k_prime as i32
    );

    // Verify decoded vectors match original
    println!("\n[Verification] Checking decoded data integrity...");
    let mut mismatches = 0;
    for i in 0..k_prime {
        let decoded_vec = decoder.manager.get_data_vector(i);
        if decoded_vec != message_vectors[i] {
            mismatches += 1;
            if mismatches <= 5 {
                println!("  ❌ Mismatch at vector {}", i);
            }
        }
    }

    if mismatches > 0 {
        panic!(
            "❌ DATA INTEGRITY CHECK FAILED! {} vectors don't match (out of {})",
            mismatches, k_prime
        );
    }

    println!("  ✅ All {} vectors verified successfully!", k_prime);

    println!("\n=== TEST PASSED ===");
    println!("k = {} is fully functional with raptor_q_main", k);
}

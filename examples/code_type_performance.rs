//! Automatic end-to-end performance comparison for systematic and ordinary RaptorQ.
//!
//! This is separate from `raptor_q_performance.rs`; it does not change that
//! example's constants or output. Both modes use the same deterministic source
//! bytes, symbol size, packet budget, and K values. Every sample constructs a
//! fresh scheme, so setup and ordinary cold-cache precoding are included.
//!
//! Run the default K values (`100, 1000, 5008`):
//! `cargo run --release --locked --example code_type_performance`
//!
//! Run selected K values:
//! `cargo run --release --locked --example code_type_performance -- 100 1000`
//!
//! Systematic mode sends source-symbol ESIs first; ordinary mode sends only LT
//! symbols beginning at `params.num_total()`. Consequently, this benchmark
//! compares real mode behavior rather than only a shared internal operation.

use fountain_engine::{CodeScheme, CodeType, DataOperator, DecodeStatus};
use fountain_raptor_q::{RaptorQDecoder, RaptorQEncoder, raptor_q_main::raptor_q_main};
use fountain_utility::VecDataOperater;
use std::time::{Duration, Instant};

const DEFAULT_K_VALUES: &[usize] = &[100, 1000, 5008];
const SYMBOL_SIZE: usize = 128;
const RUNS: usize = 5;
const PACKET_BUDGET_MULTIPLIER: usize = 2;

#[derive(Default)]
struct ModeSamples {
    encode_us: Vec<f64>,
    decode_us: Vec<f64>,
    packets_used: Vec<usize>,
}

fn source_vectors(k: usize) -> Vec<Vec<u8>> {
    (0..k)
        .map(|source_id| {
            (0..SYMBOL_SIZE)
                .map(|byte_id| ((source_id * 17 + byte_id * 11) % 251) as u8)
                .collect()
        })
        .collect()
}

fn packet_ids(
    code_type: CodeType,
    source_k: usize,
    block_k: usize,
    first_lt_esi: usize,
) -> Vec<usize> {
    let packet_budget = block_k * PACKET_BUDGET_MULTIPLIER;
    match code_type {
        CodeType::Systematic => {
            let mut ids: Vec<_> = (0..source_k).collect();
            ids.extend(first_lt_esi..first_lt_esi + packet_budget.saturating_sub(source_k));
            ids
        }
        CodeType::Ordinary => (first_lt_esi..first_lt_esi + packet_budget).collect(),
    }
}

fn run_once(source_k: usize, code_type: CodeType) -> Result<(Duration, Duration, usize), String> {
    let messages = source_vectors(source_k);
    let mut encoder_operator = VecDataOperater::new(SYMBOL_SIZE);
    for (source_id, message) in messages.iter().enumerate() {
        encoder_operator.insert_vector(message, source_id);
    }

    let code = raptor_q_main::new_with_default_setting(source_k).with_code_type(code_type);
    let params = code.get_params();
    let ids = packet_ids(code_type, source_k, params.k, params.num_total());

    let encode_started = Instant::now();
    let mut encoder =
        RaptorQEncoder::new_with_operator(code.clone(), Box::new(encoder_operator), SYMBOL_SIZE);
    let mut packets = Vec::with_capacity(ids.len());
    for esi in ids {
        let data_id = encoder
            .encode_coded_vector(esi)
            .ok_or_else(|| format!("{code_type:?} rejected ESI {esi} for K={source_k}"))?;
        packets.push((esi, encoder.get_data_vector(data_id).to_vec()));
    }
    let encode_elapsed = encode_started.elapsed();

    let decode_started = Instant::now();
    let mut decoder = RaptorQDecoder::new_with_operator(
        code,
        Box::new(VecDataOperater::new(SYMBOL_SIZE)),
        SYMBOL_SIZE,
    );
    let mut packets_used = 0;
    for (esi, payload) in packets {
        packets_used += 1;
        if decoder.add_coded_vector(esi, &payload) == DecodeStatus::Decoded {
            break;
        }
    }
    let decode_elapsed = decode_started.elapsed();

    if decoder.decode_status() != DecodeStatus::Decoded {
        return Err(format!(
            "{code_type:?} did not decode K={source_k} within the packet budget"
        ));
    }
    for (source_id, expected) in messages.iter().enumerate() {
        if decoder.get_data_vector(source_id) != expected.as_slice() {
            return Err(format!(
                "{code_type:?} changed K={source_k}, source symbol {source_id}"
            ));
        }
    }

    Ok((encode_elapsed, decode_elapsed, packets_used))
}

fn record(samples: &mut ModeSamples, result: (Duration, Duration, usize)) {
    samples.encode_us.push(result.0.as_secs_f64() * 1_000_000.0);
    samples.decode_us.push(result.1.as_secs_f64() * 1_000_000.0);
    samples.packets_used.push(result.2);
}

fn median_f64(samples: &[f64]) -> f64 {
    let mut sorted = samples.to_vec();
    sorted.sort_by(f64::total_cmp);
    let middle = sorted.len() / 2;
    if sorted.len() % 2 == 0 {
        (sorted[middle - 1] + sorted[middle]) / 2.0
    } else {
        sorted[middle]
    }
}

fn median_usize(samples: &[usize]) -> usize {
    let mut sorted = samples.to_vec();
    sorted.sort_unstable();
    sorted[sorted.len() / 2]
}

fn requested_k_values() -> Result<Vec<usize>, String> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    if args.iter().any(|arg| arg == "-h" || arg == "--help") {
        println!("Usage: cargo run --release --locked --example code_type_performance -- [K ...]");
        println!("No K arguments uses: {DEFAULT_K_VALUES:?}");
        return Ok(Vec::new());
    }
    if args.is_empty() {
        return Ok(DEFAULT_K_VALUES.to_vec());
    }
    args.into_iter()
        .map(|arg| {
            arg.parse::<usize>()
                .map_err(|_| format!("invalid K value: {arg}"))
        })
        .collect()
}

fn main() -> Result<(), String> {
    let k_values = requested_k_values()?;
    if k_values.is_empty() {
        return Ok(());
    }

    println!("=== RaptorQ systematic vs ordinary end-to-end benchmark ===");
    println!(
        "Median of {RUNS} fresh-scheme runs; symbol size {SYMBOL_SIZE} bytes; packet budget {PACKET_BUDGET_MULTIPLIER} * K'."
    );
    println!(
        "{:<7} {:>12} {:>12} {:>12} {:>12} {:>11} {:>11}",
        "K", "Sys enc us", "Ord enc us", "Sys dec us", "Ord dec us", "Enc ratio", "Dec ratio"
    );

    for source_k in k_values {
        let mut systematic = ModeSamples::default();
        let mut ordinary = ModeSamples::default();
        for run in 0..RUNS {
            // Alternate order to reduce a consistent first/second-run timing bias.
            if run % 2 == 0 {
                record(&mut systematic, run_once(source_k, CodeType::Systematic)?);
                record(&mut ordinary, run_once(source_k, CodeType::Ordinary)?);
            } else {
                record(&mut ordinary, run_once(source_k, CodeType::Ordinary)?);
                record(&mut systematic, run_once(source_k, CodeType::Systematic)?);
            }
        }

        let sys_enc = median_f64(&systematic.encode_us);
        let ord_enc = median_f64(&ordinary.encode_us);
        let sys_dec = median_f64(&systematic.decode_us);
        let ord_dec = median_f64(&ordinary.decode_us);
        println!(
            "{:<7} {:>12.2} {:>12.2} {:>12.2} {:>12.2} {:>10.2}x {:>10.2}x",
            source_k,
            sys_enc,
            ord_enc,
            sys_dec,
            ord_dec,
            ord_enc / sys_enc,
            ord_dec / sys_dec,
        );
        println!(
            "        median packets consumed: systematic {}, ordinary {}",
            median_usize(&systematic.packets_used),
            median_usize(&ordinary.packets_used),
        );
    }

    println!("Ratios are ordinary/systematic; values above 1.0 mean ordinary took longer.");
    Ok(())
}

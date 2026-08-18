//! End-to-end cache benchmark for the real ordinary RaptorQ encoder path.
//! （单独检测 cache 的效果，但通过完整 ordinary Encoder 路径。）
//!
//! Unlike a direct `HDPC::lu_idssh` microbenchmark, every measurement uses
//! [`RaptorQEncoder::new_with_operator`]. That constructor enters ordinary
//! precoding, including LDPC preparation and the HDPC solve where the LU cache
//! is consumed. It then generates LT packets, decodes them with
//! [`RaptorQDecoder`], and checks every recovered source symbol.
//!
//! The three modes are:
//! - `uncached`: [`HDPCType::CodeSchemeRQHDPC`], which recomputes the LU step;
//! - `cold`: the default cached [`HDPCType::RQHDPC`] on a fresh scheme;
//! - `warm`: repeated encoders created from clones sharing a pre-populated cache.
//!
//! Run the default K values (`100, 1000, 5008`):
//! `cargo run --release --locked --example hdpc_cache_performance`
//!
//! Run selected K values:
//! `cargo run --release --locked --example hdpc_cache_performance -- 100 1000`

use fountain_engine::{CodeScheme, CodeType, DataOperator, DecodeStatus};
use fountain_raptor_q::{HDPCType, RaptorQDecoder, RaptorQEncoder, raptor_q_main::raptor_q_main};
use fountain_utility::VecDataOperater;
use std::time::Instant;

const DEFAULT_K_VALUES: &[usize] = &[100, 1000, 5008];
const SYMBOL_SIZE: usize = 128;
const RUNS: usize = 5;
const PACKET_BUDGET_MULTIPLIER: usize = 2;

#[derive(Default)]
struct Samples {
    precoding_ms: Vec<f64>,
    total_encoding_ms: Vec<f64>,
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

fn ordinary_scheme(k: usize, hdpc_type: HDPCType) -> raptor_q_main {
    raptor_q_main::new_with_default_setting(k)
        .with_code_type(CodeType::Ordinary)
        .with_hdpc_type(hdpc_type)
}

fn run_encoder_round_trip(
    scheme: raptor_q_main,
    messages: &[Vec<u8>],
) -> Result<(f64, f64), String> {
    let mut encoder_operator = VecDataOperater::new(SYMBOL_SIZE);
    for (source_id, message) in messages.iter().enumerate() {
        encoder_operator.insert_vector(message, source_id);
    }

    let params = scheme.get_params();
    let first_ordinary_esi = params.num_total();
    let packet_budget = params.k * PACKET_BUDGET_MULTIPLIER;

    // This is the production-facing constructor. Its ordinary branch performs
    // the complete precode, including the cached or uncached HDPC solve.
    let precoding_started = Instant::now();
    let mut encoder =
        RaptorQEncoder::new_with_operator(scheme.clone(), Box::new(encoder_operator), SYMBOL_SIZE);
    let precoding_ms = precoding_started.elapsed().as_secs_f64() * 1_000.0;

    let encoding_started = Instant::now();
    let mut packets = Vec::with_capacity(packet_budget);
    for esi in first_ordinary_esi..first_ordinary_esi + packet_budget {
        let data_id = encoder
            .encode_coded_vector(esi)
            .ok_or_else(|| format!("ordinary encoder rejected ESI {esi}"))?;
        packets.push((esi, encoder.get_data_vector(data_id).to_vec()));
    }
    let lt_ms = encoding_started.elapsed().as_secs_f64() * 1_000.0;

    let mut decoder = RaptorQDecoder::new_with_operator(
        scheme,
        Box::new(VecDataOperater::new(SYMBOL_SIZE)),
        SYMBOL_SIZE,
    );
    for (esi, payload) in packets {
        if decoder.add_coded_vector(esi, &payload) == DecodeStatus::Decoded {
            break;
        }
    }
    if decoder.decode_status() != DecodeStatus::Decoded {
        return Err("ordinary decoder did not reach Decoded status".to_string());
    }
    for (source_id, expected) in messages.iter().enumerate() {
        if decoder.get_data_vector(source_id) != expected.as_slice() {
            return Err(format!("decoded source symbol {source_id} does not match"));
        }
    }

    Ok((precoding_ms, precoding_ms + lt_ms))
}

fn record(samples: &mut Samples, result: (f64, f64)) {
    samples.precoding_ms.push(result.0);
    samples.total_encoding_ms.push(result.1);
}

fn median(samples: &[f64]) -> f64 {
    let mut sorted = samples.to_vec();
    sorted.sort_by(f64::total_cmp);
    let middle = sorted.len() / 2;
    if sorted.len() % 2 == 0 {
        (sorted[middle - 1] + sorted[middle]) / 2.0
    } else {
        sorted[middle]
    }
}

fn requested_k_values() -> Result<Vec<usize>, String> {
    let args: Vec<_> = std::env::args().skip(1).collect();
    if args.iter().any(|arg| arg == "-h" || arg == "--help") {
        println!("Usage: cargo run --release --locked --example hdpc_cache_performance -- [K ...]");
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

    println!("=== RaptorQ ordinary encoder cache benchmark ===");
    println!(
        "Median of {RUNS} verified round trips; symbol size {SYMBOL_SIZE} bytes; packet budget {PACKET_BUDGET_MULTIPLIER} * K'."
    );
    println!(
        "{:<7} {:>13} {:>13} {:>13} {:>11} {:>15} {:>15} {:>11}",
        "K",
        "No pre ms",
        "Cold pre ms",
        "Warm pre ms",
        "Pre speedup",
        "No total enc ms",
        "Warm total ms",
        "Total speedup"
    );

    for source_k in k_values {
        let messages = source_vectors(source_k);
        let warm_scheme = ordinary_scheme(source_k, HDPCType::RQHDPC);

        // Populate the cache through a real ordinary encoder before measuring
        // subsequent encoders that share it through scheme clones.
        run_encoder_round_trip(warm_scheme.clone(), &messages)?;

        let mut uncached = Samples::default();
        let mut cold = Samples::default();
        let mut warm = Samples::default();
        for run in 0..RUNS {
            // Rotate measurement order to reduce a fixed first-run bias.
            let measure_uncached = || {
                run_encoder_round_trip(
                    ordinary_scheme(source_k, HDPCType::CodeSchemeRQHDPC),
                    &messages,
                )
            };
            let measure_cold =
                || run_encoder_round_trip(ordinary_scheme(source_k, HDPCType::RQHDPC), &messages);
            let measure_warm = || run_encoder_round_trip(warm_scheme.clone(), &messages);

            match run % 3 {
                0 => {
                    record(&mut uncached, measure_uncached()?);
                    record(&mut cold, measure_cold()?);
                    record(&mut warm, measure_warm()?);
                }
                1 => {
                    record(&mut cold, measure_cold()?);
                    record(&mut warm, measure_warm()?);
                    record(&mut uncached, measure_uncached()?);
                }
                _ => {
                    record(&mut warm, measure_warm()?);
                    record(&mut uncached, measure_uncached()?);
                    record(&mut cold, measure_cold()?);
                }
            }
        }

        let no_pre = median(&uncached.precoding_ms);
        let cold_pre = median(&cold.precoding_ms);
        let warm_pre = median(&warm.precoding_ms);
        let no_total = median(&uncached.total_encoding_ms);
        let warm_total = median(&warm.total_encoding_ms);
        println!(
            "{:<7} {:>13.3} {:>13.3} {:>13.3} {:>10.2}x {:>15.3} {:>15.3} {:>10.2}x",
            source_k,
            no_pre,
            cold_pre,
            warm_pre,
            no_pre / warm_pre,
            no_total,
            warm_total,
            no_total / warm_total,
        );
    }

    println!("Every sample completed ordinary encode, decode, and source-data verification.");
    Ok(())
}

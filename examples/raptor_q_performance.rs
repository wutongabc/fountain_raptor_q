//! Performance evaluation for [`raptor_q_main`] (RFC 6330).
//!
//! For each K, runs one [`test_code_scheme_with_data_vectors`] pass (with
//! [`VecDataOperater`](fountain_utility::VecDataOperater)) to verify encode/decode
//! correctness, then [`test_code_scheme_multiple`] for operation-count and timing stats.
//!
//! Run the original full benchmark with the file-wide [`CODE_TYPE`] setting:
//!
//! ```text
//! cargo run --release --locked --example raptor_q_performance
//! ```
//!
//! Change [`CODE_TYPE`] to run that benchmark in systematic or ordinary mode.
//! For an automatic side-by-side comparison, use the separate
//! `code_type_performance` example documented in the README.
//!
//! ## Default K values
//!
//! RFC 6330 table **K′** with **K = K′** (no padding): `10, 101, 200, 511, 1002, 2005, 5008`.
//! K=5008 is the K′ nearest to 5000 (primary large-K reference for [raptor-q-performance.md](../../docs/plans/raptor-q-performance.md)).
//!
//! ## Regression gate (GF(256) correctness)
//!
//! After shared `fountain_engine` edits (e.g. from [ge-phase-performance.md](../../docs/plans/ge-phase-performance.md)
//! packed GF(2) work), run from the workspace root:
//!
//! ```text
//! cargo run -p fountain_raptor_q --example raptor_q_performance --release
//! ```
//!
//! **Acceptance:** every listed K shows **100% Success%**; no `VERIFY_FAIL`, `VERIFY_PANIC`, or `FAILED`.
//! Padded [`RaptorQEncoder`](fountain_raptor_q::RaptorQEncoder) / [`RaptorQDecoder`](fountain_raptor_q::RaptorQDecoder) is
//! exercised before the generic harness when `num_padding() > 0`.
//!
//! ## GE sub-phase profiling
//!
//! For inactive LU / HDPC / back-substitution timings (not measured here), use
//! [`scheme_profile`](scheme_profile.rs) with `--release --features profiling`:
//!
//! ```text
//! cargo run -p fountain_raptor_q --example scheme_profile --release --features profiling -- -s 128 --operator slab 5008
//! ```
//!
//! Baselines and optimization plan: [docs/plans/raptor-q-performance.md](../../docs/plans/raptor-q-performance.md).
//!
//! ## Real-symbol benchmark (G3.4)
//!
//! After the abstract harness, reports padded-codec encode/decode wall times at symbol sizes
//! 128 and 1500 @ K = 5008 for **on-the-fly** and **deferred** operator execution with
//! [`VecDataOperater`](fountain_utility::VecDataOperater)
//! ([`fountain_utility::real_symbol_benchmark`](fountain_utility::real_symbol_benchmark)):
//!
//! ```text
//! cargo run -p fountain_raptor_q --example raptor_q_performance --release
//! ```
//!
//! Deferred mode splits **engine** time (solver + op recording) from **operator** time
//! (batch replay), per [doc-engine.org](../../docs/doc-engine.org) § Delayed Data Operation.
//! The table includes footnotes on column meaning (on-the-fly `enc` is LT-only);
//! see [deferred-benchmark-optimization.md](../../docs/plans/deferred-benchmark-optimization.md) §2.1.
//!
//! For GE sub-phases with real bytes, use [`decode_profile`](decode_profile.rs):
//!
//! ```text
//! cargo run -p fountain_raptor_q --example decode_profile --release --features profiling -- -s 128 5008
//! ```

use fountain_engine::CodeScheme;
use fountain_engine::traits::DataOperator;
use fountain_engine::types::{CodeType, DecodeStatus};
use fountain_raptor_q::{
    PARAMS_CSV, RFC6330_GF256_PRIMITIVE_POLYNOMIAL, RaptorQDecoder, RaptorQEncoder,
    RaptorQRealSymbolSession, raptor_q_main::raptor_q_main,
};
use fountain_utility::{
    OperatorFactory, RealSymbolBenchConfig, TestResult, TestStatistics, VecDataOperater,
    benchmark_deferred, benchmark_on_the_fly, make_test_messages,
    print_real_symbol_benchmark_table, save_test_results, test_code_scheme_multiple,
    test_code_scheme_with_data_vectors,
};
use std::fs::File;
use std::io::Write;
use std::panic::{self, AssertUnwindSafe};
use std::time::SystemTime;

const TEST_ALL_K: bool = false;
const NUMBER_OF_K_TO_TEST: usize = 477;
const NUM_RUNS: usize = 50;
const SYMBOL_SIZE: usize = 4;
const REAL_SYMBOL_K: usize = 5008;
const REAL_SYMBOL_SIZES: &[usize] = &[128, 1500];
const REAL_SYMBOL_RUNS: usize = 5;
const OVERHEAD_NUMERATOR: usize = 3;
const OVERHEAD_DENOMINATOR: usize = 2;
/// 改这一个值就能切换全文件的编码模式。
const CODE_TYPE: CodeType = CodeType::Systematic; // CodeType::Ordinary; // CodeType::Systematic

struct ExperimentStats {
    success_rate: f64,
    overhead_stats: fountain_utility::Statistics,
    precoding_operation_stats: fountain_utility::AverageComputation,
    encoding_operation_stats: fountain_utility::AverageComputation,
    decoding_operation_stats: fountain_utility::AverageComputation,
    avg_precoding_time_us: f64,
    avg_encoding_time_us: f64,
    avg_decoding_time_us: f64,
}

impl ExperimentStats {
    fn from_results(k: usize, results: &[TestResult]) -> Self {
        let (_, _, success_rate) = TestStatistics::success_rate(results);
        let overhead_stats = TestStatistics::overhead_stats(k, results);
        let (prec_avg, encoding_avg, decoding_avg) =
            TestStatistics::avg_computation_costs(k, results);
        let time_stats = TestStatistics::avg_time_costs(results);

        Self {
            success_rate,
            overhead_stats,
            precoding_operation_stats: prec_avg,
            encoding_operation_stats: encoding_avg,
            decoding_operation_stats: decoding_avg,
            avg_precoding_time_us: time_stats.precoding * 1000.0,
            avg_encoding_time_us: time_stats.encoding * 1000.0,
            avg_decoding_time_us: time_stats.decoding * 1000.0,
        }
    }
}

fn k_values_to_test() -> Vec<usize> {
    if TEST_ALL_K {
        PARAMS_CSV
            .lines()
            .skip(1)
            .filter(|line| !line.trim().is_empty())
            .filter_map(|line| line.split(',').next()?.trim().parse().ok())
            .take(NUMBER_OF_K_TO_TEST)
            .collect()
    } else {
        // RFC parameter-table K′ values (source k = K′ avoids padding in the generic harness).
        vec![10, 101, 200, 511, 1002, 2005, 5008]
    }
}

fn num_coded_vectors(k: usize) -> usize {
    k * OVERHEAD_NUMERATOR / OVERHEAD_DENOMINATOR
}

fn vec_operator_factory(symbol_size: usize) -> Box<dyn fountain_engine::DataOperator> {
    Box::new(VecDataOperater::new(symbol_size))
}

/// One on-the-fly roundtrip with message bytes via padded [`RaptorQEncoder`]/[`RaptorQDecoder`].
fn verify_with_data_operator(k: usize, num_coded: usize) -> Result<(), String> {
    let code = raptor_q_main::new_with_default_setting(k).with_code_type(CODE_TYPE);
    let source_k = code.source_symbols();

    let messages = make_test_messages(source_k, SYMBOL_SIZE);

    let mut enc_op = VecDataOperater::new(SYMBOL_SIZE);
    for (i, v) in messages.iter().enumerate() {
        enc_op.insert_vector(v, i);
    }

    let mut encoder =
        RaptorQEncoder::new_with_operator(code.clone(), Box::new(enc_op), SYMBOL_SIZE);
    let params = code.get_params();
    // let mut coded_ids: Vec<usize> = (0..source_k).collect();
    // coded_ids.extend(params.num_total()..params.num_total() + num_coded.saturating_sub(source_k));
    let coded_ids: Vec<usize> = match CODE_TYPE {
        CodeType::Systematic => {
            let mut ids: Vec<usize> = (0..source_k).collect();
            ids.extend(
                params.num_total()..params.num_total() + num_coded.saturating_sub(source_k),
            );
            ids
        }
        CodeType::Ordinary => {
            (params.num_total()..params.num_total() + num_coded).collect()
        }
    };
    
    let mut coded_payload = Vec::new();
    for &coded_id in &coded_ids {
        if let Some(data_id) = encoder.encode_coded_vector(coded_id) {
            coded_payload.push((coded_id, encoder.get_data_vector(data_id).to_vec()));
        }
    }

    let mut decoder = RaptorQDecoder::new_with_operator(
        code,
        Box::new(VecDataOperater::new(SYMBOL_SIZE)),
        SYMBOL_SIZE,
    );

    let mut decoded = false;
    for (coded_id, payload) in coded_payload {
        if decoder.add_coded_vector(coded_id, &payload) == DecodeStatus::Decoded {
            decoded = true;
            break;
        }
    }
    if !decoded {
        return Err("decode did not reach Decoded status".into());
    }

    let dec_op = decoder.inner.manager.move_operator();
    let mismatches = (0..source_k)
        .filter(|&i| dec_op.get_vector(i) != messages[i])
        .count();
    if mismatches == 0 {
        Ok(())
    } else {
        Err(format!(
            "data operator verify: {mismatches} message mismatch(es)"
        ))
    }
}

fn print_real_symbol_benchmarks() {
    let num_coded = num_coded_vectors(REAL_SYMBOL_K);
    let code = raptor_q_main::new_with_default_setting(REAL_SYMBOL_K).with_code_type(CODE_TYPE);
    let source_k = code.source_symbols();
    let params = code.get_params();     // Ordinary更改
    let session = RaptorQRealSymbolSession;

    let bench_config = |symbol_size: usize| {
        match CODE_TYPE {
            CodeType::Systematic => {
                RealSymbolBenchConfig::new(&code, source_k, symbol_size, num_coded)
                    .with_field_pp(RFC6330_GF256_PRIMITIVE_POLYNOMIAL)
            }
            CodeType::Ordinary => {
                RealSymbolBenchConfig {
                    source_k,
                    symbol_size,
                    // Ordinary情况：coded_ids全部 ESI 从 num_total() 开始取，避开 encoder 拒绝范围
                    coded_ids: (params.num_total()..params.num_total() + num_coded).collect(),
                    field_pp: RFC6330_GF256_PRIMITIVE_POLYNOMIAL,
                }
            }
        }
    };    

    print_real_symbol_benchmark_table(
        &format!("Real-symbol benchmark (G3.4 / G3.3, K={REAL_SYMBOL_K}, padded codec)"),
        REAL_SYMBOL_RUNS,
        REAL_SYMBOL_SIZES,
        &[("VecDataOperater", vec_operator_factory as OperatorFactory)],
        &|symbol_size, factory| {
            let config = bench_config(symbol_size);
            let messages = make_test_messages(source_k, symbol_size);
            benchmark_on_the_fly(&code, &session, &config, &messages, factory)
        },
        &|symbol_size, factory| {
            let config = bench_config(symbol_size);
            let messages = make_test_messages(source_k, symbol_size);
            benchmark_deferred(&code, &session, &config, &messages, factory)
        },
    );

    println!(
        "(GE sub-phases: scheme_profile --release --features profiling -- -s <size> --operator slab {REAL_SYMBOL_K})"
    );
}

/// Generic harness (no padding symbols). Use when `K = K′`.
fn verify_with_generic_harness(k: usize, num_coded: usize) -> Result<(), String> {
    let code = raptor_q_main::new_with_default_setting(k).with_code_type(CODE_TYPE);
    if code.num_padding() > 0 {
        return verify_with_data_operator(k, num_coded);
    }
    let result = test_code_scheme_with_data_vectors(&code, k, SYMBOL_SIZE, num_coded);
    if result.num_mismatches == 0 {
        Ok(())
    } else {
        Err(format!(
            "data operator verify: {} message mismatch(es)",
            result.num_mismatches
        ))
    }
}

fn main() -> std::io::Result<()> {
    let timestamp = SystemTime::now()
        .duration_since(SystemTime::UNIX_EPOCH)
        .unwrap()
        .as_secs();

    std::fs::create_dir_all("results")?;

    let ks = k_values_to_test();

    let header = format!(
        "{:<8} {:<8} {:<12} {:<12} {:<12} {:<12} {:<12} {:<10} {:<10}",
        "K",
        "Coded",
        "Success%",
        "Overhead",
        "Prec(us)",
        "Enc(us)",
        "Dec(us)",
        "Enc+add",
        "Dec+add"
    );

    println!("=== RaptorQ (RFC 6330) Performance ===");
    println!(
        "Runs per K: {NUM_RUNS}, coded vectors: k * {OVERHEAD_NUMERATOR}/{OVERHEAD_DENOMINATOR}"
    );
    println!("Test all K from parameter table: {TEST_ALL_K}");
    println!("Code type: {:?}", CODE_TYPE);
    println!("\n{header}");
    println!("{}", "-".repeat(110));

    let mut all_results: Vec<TestResult> = Vec::new();
    let mut failed_ks: Vec<usize> = Vec::new();

    for &k in &ks {
        let num_coded = num_coded_vectors(k);

        let verify = panic::catch_unwind(AssertUnwindSafe(|| {
            verify_with_generic_harness(k, num_coded)
        }));
        match verify {
            Ok(Ok(())) => {}
            Ok(Err(msg)) => {
                println!(
                    "{:<8} {:<8} {:<12} {:<12} {:<12} {:<12} {:<12} {:<10} {:<10}",
                    k, num_coded, "VERIFY_FAIL", "N/A", "N/A", "N/A", "N/A", "N/A", "N/A"
                );
                eprintln!("  k={k}: {msg}");
                failed_ks.push(k);
                continue;
            }
            Err(_) => {
                println!(
                    "{:<8} {:<8} {:<12} {:<12} {:<12} {:<12} {:<12} {:<10} {:<10}",
                    k, num_coded, "VERIFY_PANIC", "N/A", "N/A", "N/A", "N/A", "N/A", "N/A"
                );
                failed_ks.push(k);
                continue;
            }
        }

        let result = panic::catch_unwind(AssertUnwindSafe(|| {
            let code = raptor_q_main::new_with_default_setting(k).with_code_type(CODE_TYPE);
            if code.num_padding() > 0 {
                return Err("generic benchmark requires K = K′ (num_padding > 0)".to_string());
            }
            Ok(test_code_scheme_multiple(NUM_RUNS, &code, k, num_coded))
        }));

        match result {
            Ok(Ok(results)) => {
                let stats = ExperimentStats::from_results(k, &results);
                all_results.extend(results);

                println!(
                    "{:<8} {:<8} {:<12.1} {:<12.4} {:<12.4} {:<12.4} {:<12.4} {:<10.2} {:<10.2}",
                    k,
                    num_coded,
                    stats.success_rate * 100.0,
                    stats.overhead_stats.mean,
                    stats.avg_precoding_time_us,
                    stats.avg_encoding_time_us,
                    stats.avg_decoding_time_us,
                    stats.encoding_operation_stats.vector_add
                        + stats.precoding_operation_stats.vector_add,
                    stats.decoding_operation_stats.vector_add,
                );
            }
            Ok(Err(msg)) => {
                println!(
                    "{:<8} {:<8} {:<12} {:<12} {:<12} {:<12} {:<12} {:<10} {:<10}",
                    k, num_coded, "BENCH_SKIP", "N/A", "N/A", "N/A", "N/A", "N/A", "N/A"
                );
                eprintln!("  k={k}: {msg}");
            }
            Err(_) => {
                println!(
                    "{:<8} {:<8} {:<12} {:<12} {:<12} {:<12} {:<12} {:<10} {:<10}",
                    k, num_coded, "FAILED", "N/A", "N/A", "N/A", "N/A", "N/A", "N/A"
                );
                failed_ks.push(k);
            }
        }
    }

    let json_path = format!("results/raptor_q_performance_{timestamp}.jsonl");
    save_test_results(&all_results, &json_path).expect("save JSONL results");
    println!("\nDetailed results: {json_path}");

    if !failed_ks.is_empty() {
        println!("Failed K values: {failed_ks:?}");
        let failed_path = format!("results/raptor_q_failed_ks_{timestamp}.txt");
        let mut failed_file = File::create(&failed_path)?;
        writeln!(failed_file, "Failed K values: {failed_ks:?}")?;
        println!("Failed K list: {failed_path}");
    }

    print_real_symbol_benchmarks();

    println!("\nDone.");
    Ok(())
}

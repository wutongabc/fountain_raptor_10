//! Performance evaluation for [`Raptor10SysCode`] (RFC 5053).
//!
//! For each K, runs one [`test_code_scheme_with_data_vectors`] pass (with
//! [`VecDataOperater`](fountain_utility::VecDataOperater)) to verify encode/decode
//! correctness, then [`test_code_scheme_multiple`] for operation-count and timing stats.
//!
//! Run from the workspace root:
//! ```text
//! cargo run -p fountain_raptor_10 --example raptor_10_performance --release
//! ```
//!
//! ## Real-symbol benchmark
//!
//! After the abstract harness, reports systematic-codec encode/decode wall times with
//! [`VecDataOperater`](fountain_utility::VecDataOperater)
//! at symbol sizes 128 and 1500 @ k = 5000 for **on-the-fly** and **deferred** execution
//! ([`fountain_utility::real_symbol_benchmark`](fountain_utility::real_symbol_benchmark)).
//! Table footnotes explain that on-the-fly `enc` is LT-only; see
//! [deferred-benchmark-optimization.md](../../docs/plans/deferred-benchmark-optimization.md) §2.1.
//!
//! For GE sub-phases with real bytes, use [`scheme_profile`](scheme_profile.rs):
//!
//! ```text
//! cargo run -p fountain_raptor_10 --example scheme_profile --release --features profiling -- -s 128 --operator slab 5000
//! ```

use fountain_engine::types::GF2_FIELD_POLY;
use fountain_raptor_10::Raptor10SysCode;
use fountain_utility::{
    OperatorFactory, RealSymbolBenchConfig, StandardRealSymbolSession, TestResult, TestStatistics,
    VecDataOperater, benchmark_deferred, benchmark_on_the_fly, make_test_messages,
    print_real_symbol_benchmark_table, save_test_results, test_code_scheme_multiple,
    test_code_scheme_with_data_vectors,
};
use std::fs::File;
use std::io::Write;
use std::panic::{self, AssertUnwindSafe};
use std::time::SystemTime;

const TEST_ALL_K: bool = false;
const K_MIN: usize = 5;
const K_MAX: usize = 500;
const NUM_RUNS: usize = 50;
const SYMBOL_SIZE: usize = 4;
const REAL_SYMBOL_K: usize = 5000;
const REAL_SYMBOL_SIZES: &[usize] = &[128, 1500];
const REAL_SYMBOL_RUNS: usize = 5;
const OVERHEAD_NUMERATOR: usize = 3;
const OVERHEAD_DENOMINATOR: usize = 2;

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
        (K_MIN..=K_MAX).collect()
    } else {
        vec![100, 200, 500, 1000, 2000, 5000, 9000]
    }
}

fn num_coded_vectors(k: usize) -> usize {
    k * OVERHEAD_NUMERATOR / OVERHEAD_DENOMINATOR
}

fn vec_operator_factory(symbol_size: usize) -> Box<dyn fountain_engine::DataOperator> {
    Box::new(VecDataOperater::new(symbol_size))
}

fn print_real_symbol_benchmarks() {
    let num_coded = num_coded_vectors(REAL_SYMBOL_K);
    let code = Raptor10SysCode::new_with_default_setting(REAL_SYMBOL_K);
    let session = StandardRealSymbolSession;

    let bench_config = |symbol_size: usize| {
        RealSymbolBenchConfig::new(&code, REAL_SYMBOL_K, symbol_size, num_coded)
            .with_field_pp(GF2_FIELD_POLY)
    };

    print_real_symbol_benchmark_table(
        &format!("Real-symbol benchmark (k={REAL_SYMBOL_K}, systematic codec)"),
        REAL_SYMBOL_RUNS,
        REAL_SYMBOL_SIZES,
        &[("VecDataOperater", vec_operator_factory as OperatorFactory)],
        &|symbol_size, factory| {
            let config = bench_config(symbol_size);
            let messages = make_test_messages(REAL_SYMBOL_K, symbol_size);
            benchmark_on_the_fly(&code, &session, &config, &messages, factory)
        },
        &|symbol_size, factory| {
            let config = bench_config(symbol_size);
            let messages = make_test_messages(REAL_SYMBOL_K, symbol_size);
            benchmark_deferred(&code, &session, &config, &messages, factory)
        },
    );

    println!(
        "(GE sub-phases: scheme_profile --release --features profiling -- -s <size> --operator slab {REAL_SYMBOL_K})"
    );
}

/// One on-the-fly roundtrip with message bytes via `VecDataOperater`.
fn verify_with_data_operator(k: usize, num_coded: usize) -> Result<(), String> {
    let code = Raptor10SysCode::new_with_default_setting(k);
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

    println!("=== Raptor10 (RFC 5053) Performance ===");
    println!(
        "Runs per K: {NUM_RUNS}, coded vectors: k * {OVERHEAD_NUMERATOR}/{OVERHEAD_DENOMINATOR}"
    );
    println!("Test all K in [{K_MIN}, {K_MAX}]: {TEST_ALL_K}");
    println!("\n{header}");
    println!("{}", "-".repeat(110));

    let mut all_results: Vec<TestResult> = Vec::new();
    let mut failed_ks: Vec<usize> = Vec::new();

    for &k in &ks {
        let num_coded = num_coded_vectors(k);

        let verify =
            panic::catch_unwind(AssertUnwindSafe(|| verify_with_data_operator(k, num_coded)));
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
            let code = Raptor10SysCode::new_with_default_setting(k);
            test_code_scheme_multiple(NUM_RUNS, &code, k, num_coded)
        }));

        match result {
            Ok(results) => {
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
            Err(_) => {
                println!(
                    "{:<8} {:<8} {:<12} {:<12} {:<12} {:<12} {:<12} {:<10} {:<10}",
                    k, num_coded, "FAILED", "N/A", "N/A", "N/A", "N/A", "N/A", "N/A"
                );
                failed_ks.push(k);
            }
        }
    }

    let json_path = format!("results/raptor_10_performance_{timestamp}.jsonl");
    save_test_results(&all_results, &json_path).expect("save JSONL results");
    println!("\nDetailed results: {json_path}");

    if !failed_ks.is_empty() {
        println!("Failed K values: {failed_ks:?}");
        let failed_path = format!("results/raptor_10_failed_ks_{timestamp}.txt");
        let mut failed_file = File::create(&failed_path)?;
        writeln!(failed_file, "Failed K values: {failed_ks:?}")?;
        println!("Failed K list: {failed_path}");
    }

    print_real_symbol_benchmarks();

    println!("\nDone.");
    Ok(())
}

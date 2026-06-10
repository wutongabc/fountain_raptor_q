# Changelog

All notable changes to the published `fountain_raptor_q` crate are documented here.

## [1.2.0] - 2026-06-08

### Added

- **RFC 6330 padding** via `fountain_utility::padding_codec` and `fountain_engine` v1.3.1 native padding (`new_with_num_source`):
  - `RaptorQEncoder` / `RaptorQDecoder` type aliases (`PaddedEncoder` / `PaddedDecoder` over `raptor_q_main`).
  - `raptor_q_main::source_symbols`, `block_symbols`, `num_padding`, and `padded_*` helpers.
  - `RaptorQRealSymbolSession` for deferred encode/decode benchmarks with padding.
- `padding_codec_test` integration tests (K < K′ round-trip, execute-only operator path).

### Changed

- **Dependencies** bumped to `fountain_engine` **1.3.1**, `fountain_scheme` **1.3.0**, `fountain_utility` **1.3.0**.
- Encoder/decoder call sites use `&CodeScheme` references (published engine API).
- Removed monorepo-only dev dependencies (`fountain_operators`, `profiling`, `legacy_solver`, `compare-ref`).

### Packaging

- Crates.io tarball includes round-trip **examples** (`test_single_k_raptor_q`, `test_all_k_raptor_q`, `raptor_q_performance`, …).
- Excludes engine2-only tooling (`scheme_profile`, `ref_raptorq_compare`, `performance_analysis/`).
- `raptor_q_performance` includes portable real-symbol benchmarks using `VecDataOperater`.

# Changelog

All notable changes to the published `fountain_raptor_q` crate are documented here.

## [2.0.0] - Unreleased

### Breaking Changes

- Migrated the public Fountain stack to `fountain_engine` 2.0.1,
  `fountain_scheme` 2.0.0, and `fountain_utility` 2.0.0.
- Public API types from the Fountain stack now come from their 2.x crates.
  Downstream applications that directly depend on these crates must update
  them together; 1.x and 2.x versions of same-named Rust types are not
  interchangeable.

### Migration

- Update `fountain_raptor_q` to 2.0 and direct `fountain_engine`,
  `fountain_scheme`, and `fountain_utility` dependencies to 2.x.
- See **New in 2.0: Migrating from the 1.x Fountain Stack** in the README for
  dependency examples.

### Added

- Documented and tested ordinary (non-systematic) encoding, including its LT
  packet-ID range, representative round trips, and RFC-table padding behavior.
- Added `code_type_performance` for automatic systematic/ordinary end-to-end
  comparison and `hdpc_cache_performance` for verified ordinary Encoder cache
  measurements through the complete encode/decode path.

### Performance

- Measured uncached, cold-cache, and warm-cache ordinary Encoder construction.
  Current full-path results do not show a stable, material cache speedup; see
  the README for the reproducible command, methodology, and measured values.

### Validation

- Added an integration test that compiles the public API against the Fountain
  2.x `CodeScheme`, `CodeType`, `SubstitutionMethod`, `DataOperator`, encoder,
  decoder, HDPC, and padding-wrapper types.
- Added cache equivalence, clone-sharing, and concurrent-sharing checks, plus
  systematic/ordinary packet-ID and recovered-data comparisons.

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

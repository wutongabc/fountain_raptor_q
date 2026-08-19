# fountain_raptor_q

RaptorQ fountain code scheme built on `fountain_engine`.

This crate provides `raptor_q_main`, a systematic RaptorQ code scheme based on
RFC 6330 parameters, degree generation, LDPC relationships, and RFC octet-field
HDPC arithmetic. It is designed to be used with the generic `Encoder` and
`Decoder` types from `fountain_engine`.

## New in 2.0: Migrating from the 1.x Fountain Stack

> **Breaking dependency migration:** `fountain_raptor_q` 2.0 uses
> `fountain_engine` 2.x, `fountain_scheme` 2.x, and `fountain_utility` 2.x.
> Applications upgrading from `fountain_raptor_q` 1.x should update the
> Fountain crates together.

The major-version update is required because this crate's public API accepts
and returns Fountain types such as `CodeType`, `SubstitutionMethod`, `Encoder`,
`Decoder`, `DataOperator`, and the padding wrappers. Rust treats a type from
`fountain_engine` 1.x as different from the same-named type in
`fountain_engine` 2.x, so mixing the two generations can produce type-mismatch
errors.

Update direct dependencies as a set:

```toml
# Before: fountain_raptor_q 1.x
fountain_raptor_q = "1.2"
fountain_engine = "1.3"
fountain_utility = "1.3"

# After: fountain_raptor_q 2.x
fountain_raptor_q = "2.0"
fountain_engine = "2.0"
fountain_utility = "2.0"
```

If the application directly uses `fountain_scheme`, update it to 2.x as well.
After the dependency update, rebuild the application so all public Fountain
types come from the same major-version family.

## Quick Start

Add the crate to your project. `fountain_utility` is optional, but it provides
the in-memory `VecDataOperater` used in the example below.

```toml
[dependencies]
fountain_raptor_q = "2.0"
fountain_engine = "2.0"
fountain_utility = "2.0"
```

Create a RaptorQ systematic code scheme:

```rust
use fountain_engine::CodeScheme;
use fountain_raptor_q::raptor_q_main::raptor_q_main;

let source_symbols = 100usize;
let code = raptor_q_main::new_with_default_setting(source_symbols);
let params = code.get_params();

assert!(params.k >= source_symbols);
assert!(params.h > 0);
assert!(params.l > 0);
```

Use this `code` with `fountain_engine::Encoder` and `fountain_engine::Decoder`.
The example below shows a complete in-memory encode/decode flow.

## Complete Encode/Decode Example

Load `K'` fixed-size source symbols into an encoder, generate coded symbols,
then feed received symbols into a decoder.

RaptorQ uses RFC 6330's extended source-symbol count `K'`, exposed as
`params.k`. If your payload has only `K` source symbols, pad it to `K'` before
encoding. Repair symbol IDs start at `params.num_total()`. The gap between
`params.k` and `params.num_total()` is reserved for precode symbols and should
not be sent as repair symbols.

```rust
use std::collections::HashMap;

use fountain_engine::{CodeScheme, DataOperator, DecodeStatus};
use fountain_raptor_q::raptor_q_main::raptor_q_main;
use fountain_utility::VecDataOperater;

fn main() {
    let source_symbols = 8usize;
    let symbol_size = 4usize;
    let code = raptor_q_main::new_with_default_setting(source_symbols);
    let params = code.get_params();
    let k_prime = params.k;

    // Your application should split and pad its payload into K' equally sized
    // symbols. This sample uses deterministic bytes for a compact round trip.
    let source_vectors: Vec<Vec<u8>> = (0..k_prime)
        .map(|i| vec![i as u8, i as u8 + 1, i as u8 + 2, i as u8 + 3])
        .collect();

    // Load the source symbols into an in-memory data operator.
    let mut encode_operator = VecDataOperater::new(symbol_size);
    for (source_id, symbol) in source_vectors.iter().enumerate() {
        encode_operator.insert_vector(symbol, source_id);
    }

    // Build an encoder from the RaptorQ scheme and the data operator.
    let mut encoder = code.new_encoder_with_operator(Box::new(encode_operator));
    let mut esi_to_data_id = HashMap::new();

    // Encode the K' systematic source symbols.
    for esi in 0..k_prime {
        if let Some(data_id) = encoder.encode_coded_vector(esi) {
            esi_to_data_id.insert(esi, data_id);
        }
    }

    // Encode a few repair symbols. These are useful when some source symbols are lost.
    let first_repair_esi = params.num_total();
    for esi in first_repair_esi..first_repair_esi + 4 {
        if let Some(data_id) = encoder.encode_coded_vector(esi) {
            esi_to_data_id.insert(esi, data_id);
        }
    }

    let encoded_operator = encoder.manager.move_operator();

    // In a real protocol, send each pair `(esi, encoded_symbol)` over the channel.
    // The decoder only needs the ESI and the bytes for each received symbol.
    let mut decoder = code.new_decoder_with_operator(
        Box::new(VecDataOperater::new(symbol_size)),
    );

    let mut decoded = false;
    for esi in 0..k_prime {
        let data_id = esi_to_data_id[&esi];
        let status = decoder.add_coded_vector(esi, encoded_operator.get_vector(data_id));
        if matches!(status, DecodeStatus::Decoded) {
            decoded = true;
            break;
        }
    }

    assert!(decoded);

    let decoded_operator = decoder.manager.move_operator();
    for source_id in 0..k_prime {
        assert_eq!(decoded_operator.get_vector(source_id), source_vectors[source_id]);
    }
}
```

## What It Provides

- `raptor_q_main`: a systematic RaptorQ scheme implementing
  `fountain_engine::traits::CodeScheme`.
- RFC 6330 parameter-table lookup for `K`, `K'`, LDPC symbols, HDPC symbols,
  pre-inactive symbols, and systematic index values.
- RFC 6330 random, tuple, degree, and degree-set generators.
- RaptorQ LDPC and HDPC construction helpers aligned with the
  `fountain_engine` variable layout.
- Examples for full K-table validation, single-K smoke tests, and performance
  experiments.

## Ordinary (Non-Systematic) Encoding

The default remains systematic encoding. Select ordinary encoding explicitly
with `with_code_type(CodeType::Ordinary)`:

```rust
use fountain_engine::{CodeScheme, CodeType};
use fountain_raptor_q::raptor_q_main::raptor_q_main;

let code = raptor_q_main::new_with_default_setting(100)
    .with_code_type(CodeType::Ordinary);
let params = code.get_params();

// Ordinary packets are LT symbols. Their ESI range starts after all source and
// precode symbols; IDs below this boundary are not ordinary output packets.
let first_ordinary_esi = params.num_total();
assert!(first_ordinary_esi > params.k);
```

For systematic encoding, source-symbol ESIs are `0..K` and repair ESIs start
at `params.num_total()`. For ordinary encoding, all transmitted ESIs start at
`params.num_total()`.

## Ordinary Encoder Cache Benchmark

The HDPC LU cache is used by ordinary precoding, not by the normal systematic
encoding path. Run the standalone benchmark through the complete ordinary
encoder and decoder workflow:

```bash
cargo run --release --locked --example hdpc_cache_performance
```

The example compares an uncached encoder, a fresh cached encoder, and repeated
encoders sharing a warmed cache. It reports both full precoding time and total
encoding time, and every measurement performs a complete decode and verifies
the recovered source data; it never calls `HDPC::lu_idssh` directly. Measurements
use `CodeType::Ordinary`, 128-byte symbols, and `2 * K′` LT packets. Each value
below is the median of five benchmark executions, each containing five verified
round trips (2026-08-19). Speedup is `uncached / warm`.

| K | Uncached precoding | Warm precoding | Precoding speedup | Uncached total encoding | Warm total encoding | Total speedup |
|---:|---:|---:|---:|---:|---:|---:|
| 100 | 0.735 ms | 0.721 ms | 1.02x | 0.995 ms | 0.978 ms | 1.02x |
| 1000 | 2.223 ms | 2.154 ms | 1.03x | 4.992 ms | 4.697 ms | 1.06x |
| 5008 | 8.055 ms | 8.169 ms | 0.99x | 23.167 ms | 23.211 ms | 1.00x |

No stable, material end-to-end cache speedup is visible: the small differences
change with K and are comparable to timing variation. Absolute timings depend
on hardware and load; pass K values after `--` to rerun targeted cases.

## Systematic vs Ordinary Performance

Run both code types automatically without changing the original performance
example:

```bash
cargo run --release --locked --example code_type_performance
```

It uses identical source data, symbol size, packet budget, and K values, checks
every recovered source symbol, and reports median encode/decode times. Pass K
values after `--` for targeted runs.

## Validation

Run the crate tests:

```bash
cargo test
```

Run the multi-threaded all-K validation against the RFC 6330 parameter table:

```bash
cargo run --release --example test_all_k_raptor_q_multi_threads
```

The all-K validation checks every K value in `src/raptor_q_para.csv` and reports
any unsolvable or panicking cases.

## License

MIT License. See [LICENSE](LICENSE).

## Authors

Zigeng Xu and Shenghao Yang.

Copyright (c) 2025, 2026 Zigeng Xu and Shenghao Yang. All rights reserved.

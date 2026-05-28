# fountain_raptor_q

RaptorQ fountain code scheme built on `fountain_engine`.

This crate provides `raptor_q_main`, a systematic RaptorQ code scheme based on
RFC 6330 parameters, degree generation, LDPC relationships, and RFC octet-field
HDPC arithmetic. It is designed to be used with the generic `Encoder` and
`Decoder` types from `fountain_engine`.

## Quick Start

Add the crate to your project. `fountain_utility` is optional, but it provides
the in-memory `VecDataOperater` used in the example below.

```toml
[dependencies]
fountain_raptor_q = "1.1"
fountain_engine = "1.2"
fountain_utility = "1.1"
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

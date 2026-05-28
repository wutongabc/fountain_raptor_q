# `raptor_q_fix` Branch Notes

This note records the main RaptorQ-related fixes on this branch, with focus on the RFC 6330 finite field and the code paths that were changed to support it.

## 1. RFC finite field vs. the old `fountain_engine` default

Reference:

- [RFC 6330](https://www.rfc-editor.org/rfc/rfc6330)
- especially Section 5.7, Section 5.7.2, Section 5.7.3, and Section 5.7.4

RFC 6330 does not describe RaptorQ octet arithmetic as an implementation detail. It defines it as part of the codec itself.

In Section 5.7, RFC 6330 says that:

- octets are treated as elements of `GF(256)`
- symbol operations and matrix operations are built on top of those octet operations

In Section 5.7.2, the RFC defines:

- octet addition as XOR
- octet multiplication using `OCT_EXP` and `OCT_LOG`
- octet division using `OCT_EXP` and `OCT_LOG`
- `alpha` as the octet with integer value `2`
- `alpha^^i = OCT_EXP[i]`

This means the actual field is determined by the RFC tables, not by a separate implementation choice.

The key RFC evidence is the beginning of `OCT_EXP` in Section 5.7.3:

- `1, 2, 4, 8, 16, 32, 64, 128, 29, 58, 116, 232, ...`

From this table:

- `alpha = 2`
- `alpha^8 = OCT_EXP[8] = 29`

That sequence matches reduction by the primitive polynomial:

- `x^8 + x^4 + x^3 + x^2 + 1`
- hexadecimal: `0x11D`

By contrast, the old `fountain_engine` default field used `0x11B`, which would produce:

- `1, 2, 4, 8, 16, 32, 64, 128, 27, 54, 108, 216, ...`

So even though RFC 6330 does not spell out the literal text "use primitive polynomial `0x11D`", the polynomial follows directly from the `OCT_EXP` table given by the RFC. This is an implementation inference from the normative table, not a separate design choice.

Before this branch, `fountain_engine` used:

- `fountain_engine/src/algebra/finite_field.rs`
- `GF256::default() -> GF256::new(0x11B)`

So the old default field and the RFC field were different:

- old engine default: `0x11B`
- RFC 6330 RaptorQ: `0x11D`

This difference matters because RFC 6330 uses the same octet arithmetic in:

- HDPC construction and multiplication
- symbol operations
- matrix operations
- decoding-side elimination

So for RaptorQ, the field is part of the wire-compatible algorithm. In practice, some `K'` values could end up one rank short when RaptorQ used the old engine default field.

Note: inside `GF256`, the stored `primitive_polynomial` is the low byte, so `0x11D` is stored as `0x1D`.

## 2. How `fountain_engine` was changed to support the RFC field

Goal: keep the old default behavior unchanged, and let only RaptorQ explicitly request the RFC field.

### 2.1 Default behavior was kept

- `fountain_engine/src/algebra/finite_field.rs`
  - `impl Default for GF256 { fn default() -> Self { Self::new(0x11B) } }`

So existing code that uses the old default path still gets `0x11B`.

### 2.2 Explicit field-aware constructors were added

- `fountain_engine/src/encoder.rs`
  - `Encoder::new_with_gf(...)`
  - `Encoder::new_with_operator_and_gf(...)`

- `fountain_engine/src/decoder.rs`
  - `Decoder::new_with_gf(...)`
  - `Decoder::new_with_operator_and_gf(...)`

These are the new entry points used by RaptorQ.

### 2.3 `DataManager` now carries the active `GF256`

- `fountain_engine/src/data_manager.rs`
  - added field: `gf256: GF256`
  - added `set_gf256_primitive_polynomial(...)`
  - added `gf256()`
  - `set_operator(...)` now pushes the current field into the operator
  - `multiply_scalar(...)` and `divide_scalar(...)` now use `self.gf256`

This is the core change that lets one code path use `0x11B` and another use `0x11D`.

### 2.4 `DataOperator` and `VecDataOperater` were extended

- `fountain_engine/src/traits/data_operator.rs`
  - added hook: `set_gf256_primitive_polynomial(...)`

- `fountain_utility/src/vec_data_operater.rs`
  - `new(...)` still keeps the old default behavior
  - added `new_with_gf256_primitive_polynomial(...)`
  - implemented `set_gf256_primitive_polynomial(...)`

So the vector operator can now follow the same field as the engine manager.

### 2.5 Solver/precode paths now read the manager field

These places were changed to use `manager.gf256()` instead of `GF256::default()`:

- `fountain_engine/src/core/precode.rs`
- `fountain_engine/src/core/inactivation.rs`
- `fountain_engine/src/core/solver.rs`

This makes the rank/LU steps use the same field that RaptorQ explicitly requested.

### 2.6 How RaptorQ now explicitly asks for `0x11D`

- `raptor_q/src/rfc6330_hdpc.rs`
  - added `RFC6330_GF256_PRIMITIVE_POLYNOMIAL: u16 = 0x11D`
  - added local `RFC6330HDPC`
  - this HDPC implementation constructs its own `GF256::new(0x11D)`

- `raptor_q/src/raptor_q_main.rs`
  - added:
    - `new_encoder()`
    - `new_encoder_with_operator()`
    - `new_decoder()`
    - `new_decoder_with_operator()`
  - these helpers call:
    - `Encoder::new_with_gf(..., 0x11D)`
    - `Encoder::new_with_operator_and_gf(..., 0x11D)`
    - `Decoder::new_with_gf(..., 0x11D)`
    - `Decoder::new_with_operator_and_gf(..., 0x11D)`

- `raptor_q/tests/raptor_q_codec_test.rs`
  - `test_rfc_field_configuration_reaches_engine_managers`
  - verifies that the encoder/decoder managers receive `0x1D` internally

In short: the engine default stays `0x11B`, and RaptorQ now opts into `0x11D` explicitly.

## 3. Other fixes on this branch

### 3.1 Fix `P1` to use `P`, not `L`

Commit:

- `b06a505 fix p1 from find_smallest_prime_gte(l) to find_smallest_prime_gte(p)`

Code changes:

- `raptor_q/src/generators/tuple_gen.rs`
  - now computes:
    - `let p = l - w;`
    - `let p1 = find_smallest_prime_gte(p);`

- `raptor_q/src/generators/rfc6330_degree_set.rs`
  - same correction:
    - `let p = l - w;`
    - `let p1 = find_smallest_prime_gte(p);`

This fixes the RFC meaning of `P1` for the tuple and degree-set generation path.

### 3.2 Align the flat engine mapping with RFC `W / P`

- `raptor_q/src/raptor_q_main.rs`
  - `W` is read from the RFC parameter table
  - `CodeParams` is now built so that `a + l = W`
  - this makes the engine-side active boundary match RFC `W`

- `raptor_q/src/generators/rfc6330_degree_set.rs`
  - LT indices stay in local domain `[0, W)`
  - PI indices stay in local domain `[0, P)`

- `raptor_q/src/raptor_q_main.rs`
  - the degree-set adapter offsets PI indices by `num_active()`

This is the key `W / P` alignment that was needed for RFC-style degree sets.

### 3.3 Add RFC-style LDPC instead of relying on the old generic path

Commit:

- `127bd48 RQLDPC`

Code changes:

- `raptor_q/src/RQLDPC.rs`
  - added a local RFC 6330 LDPC implementation
  - implements RFC 6330 Section 5.3.3.3
  - matches the engine variable order used by `raptor_q_main`:
    - active: `B | S`
    - inactive: `U | H`

- `raptor_q/src/raptor_q_main.rs`
  - `new_with_default_setting(...)` now uses `LDPCType::RQLDPC`

### 3.4 Fix a copied RFC table value in the degree CDF table

- `raptor_q/src/generators/degree_gen.rs`
  - corrected:
    - `937111` -> `937311`

This is a confirmed copied-table error from the RFC degree CDF table.

During this branch, the other copied RFC lookup tables were also rechecked. The confirmed value fix that landed in code is the `937311` entry above.

The same file also adds boundary tests:

- `rfc6330_degree_cdf_table_matches_table_1_boundary`
- `degree_boundary_around_rfc_table_index_9`

### 3.5 Add local RFC HDPC

Commit path:

- mainly in `4c0534e fix all raptor_q test`

Code changes:

- `raptor_q/src/rfc6330_hdpc.rs`
  - adds a local HDPC implementation for RaptorQ
  - uses RFC delta-column generation and RFC field `0x11D`

- `raptor_q/src/raptor_q_main.rs`
  - `rfc6330_hdpc(...)` now returns `RFC6330HDPC`

### 3.6 Tests and examples were updated around the new path

Examples/tests touched on this branch include:

- `raptor_q/examples/test_single_k_raptor_q.rs`
- `raptor_q/examples/test_all_k_raptor_q.rs`
- `raptor_q/examples/test_all_k_raptor_q_multi_threads.rs`
- `raptor_q/tests/raptor_q_codec_test.rs`
- `raptor_q/tests/tuple_gen_test.rs`
- `raptor_q/tests/degree_gen_test.rs`
- `raptor_q/tests/params_table_test.rs`
- `raptor_q/tests/rand_num_gen_test.rs`

The main purpose of these updates is to keep the RFC-aligned RaptorQ path testable and to cover the corrected parameter/table behavior.

## 4. Short branch history

Recent branch landmarks:

- `59be8f3 Revert incorrect RaptorQ changes`
- `990ecb0 only 36479 goes wrong`
- `104b800 modify test and decrease redudant comments`
- `b06a505 fix p1 from find_smallest_prime_gte(l) to find_smallest_prime_gte(p)`
- `127bd48 RQLDPC`
- `4c0534e fix all raptor_q test`

In summary, this branch does three main things:

1. fixes RFC-alignment bugs in RaptorQ (`P1`, `W/P` mapping, LDPC, HDPC, degree-table value),
2. keeps `fountain_engine` backward compatible with the old `0x11B` default,
3. lets only RaptorQ explicitly use the RFC field `0x11D`.

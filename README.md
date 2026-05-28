# RaptorQ Library - RFC 6330 Implementation

A Rust implementation of RaptorQ fountain codes as specified in **RFC 6330**. This library provides standardized RaptorQ parameters and convenient configuration builders for creating fountain codes with high reliability and efficiency.

## What is RaptorQ?

RaptorQ is an advanced **fountain code** designed for reliable data transmission over unreliable networks (like wireless, satellite, or lossy Internet connections).

### Key Concepts:

1. **Fountain Codes**: Like a fountain that produces an infinite stream of water droplets, fountain codes can generate an unlimited number of encoded symbols from a limited set of source symbols. You need only a small fraction of encoded symbols to recover the original data.

2. **RFC 6330**: RaptorQ is standardized in RFC 6330, which specifies exact parameters for different source symbol counts (k values), ensuring compatibility across implementations.

3. **Erasure Coding**: Protects data against symbol loss/corruption by creating redundant encoded symbols. If some symbols are lost, you can still recover the original data.

---

## Architecture Overview

The RaptorQ library is built on top of three core libraries:

```
鈹屸攢鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹?
鈹?       RaptorQ Library              鈹?
鈹?  (Configuration & Parameters)      鈹?
鈹溾攢鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹?
鈹?      Fountain Engine               鈹?
鈹? (Core Encoding/Decoding Logic)     鈹?
鈹溾攢鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹?
鈹?      Fountain Basic                鈹?
鈹? (LT Codes, LDPC, HDPC, Precodes)  鈹?
鈹溾攢鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹?
鈹?      Fountain Utility              鈹?
鈹? (Statistics & Testing Tools)       鈹?
鈹斺攢鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹€鈹?
```

### Module Structure

```
raptor_q/
鈹溾攢鈹€ src/
鈹?  鈹溾攢鈹€ lib.rs                 # Library entry point, exports all modules
鈹?  鈹溾攢鈹€ params_table.rs        # RFC 6330 parameter lookup from CSV
鈹?  鈹溾攢鈹€ raptorq_config.rs      # Configuration builders (standard/systematic/custom)
鈹?  鈹溾攢鈹€ raptorq_lt.rs          # RaptorQ-LT code implementation
鈹?  鈹溾攢鈹€ raptorq_sys.rs         # RaptorQ systematic code implementation
鈹?  鈹斺攢鈹€ generators/            # RFC 6330 compliant encoding symbol generators
鈹?      鈹溾攢鈹€ mod.rs             # Module exports
鈹?      鈹溾攢鈹€ rand_num_gen.rs    # RFC 6330 random number generator
鈹?      鈹溾攢鈹€ tuple_gen.rs       # Tuple (d,a,b,d1,a1,b1) generator
鈹?      鈹溾攢鈹€ degree_gen.rs      # Degree distribution generator
鈹?      鈹斺攢鈹€ rfc6330_degree_set.rs  # Complete RFC 6330 degree set
鈹溾攢鈹€ examples/
鈹?  鈹溾攢鈹€ raptorq_example.rs     # Configuration usage examples
鈹?  鈹斺攢鈹€ raptorq_lt_sys_example.rs  # Full encode/decode examples
鈹溾攢鈹€ tests/
鈹?  鈹溾攢鈹€ config_test.rs         # Configuration tests
鈹?  鈹溾攢鈹€ params_table_test.rs   # Parameter lookup tests
鈹?  鈹溾攢鈹€ tuple_gen_test.rs      # Tuple generator tests
鈹?  鈹溾攢鈹€ degree_gen_test.rs     # Degree generator tests
鈹?  鈹斺攢鈹€ raptorq_codec_test.rs  # Full codec tests
鈹斺攢鈹€ raptor_q_para.csv          # RFC 6330 parameter table
```

---

## Core Components

### 1. **params_table.rs** - RFC 6330 Parameters

This module loads and manages the standardized RaptorQ parameters from `raptor_q_para.csv`.

**Key Struct:**
```rust
pub struct RaptorQParams {
    pub k: usize,        // Original number of source symbols
    pub k_prime: usize,  // Extended source symbols (K') - required for processing
    pub l: usize,        // LDPC symbols (S) - for Low-Density Parity Check
    pub h: usize,        // HDPC symbols (H) - for High-Density Parity Check
    pub pi: usize,       // Number of pre-inactive symbols
    pub total_l: usize,  // Total intermediate symbols (K' + S + H)
    pub j: usize,        // Systematic index (J)
}
```

**Key Functions:**
- `get_params(k)` - Lookup RaptorQ parameters for given k
- `get_k_prime(k)` - Get extended symbol count
- `get_inactive_number(k)` - Get pre-inactive symbol count
- `get_systematic_index(k)` - Get systematic encoding index

**Example Usage:**
```rust
use raptor_q::get_params;

// Look up parameters for k=100 source symbols
if let Some(params) = get_params(100) {
    println!("K' (extended): {}", params.k_prime);
    println!("S (LDPC): {}", params.l);
    println!("H (HDPC): {}", params.h);
}
```

### 2. **raptorq_config.rs** - Configuration Builders

Provides convenient builders for creating RaptorQ configurations.

**Main Types:**

#### **RaptorQConfig** - Factory for standard configurations

```rust
use raptor_q::RaptorQConfig;
use fountain_scheme::config::CodeConfig;

// Standard (non-systematic) RaptorQ
let config = RaptorQConfig::new(100);

// Systematic RaptorQ - first k symbols are original data
let config = RaptorQConfig::systematic(100);

// Custom configuration with builder pattern
let config = RaptorQConfig::custom(100)
    .with_hdpc_type(HDPCType::Random)
    .with_degree_set(DegreeSetConfig::RandomRobust { c: 0.1, delta: 0.5 })
    .with_code_type(CodeType::Systematic)
    .build();
```

#### **RaptorQConfigBuilder** - Advanced customization

Allows fine-grained control while using standard RFC 6330 parameters:

```rust
let builder = RaptorQConfig::custom(100)
    .with_hdpc_type(HDPCType::Random)        // Random HDPC generation
    .with_ldpc_type(LDPCType::RQLDPC)        // RFC 6330 LDPC
    .with_degree_set(DegreeSetConfig::RandomIdealTruncated { dmax: 30 })
    .with_code_type(CodeType::Ordinary);      // Non-systematic

let config = builder.build();
```

---

### 3. **raptorq_lt.rs** - RaptorQ-LT Implementation

Implements **RaptorQ-LT (Non-Systematic) Codes**.

**Key Struct:**
```rust
pub struct RaptorQLTCode {
    degree_distribution: DegreeDistribution,  // How symbol connections are distributed
    params: CodeParams,                        // Coding parameters
    hdpc_type: HDPCType,                      // HDPC configuration
    ldpc_type: LDPCType,                      // LDPC configuration
}
```

**Key Methods:**
```rust
// Create with custom parameters
let code = RaptorQLTCode::new(
    100,                    // k: source symbols
    30,                     // dmax: max degree
    HDPCType::Default,
    LDPCType::RQLDPC
);

// Create with RFC 6330 defaults
let code = RaptorQLTCode::new_with_default_setting(100);

// Get parameters
let params = code.get_params();
```

**Usage with Encoder/Decoder:**
```rust
use fountain_engine::encoder::Encoder;
use fountain_engine::decoder::Decoder;
use raptor_q::RaptorQLTCode;

let code = RaptorQLTCode::new_with_default_setting(100);
let encoder = Encoder::new(code);
let decoder = Decoder::new(code);
```

---

### 4. **raptorq_sys.rs** - RaptorQ Systematic Implementation

Implements **RaptorQ Systematic Codes** for better streaming performance.

**Key Struct:**
```rust
pub struct RaptorQSysCode {
    params: CodeParams,
    hdpc_type: HDPCType,
    ldpc_type: LDPCType,
}
```

**Key Methods:**
```rust
// Standard systematic code
let code = RaptorQSysCode::new_with_default_setting(100);

// Custom HDPC type
let code = RaptorQSysCode::new_with_hdpc(100, HDPCType::Random);
```

**Advantage of Systematic Codes:**
- First k encoded symbols are identical to original source symbols
- Better for streaming: receiver can decode as soon as first k symbols arrive
- Reduces re-transmission overhead

---

### 5. **generators/** - RFC 6330 Compliant Generators

The `generators/` directory contains RFC 6330 compliant implementations for generating encoding parameters.

**Files:**
- **`mod.rs`**: Module exports and public API
- **`rand_num_gen.rs`**: Random number generator as per RFC 6330 Section 5.3.5.1
- **`tuple_gen.rs`**: Tuple generator (d, a, b, d1, a1, b1) as per RFC 6330 Section 5.3.5.4
- **`degree_gen.rs`**: Degree distribution generator based on tuple parameters
- **`rfc6330_degree_set.rs`**: Complete RFC 6330 degree set generator implementation

**Key Component: RFC6330DegreeSet**

This is a faithful implementation of RFC 6330's encoding symbol generation algorithm:

```rust
pub struct RFC6330DegreeSet {
    k_prime: usize,  // K' from RFC 6330
    // Internal state for random generation...
}

impl RFC6330DegreeSet {
    pub fn new(k_prime: usize) -> Self { /* ... */ }
    
    // Generate (active_indices, inactive_indices) for ESI
    pub fn degree_set(&mut self, esi: usize) -> (Vec<usize>, Vec<usize>) { /* ... */ }
}
```

**Why RFC6330DegreeSet Cannot Be Used with RaptorQSysCode in Current Framework**

鈿狅笍 **Important Limitation**: While `RFC6330DegreeSet` provides a standard-compliant implementation, it **cannot** be directly used with `RaptorQSysCode` in the current `fountain_engine` framework, because the systematic encoding approach is different from the non-systematic encoding approach. Here's the detailed explanation:

**The Framework's Systematic Encoding Approach:**

The `fountain_engine` framework implements systematic encoding using a **"pre-computation solver"** approach (as described in `doc-erasurecodes.org`):

1. **Step 1 (Pre-encoding):** The encoder internally runs a decoder-like solver to compute intermediate symbols \(\tilde{B}\) by solving the equation: `G_sys 路 B_tilde = B`
2. **Step 2:** It requires the first `a` coded vectors to form a **lower-triangular matrix structure**
3. **Step 3:** This triangular structure ensures the internal BP (Belief Propagation) decoder can solve all active variables efficiently

**The Incompatibility:**

- RFC6330 directly use the source symbols as the first `k` encoded symbols, and only apply encoding with the repair symbols (ESI 鈮?K)
- `RFC6330DegreeSet` generates **random, non-triangular** degree sequences optimized for repair symbol generation (ESI 鈮?K)
- When used for systematic encoding's first `a` vectors, it creates a random matrix instead of the required triangular structure
- This causes the internal solver to get stuck in BP phase, unable to transition to GE phase
- Result: `assert!(solver.phase == DecodePhase::GE)` fails at line 56 in `encoder.rs`

**The Solution:**

`RaptorQSysCode` must use `TriangularIdealTruncated` degree set configuration:

```rust
fn create_degree_set_fn(&self) -> Box<dyn FnMut(usize) -> (Vec<usize>, Vec<usize>)> {
    let degree_set_config = fountain_scheme::degree_sets::DegreeSetConfig::TriangularIdealTruncated { dmax: 30 };
    let mut degree_set = degree_set_config.create(&self.params);
    Box::new(move |coded_id| degree_set.degree_set(coded_id))
}
```

**Alternative Approaches:**

To use `RFC6330DegreeSet` with systematic codes, the framework would need to implement RFC 6330's native systematic encoding approach (Section 5.4), which uses:
- Direct mapping for systematic symbols (ESI < K': output = C[ESI])
- RFC6330DegreeSet only for repair symbols (ESI 鈮?K')

This would require substantial modifications to `fountain_engine`'s core `Encoder` logic.

**Best Practice:**
- Use `RFC6330DegreeSet` with `RaptorQLTCode` (non-systematic, ordinary encoding) 鉁?
- Use `TriangularIdealTruncated` with `RaptorQSysCode` (systematic encoding) 鉁?

---

## Rust Concepts Explained (For Beginners)

### 1. **Traits (Similar to Interfaces in Java/C++)**

In Rust, traits define shared behavior that types must implement:

```rust
// Trait definition
pub trait CodeScheme {
    fn get_params(&self) -> CodeParams;
    fn code_type(&self) -> CodeType;
}

// Implementation
impl CodeScheme for RaptorQLTCode {
    fn get_params(&self) -> CodeParams {
        self.params.clone()
    }
    
    fn code_type(&self) -> CodeType {
        CodeType::Ordinary
    }
}
```

**Key Differences from Python/C++:**
- Python: Uses duck typing (no explicit trait declaration needed)
- C++: Uses inheritance with virtual functions
- Rust: Must explicitly implement trait - compiler enforces correctness at compile time

### 2. **Generic Functions (Similar to C++ Templates)**

```rust
// Generic function - works with any type implementing CodeScheme
fn setup_encoder<T: CodeScheme>(code: T) -> Encoder {
    let params = code.get_params();
    Encoder::new(code)
}

// Usage
setup_encoder(RaptorQLTCode::new_with_default_setting(100));
setup_encoder(RaptorQSysCode::new_with_default_setting(100));
```

### 3. **Builder Pattern**

```rust
// Chain method calls to build objects step by step
let config = RaptorQConfig::custom(100)
    .with_hdpc_type(HDPCType::Random)
    .with_degree_set(DegreeSetConfig::RandomRobust { c: 0.1, delta: 0.5 })
    .build();

// Similar to:
// In Python:
// builder = ConfigBuilder(100)
// builder.set_hdpc_type(HDPCType.Random)
// config = builder.build()
```

### 4. **Option Type (Replaces null/None)**

```rust
// In Python: returns None or a value
// In Rust: returns Option<T> - either Some(value) or None

pub fn get_params(k: usize) -> Option<RaptorQParams> {
    // Returns Some(params) or None
}

// Pattern matching - safe handling
if let Some(params) = get_params(100) {
    println!("Found params: {:?}", params);
} else {
    println!("Parameters not found");
}

// Compared to Python:
// params = get_params(100)
// if params is not None:
//     print(params)
```

### 5. **Ownership and References**

```rust
// Ownership: value is owned by one variable
let code = RaptorQLTCode::new_with_default_setting(100);

// Borrowing: temporary reference without taking ownership
fn analyze_code(code: &RaptorQLTCode) {
    let params = code.get_params();
}
analyze_code(&code);  // code is still valid

// Compare to Python:
// In Python, all values are reference-counted
// In Rust, this is explicit in the type system
```

### 6. **Clone vs Copy**

```rust
// Copy: simple types automatically copied
let a = 5;
let b = a;  // a is copied, both are valid

// Clone: complex types must be explicitly cloned
let params1 = code.get_params();
let params2 = params1.clone();  // explicit clone
```

---

## Usage Examples

### Example 1: Basic Configuration

```rust
use raptor_q::RaptorQConfig;

// Create a standard RaptorQ configuration for 100 source symbols
let config = RaptorQConfig::new(100);

// Display parameters
println!("Total symbols: {}", config.params.k);
println!("LDPC symbols: {}", config.params.l);
println!("HDPC symbols: {}", config.params.h);
```

### Example 2: Systematic Encoding

```rust
use raptor_q::RaptorQConfig;
use fountain_engine::encoder::Encoder;
use fountain_engine::decoder::Decoder;

// Create systematic configuration
let config = RaptorQConfig::systematic(100);

// Create encoder and decoder
let encoder = Encoder::new(config.clone());
let decoder = Decoder::new(config);

// First 100 encoded symbols are the original source symbols
// Additional symbols provide redundancy for erasure recovery
```

### Example 3: Custom Configuration

```rust
use raptor_q::RaptorQConfig;
use fountain_scheme::degree_sets::DegreeSetConfig;
use fountain_scheme::precodes::HDPCType;
use fountain_engine::types::CodeType;

let config = RaptorQConfig::custom(100)
    .with_hdpc_type(HDPCType::Random)
    .with_degree_set(DegreeSetConfig::RandomRobust { 
        c: 0.1,      // distribution parameter
        delta: 0.5   // robustness parameter
    })
    .with_code_type(CodeType::Systematic)
    .build();
```

### Example 4: Comparing Different k Values

```rust
use raptor_q::get_params;

// See how parameters scale with different source symbol counts
for k in [10, 50, 100, 200, 500] {
    if let Some(params) = get_params(k) {
        let overhead = (params.l + params.h) as f64 / k as f64 * 100.0;
        println!("k={:3}: K'={:3}, overhead={:.1}%", k, params.k_prime, overhead);
    }
}
```

---

## RFC 6330 Parameters Explained

The `raptor_q_para.csv` file contains pre-computed parameters from RFC 6330:

| Field | Meaning |
|-------|---------|
| K' (k_prime) | Extended source symbols - used internally in fountain code |
| S (l) | LDPC symbols count - Low-Density Parity Check symbols |
| H (h) | HDPC symbols count - High-Density Parity Check symbols |
| 蟺 (pi) | Pre-inactive symbols - symbols that cannot be decoded initially |
| J (j) | Systematic index - position of systematic part in encoding |
| W | Intermediate calculation value |

**Example:**
```
k=100 鈫?k_prime=110, S=11, H=12
```

This means for 100 source symbols:
- Extended to 110 symbols internally (K')
- Add 11 LDPC symbols
- Add 12 HDPC symbols
- Total: 110 + 11 + 12 = 133 intermediate symbols

---

## Performance Characteristics

| Aspect | Description |
|--------|------------|
| **Encoding Complexity** | O(k路d) where d is average degree (usually ~6-7) |
| **Decoding Complexity** | O(k虏) with belief propagation optimization |
| **Overhead** | Typically 5-15% extra symbols needed for reliable recovery |
| **Streaming** | Systematic codes better for streaming (immediate decoding) |
| **Reliability** | Near-optimal recovery probability (close to theoretical limit) |

---

## Integration with Fountain Engine

The `RaptorQConfig` produces `CodeConfig` objects compatible with `fountain_engine`:

```rust
use raptor_q::RaptorQConfig;
use fountain_engine::encoder::Encoder;
use fountain_engine::decoder::Decoder;

let code_config = RaptorQConfig::new(100);

// Encoder takes CodeConfig and implements encoding
let encoder = Encoder::new(code_config);

// Decoder takes CodeConfig and implements decoding  
let decoder = Decoder::new(code_config);
```

---

## Testing

Run tests for RaptorQ library:

```bash
# Test RaptorQ module only
cargo test -p raptor_q

# Test specific test file
cargo test -p raptor_q --test config_test

# Run with output
cargo test -p raptor_q -- --nocapture
```

**Test Coverage:**
- Parameter lookup from CSV table
- Configuration creation and customization
- Code scheme trait implementations
- Systematic vs non-systematic encoding
- HDPC/LDPC component generation

---

## References

- **RFC 6330**: RaptorQ Forward Error Correction Scheme for Object Delivery
  - https://tools.ietf.org/html/rfc6330
- **Fountain Codes**: Class of erasure codes with optimal performance
- **LDPC**: Low-Density Parity-Check codes (efficient error correction)
- **HDPC**: High-Density Parity-Check codes (auxiliary error correction)

---

## License

Copyright (c) 2025 Shenghao Yang. All rights reserved.  
Licensed under the MIT License.

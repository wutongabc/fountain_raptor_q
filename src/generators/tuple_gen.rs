use super::degree_gen::degree_gen;
use super::rand_num_gen::rand;
/// Tuple Generator for RaptorQ (RFC 6330 Section 5.3.5.4)
///
/// # Overview
///
/// The tuple generator creates a 6-tuple (d, a, b, d1, a1, b1) that determines
/// which source symbols are combined to create each encoded symbol.
///
/// # Parameters
///
/// - `d`, `a`, `b`: Used for LT encoding (connecting to intermediate symbols)
/// - `d1`, `a1`, `b1`: Used for LDPC/HDPC precode connections
///
/// # Algorithm (from RFC 6330)
///
/// Given inputs K' (extended source symbols) and X (Encoding Symbol ID - ESI):
///
/// 1. Calculate intermediate values:
///    - A = 53591 + J*997, if (A % 2 == 0) then A = A + 1
///    - B = 10267*(J+1)
///    - y = (B + X*A) % 2^32
///
/// 2. Generate tuple elements:
///    - v = Rand[y, 0, 2^20]
///    - d = Deg[v]
///    - a = 1 + Rand[y, 1, W-1]
///    - b = Rand[y, 2, W]
///    - d1 = if (d < 4) { 2 + Rand[X, 3, 2] } else { 2 }
///    - a1 = 1 + Rand[X, 4, P1-1]
///    - b1 = Rand[X, 5, P1]
///
/// # Usage Example
///
/// ```ignore
/// use raptor_q::generators::tuple_gen;
///
/// let k = 100;  // source symbols
/// let x = 5;    // encoding symbol ID
///
/// let (d, a, b, d1, a1, b1) = tuple_gen::generate_tuple(x, k);
/// println!("Tuple: d={}, a={}, b={}, d1={}, a1={}, b1={}", d, a, b, d1, a1, b1);
use crate::params_table;

/// Generate a tuple (d, a, b, d1, a1, b1) for a given encoding symbol
///
/// # Arguments
///
/// * `x` - Encoding Symbol ID (ESI), also called ISI in the RFC
/// * `k` - Original number of source symbols (used to look up parameters)
///
/// # Returns
///
/// A 6-tuple `(d, a, b, d1, a1, b1)` where:
/// - `d`: Degree for LT encoding
/// - `a`, `b`: LT encoding parameters
/// - `d1`: LDPC degree (always 2 or varies based on d)
/// - `a1`, `b1`: LDPC encoding parameters
///
/// # Panics
///
/// Panics if parameters for k cannot be found in the RaptorQ parameters table
pub fn generate_tuple(x: usize, k: usize) -> (usize, usize, usize, usize, usize, usize) {
    // Get L (total intermediate symbols) and J (systematic index) from parameters table
    let params = params_table::get_params(k)
        .unwrap_or_else(|| panic!("RaptorQ parameters not found for k={}", k));

    let l = params.total_l; // Total intermediate symbols (K' + S + H)
    let j = params.j; // Systematic index
    let k_prime = params.k_prime; // Extended source symbols

    // Calculate x_adjusted based on whether x is a repair symbol or source symbol
    // RFC 6330: ESI 0 to K'-1 are source symbols, ESI K' onwards are repair symbols
    let x_adjusted = if x >= k_prime {
        // Adjust x by subtracting h and l (LDPC symbols)
        // In our standard, repair symbols are indexed from k'+h+l onwards.
        // But in RFC 6330, repair symbols are indexed from k' onwards.
        let h = params.h; // HDPC symbols
        let ldpc_l = params.l; // LDPC symbols (S)

        // Check if x is in the valid repair symbol range
        if x < k_prime + h + ldpc_l {
            panic!(
                "Invalid encoding symbol ID: x={} is in the gap [k', k'+h+l). Valid ranges: [0, k') for source symbols, or [k'+h+l, ...) for repair symbols. (k'={}, h={}, l={})",
                x, k_prime, h, ldpc_l
            );
        }

        x.saturating_sub(h + ldpc_l) // Return the adjusted value
    } else {
        x // For source symbols (0 to k'-1), use x directly
    };

    // Get W parameter
    let w = params_table::find_w_for_k(k).expect("Failed to get W parameter for k");

    // P1 is the smallest prime >= P, where P = L - W.
    let p = l - w;
    let p1 = find_smallest_prime_gte(p);

    // Step 1: Calculate A
    let mut a_val = 53591 + j * 997;
    if a_val % 2 == 0 {
        a_val += 1;
    }

    // Step 2: Calculate B
    let b_val = 10267 * (j + 1);

    // Step 3: Calculate y = (B + X*A) % 2^32
    // Use wrapping arithmetic to handle overflow (mod 2^32)
    // Use x_adjusted instead of original x
    let x_u32 = x_adjusted as u32;
    let a_val_u32 = a_val as u32;
    let b_val_u32 = b_val as u32;

    let y = b_val_u32.wrapping_add(x_u32.wrapping_mul(a_val_u32));

    // Step 4: Generate tuple elements

    // v = Rand[y, 0, 2^20]
    let v = rand(y, 0, 1 << 20) as usize;

    // d = Deg[v]
    let d = degree_gen(v, k);

    // a = 1 + Rand[y, 1, W-1]
    let a = 1 + rand(y, 1, (w - 1) as u32) as usize;

    // b = Rand[y, 2, W]
    let b = rand(y, 2, w as u32) as usize;

    // d1 = if (d < 4) { 2 + Rand[X, 3, 2] } else { 2 }
    let d1 = if d < 4 {
        2 + rand(x_adjusted as u32, 3, 2) as usize
    } else {
        2
    };

    // a1 = 1 + Rand[X, 4, P1-1]
    let a1 = 1 + rand(x_adjusted as u32, 4, (p1 - 1) as u32) as usize;

    // b1 = Rand[X, 5, P1]
    let b1 = rand(x_adjusted as u32, 5, p1 as u32) as usize;

    (d, a, b, d1, a1, b1)
}

/// Find the smallest prime number greater than or equal to n
///
/// # Algorithm
///
/// Uses a simple trial division to check primality.
/// This is sufficient for the relatively small values of L in RaptorQ.
///
/// # Arguments
///
/// * `n` - The lower bound
///
/// # Returns
///
/// The smallest prime >= n
///
/// # Rust Concept: Helper Functions
///
/// In Rust, helper functions are often defined as private (not marked `pub`)
/// within the same module. This keeps the implementation details hidden.
///
/// Python equivalent:
/// ```python
/// def _find_smallest_prime_gte(n):  # Leading _ suggests private
///     # Implementation
///     pass
/// ```
fn find_smallest_prime_gte(n: usize) -> usize {
    let mut candidate = n;
    loop {
        if is_prime(candidate) {
            return candidate;
        }
        candidate += 1;
    }
}

/// Check if a number is prime
///
/// # Algorithm
///
/// Uses trial division up to sqrt(n).
///
/// # Arguments
///
/// * `n` - The number to check
///
/// # Returns
///
/// `true` if n is prime, `false` otherwise
///
/// # Rust Concept: Early Returns
///
/// The `return` keyword allows early exit from a function.
/// The last expression in a Rust function is automatically returned (no `return` needed).
///
/// ```rust
/// fn max(a: i32, b: i32) -> i32 {
///     if a > b {
///         return a;  // Early return
///     }
///     b  // Implicit return (no semicolon)
/// }
/// ```
fn is_prime(n: usize) -> bool {
    if n < 2 {
        return false;
    }
    if n == 2 {
        return true;
    }
    if n % 2 == 0 {
        return false;
    }

    let limit = (n as f64).sqrt() as usize;
    for i in (3..=limit).step_by(2) {
        if n % i == 0 {
            return false;
        }
    }

    true
}

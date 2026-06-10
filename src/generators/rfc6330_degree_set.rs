/// RFC 6330 Degree Set Generator
///
/// ！only generate the repair symbol degree set
/// # Overview
///
/// This module implements the RFC 6330 Encoding Symbol Generator (Enc[]) algorithm
/// for generating degree sets. Unlike random-based degree set generators, this
/// implementation follows the exact deterministic algorithm specified in RFC 6330
/// Section 5.3.5.3.
///
/// # Algorithm Split (Two Parts)
///
/// The original RFC 6330 Enc[] algorithm combines index generation and data XOR operations.
/// We split it into two parts:
///
/// **Part 1 (this module)**: Generate degree and index set
/// - Input: coded_id (Encoding Symbol ID)
/// - Output: Vec<usize> of intermediate symbol indices
/// - Logic: Uses tuple (d, a, b, d1, a1, b1) to compute which symbols to XOR
///
/// **Part 2 (in Encoder)**: Perform XOR operations
/// - Input: indices from Part 1
/// - Output: encoded symbol (XOR of selected intermediate symbols)
///
/// # RFC 6330 Algorithm (Section 5.3.5.3)
///
/// Given tuple (d, a, b, d1, a1, b1) from Tuple[K', X]:
///
/// ```text
/// result = C[b]
/// For j = 1, ..., d-1 do
///     b = (b + a) % W
///     result = result + C[b]
/// While (b1 >= P) do b1 = (b1+a1) % P1
/// result = result + C[W+b1]
/// For j = 1, ..., d1-1 do
///     b1 = (b1 + a1) % P1
///     While (b1 >= P) do b1 = (b1+a1) % P1
///     result = result + C[W+b1]
/// Return result
/// ```
///
/// We extract the index computation part:
/// - LT degree part: generate d indices starting from b
/// - PI degree part: generate d1 indices starting from b1, constrained by P
///
/// # Parameters
///
/// - W: Number of LT symbols (from RFC 6330 parameter table)
/// - P: Number of active PI symbols = L - W (where L = K' + S + H)
/// - P1: Smallest prime >= P
///
///
/// ## 3. Module Arithmetic for Index Generation
///
/// ```rust,ignore
/// b_current = (b_current + a) % W;
/// ```
///
/// This generates a pseudo-random sequence of indices in range [0, W):
/// - Ensures indices wrap around
/// - Creates deterministic but well-distributed connections
/// - Same seed (b, a) always produces same sequence
///
/// **Example:**
/// ```text
/// W = 10, a = 3, b = 2
/// Sequence: 2 -> 5 -> 8 -> 1 -> 4 -> 7 -> 0 -> 3 -> 6 -> 9 -> 2 ...
/// ```
///
/// ## 4. While Loop for Constraint Satisfaction
///
/// ```rust,ignore
/// while b1_current >= p {
///     b1_current = (b1_current + a1) % p1;
/// }
/// ```
///
/// RFC 6330 requires PI indices to be in range [0, P), not [0, P1):
/// - P1 is a prime number >= P (for mathematical properties)
/// - P is the actual number of PI symbols (P = L - W)
/// - This loop skips invalid indices by advancing b1 until valid
///
/// **Why this design?**
/// - Using prime P1 ensures good distribution via linear congruential generator
/// - But we only have P actual symbols, so must skip P <= index < P1
use super::tuple_gen::generate_tuple;
use crate::params_table;

/// RFC 6330 compliant degree set generator
///
/// This generator follows the exact algorithm specified in RFC 6330 Section 5.3.5.3
/// for generating encoding symbol indices.
///
/// # Fields
///
/// - `k`: Original number of source symbols
/// - `w`: Number of LT symbols (W parameter from RFC 6330)
/// - `p`: Number of PI symbols that are active (P = L - W)
/// - `p1`: Smallest prime >= P (for linear congruential generator)
///
/// # Example
///
/// ```ignore
/// use raptor_q::RFC6330DegreeSet;
///
/// let k = 100;
/// let mut degree_set = RFC6330DegreeSet::new(k);
///
/// // Generate degree set for encoding symbol 0
/// let (active_indices, inactive_indices) = degree_set.degree_set(0);
///
/// // active_indices contains the intermediate symbol indices to XOR
/// println!("Degree: {}", active_indices.len());
/// println!("Indices: {:?}", active_indices);
/// ```
pub struct RFC6330DegreeSet {
    k: usize,
    w: usize,
    p: usize,
    p1: usize,
}

impl RFC6330DegreeSet {
    /// Create a new RFC 6330 degree set generator
    ///
    /// # Arguments
    ///
    /// * `k` - Number of source symbols
    ///
    /// # Returns
    ///
    /// A new RFC6330DegreeSet instance configured with RFC 6330 parameters
    ///
    /// # Panics
    ///
    /// Panics if RFC 6330 parameters cannot be found for the given k value
    pub fn new(k: usize) -> Self {
        let params = params_table::get_params(k).expect("RFC 6330 parameters not found for k");

        let w = params_table::find_w_for_k(k).expect("W parameter not found for k");

        let l = params.total_l; // L = K' + S + H
        let p = l - w; // P = L - W (number of active PI symbols)
        let p1 = find_smallest_prime_gte(p);

        Self { k, w, p, p1 }
    }

    /// Generate indices according to RFC 6330 Enc[] algorithm
    ///
    /// This implements the index generation part of RFC 6330 Section 5.3.5.3
    ///
    /// # Algorithm
    ///
    /// 1. Get tuple (d, a, b, d1, a1, b1) from Tuple[K', X]
    /// 2. Generate LT part indices:
    ///    - Start with b, repeat d times
    ///    - Each iteration: b = (b + a) % W
    /// 3. Generate PI part indices:
    ///    - Start with b1, ensure b1 < P
    ///    - Add W + b1 to indices
    ///    - Repeat d1 times, skipping when b1 >= P
    ///
    /// # Arguments
    ///
    /// * `coded_id` - Encoding Symbol ID (ESI)
    ///
    /// # Returns
    ///
    /// Vector of intermediate symbol indices to XOR together
    fn generate_indices(&self, coded_id: usize) -> (Vec<usize>, Vec<usize>) {
        // Step 1: Get tuple from RFC 6330 Tuple Generator
        let (d, a, b, d1, a1, b1) = generate_tuple(coded_id, self.k);

        // check the data valid
        if d > 0
            && a >= 1
            && a <= self.w - 1
            && b >= 0
            && b <= self.w - 1
            && (d1 == 2 || d1 == 3)
            && a1 >= 1
            && a1 <= self.p1 - 1
            && b1 >= 0
            && b1 <= self.p1 - 1
        {
        } else {
            dbg!("the data of RFC6330 degree set is invalid, return empty vector");
            return (Vec::new(), Vec::new());
        }

        let mut active_indices = Vec::new();
        let mut inactive_indices = Vec::new();

        // Step 2: Generate LT degree part indices
        // RFC 6330: result = C[b]; For j=1..d-1: b=(b+a)%W, result+=C[b]
        let mut b_current = b;
        active_indices.push(b_current);
        for _ in 0..d - 1 {
            b_current = (b_current + a) % self.w;
            active_indices.push(b_current);
        }

        // Step 3: Generate PI degree part indices (RFC 6330 style)
        // These should be added to active_indices with offset W
        let mut b1_current = b1;

        // [A] Pre-process the initial b1
        while b1_current >= self.p {
            b1_current = (b1_current + a1) % self.p1;
        }

        // [B] Use the first valid index (j=0 equivalent)
        // Add W offset as per RFC 6330: result = result + C[W+b1]
        // inactive_indices.push(self.w + b1_current);
        inactive_indices.push(b1_current);

        for _ in 0..d1 - 1 {
            b1_current = (b1_current + a1) % self.p1;
            while b1_current >= self.p {
                b1_current = (b1_current + a1) % self.p1;
            }
            // Add W offset as per RFC 6330
            // inactive_indices.push(self.w + b1_current);
            inactive_indices.push(b1_current);
        }
        // println!("active_indices: {:?}", active_indices);
        // println!("inactive_indices: {:?}", inactive_indices);
        // println!("the tuple is: {:?}", (d, a, b, d1, a1, b1));
        // println!("the coded_id is: {}", coded_id);
        // println!("the number of active indices is: {}", active_indices.len());
        // println!("the number of inactive indices is: {}", inactive_indices.len());
        (active_indices, inactive_indices)
    }

    /// Generate degree set for a coded symbol.
    pub fn degree_set(&mut self, coded_id: usize) -> (Vec<usize>, Vec<usize>) {
        self.generate_indices(coded_id)
    }

    /// Get the name of this degree set generator.
    pub fn name(&self) -> &str {
        "RFC6330"
    }
}

/// Find the smallest prime number >= n
///
/// # Algorithm
///
/// Uses trial division to check primality. Sufficient for RaptorQ parameter range.
///
/// # Arguments
///
/// * `n` - Lower bound
///
/// # Returns
///
/// The smallest prime >= n
///
/// # Note
///
/// This is the same algorithm used in tuple_gen.rs. We duplicate it here to
/// keep this module self-contained. In production code, you might want to
/// extract this to a shared utility module.
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
/// Trial division up to sqrt(n)
///
/// # Arguments
///
/// * `n` - Number to check
///
/// # Returns
///
/// true if n is prime, false otherwise
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_rfc6330_degree_set_creation() {
        // Test that we can create a degree set generator for valid k values
        let k = 100;
        let degree_set = RFC6330DegreeSet::new(k);

        assert_eq!(degree_set.k, k);
        assert!(degree_set.w > 0);
        assert!(degree_set.p > 0);
        assert!(degree_set.p1 >= degree_set.p);
    }

    #[test]
    fn test_degree_set_generation() {
        // Test that degree set generation produces valid indices
        let k = 100;
        let mut degree_set = RFC6330DegreeSet::new(k);

        // Get RFC 6330 split parameters for validation
        let params = params_table::get_params(k).unwrap();
        let w = params_table::find_w_for_k(k).unwrap();
        let p = params.total_l - w;

        // Generate degree set for first encoding symbol
        let (active_indices, inactive_indices) = degree_set.degree_set(0);

        // Should have at least 1 index (degree >= 1)
        assert!(!active_indices.is_empty());

        // RFC 6330 returns LT indices and PI indices in separate local domains.
        assert!(!inactive_indices.is_empty());

        // LT indices live in [0, W)
        for &idx in &active_indices {
            assert!(idx < w, "Active index {} out of range [0, {})", idx, w);
        }

        // PI indices are local offsets in [0, P)
        for &idx in &inactive_indices {
            assert!(idx < p, "Inactive index {} out of range [0, {})", idx, p);
        }
    }

    #[test]
    fn test_deterministic_generation() {
        // Test that same coded_id produces same indices
        let k = 100;
        let mut degree_set1 = RFC6330DegreeSet::new(k);
        let mut degree_set2 = RFC6330DegreeSet::new(k);

        let coded_id = 42;
        let (indices1, _) = degree_set1.degree_set(coded_id);
        let (indices2, _) = degree_set2.degree_set(coded_id);

        assert_eq!(
            indices1, indices2,
            "Same coded_id should produce same indices"
        );
    }

    #[test]
    fn test_different_coded_ids() {
        // Test that different coded_ids produce different indices
        let k = 100;
        let mut degree_set = RFC6330DegreeSet::new(k);

        let (indices1, _) = degree_set.degree_set(0);
        let (indices2, _) = degree_set.degree_set(1);

        assert_ne!(
            indices1, indices2,
            "Different coded_ids should produce different indices"
        );
    }

    #[test]
    fn test_prime_finding() {
        assert_eq!(find_smallest_prime_gte(1), 2);
        assert_eq!(find_smallest_prime_gte(2), 2);
        assert_eq!(find_smallest_prime_gte(3), 3);
        assert_eq!(find_smallest_prime_gte(4), 5);
        assert_eq!(find_smallest_prime_gte(10), 11);
        assert_eq!(find_smallest_prime_gte(11), 11);
    }

    #[test]
    fn test_is_prime() {
        assert!(!is_prime(0));
        assert!(!is_prime(1));
        assert!(is_prime(2));
        assert!(is_prime(3));
        assert!(!is_prime(4));
        assert!(is_prime(5));
        assert!(!is_prime(6));
        assert!(is_prime(7));
        assert!(!is_prime(8));
        assert!(!is_prime(9));
        assert!(!is_prime(10));
        assert!(is_prime(11));
    }
}

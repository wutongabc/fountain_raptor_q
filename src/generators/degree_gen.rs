// Cumulative Distribution Function (CDF) table for RFC 6330 degree distribution
// This is Table 1 from RFC 6330: Soliton-like degree distribution
// Each f[d] is the cumulative probability scaled to [0, 2^20) = [0, 1048576)
//
// # How to populate DEGREE_CDF_TABLE:
// =====================================
// From RFC 6330 Section 5.3.5.2, the table f[d] (d = 0, 1, ..., n) contains
// cumulative distribution function values. The algorithm:
//
// 1. Start with f[0] = 0 (always)
// 2. For each degree d (d = 1, 2, ..., n):
//    f[d] = floor(probability[d] * 1048576)
//    where probability[d] is the cumulative probability up to degree d
//
// 3. The table should satisfy: f[0] < f[1] < f[2] < ... < f[n] = 1048576
//
// # Usage Example:
// ================
// const DEGREE_CDF_TABLE: &[u32] = &[
//     0,           // f[0] = 0
//     110427,      // f[1] 鈮?0.1053 * 2^20
//     220854,      // f[2] 鈮?0.2106 * 2^20
//     331281,      // f[3] 鈮?0.3158 * 2^20
//     // ... more values ...
//     1048576,     // f[n] = 2^20 (final value, representing 100% probability)
// ];
//
// # RFC 6330 Reference:
// The actual Table 1 from RFC 6330 should be consulted for exact values.
// These define a Soliton-like degree distribution optimized for RaptorQ encoding.
const DEGREE_CDF_TABLE: &[u32] = &[
    // TODO: Add values from RFC 6330 Table 1 here
    // Placeholder: User will populate this with actual CDF values
    0, 5243, 529531, 704294, 791675, 844104, 879057, 904023, 922747, 937311, 948962, 958494, 966438,
    973160, 978921, 983914, 988283, 992138, 995565, 998631, 1001391, 1003887, 1006157, 1008229,
    1010129, 1011876, 1013490, 1014983, 1016370, 1017662, 1048576,
];

/// Generate degree for a given random value v
///
/// # Arguments
/// * `v` - A random non-negative integer less than 2^20 = 1048576
/// * `k` - Number of source symbols (used to look up W from parameters table)
///
/// # Returns
/// The degree d = min(index, W-2) where:
/// - index is found such that f[index-1] <= v < f[index]
/// - W is the parameter from RFC 6330 lookup table
///
/// # Algorithm (RFC 6330 Section 5.3.5.2)
/// 1. Find d such that f[d-1] <= v < f[d]
/// 2. Apply constraint: return min(d, W-2)
pub fn degree_gen(v: usize, k: usize) -> usize {
    // Import the parameters table to get W
    use crate::params_table;

    // Validate input
    assert!(v < (1 << 20), "v must be less than 2^20 = 1048576");

    // Get W parameter from RFC 6330 table
    let w = params_table::find_w_for_k(k).expect("Failed to get W parameter for k");

    // Apply degree constraint: max degree is W-2
    let max_degree = if w > 2 { w - 2 } else { 1 };

    // Binary search to find degree d where f[d-1] <= v < f[d]
    let v = v as u32;
    let degree = binary_search_degree(v);

    // Return the minimum of computed degree and W-2
    std::cmp::min(degree, max_degree)
}

/// Binary search helper function
///
/// Find the index d such that f[d-1] <= v < f[d]
/// Uses the DEGREE_CDF_TABLE lookup
fn binary_search_degree(v: u32) -> usize {
    // Handle edge case: if table is empty or v is less than first entry
    if DEGREE_CDF_TABLE.is_empty() || v < DEGREE_CDF_TABLE[0] {
        return 1; // minimum degree is 1
    }

    // Binary search in the CDF table
    // Find the first index d where f[d] > v
    match DEGREE_CDF_TABLE.binary_search(&v) {
        Ok(idx) => {
            // Exact match: v == f[idx]
            // Return idx+1 to move to next degree
            idx + 1
        }
        Err(idx) => {
            // v falls between f[idx-1] and f[idx]
            // Return idx to indicate degree is idx
            if idx == 0 { 1 } else { idx }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn rfc6330_degree_cdf_table_matches_table_1_boundary() {
        assert_eq!(DEGREE_CDF_TABLE[8], 922747);
        assert_eq!(DEGREE_CDF_TABLE[9], 937311);
        assert_eq!(DEGREE_CDF_TABLE[10], 948962);
    }

    #[test]
    fn degree_boundary_around_rfc_table_index_9() {
        assert_eq!(binary_search_degree(937110), 9);
        assert_eq!(binary_search_degree(937111), 9);
        assert_eq!(binary_search_degree(937310), 9);
        assert_eq!(binary_search_degree(937311), 10);
    }
}

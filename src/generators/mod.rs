// Degree Generator Module
//
// This module implements the RFC 6330 degree generation algorithm.
//
// # Overview
//
// The degree generator is a critical component of RaptorQ encoding that determines
// how many source symbols each encoded symbol should be connected to.
//
// # Key Concept
//
// For each encoded symbol, the encoder needs to know its degree (number of source symbols
// it's connected to). The degree is sampled from a pre-defined distribution using:
//
// ```text
// Deg[v] = min(d, W-2)
// ```
//
// Where:
// - `v` is a pseudo-random number (0 ≤ v < 2^20)
// - `d` is found by binary search in Table 1 such that f[d-1] ≤ v < f[d]
// - `W` is the number of intermediate symbols (from RFC 6330 parameters)
// - `f[d]` is the cumulative distribution function value scaled to [0, 2^20)
//
// # Usage Example
//
// ```ignore
// use raptor_q::generators::degree_gen;
//
// // Generate degree for encoded symbol with ID 5
// let k = 100; // source symbols
// let v = 12345; // pseudo-random value (must be < 2^20)
// let degree = degree_gen::degree_gen(v, k);
// println("Generated degree: {}", degree);
// ```
//
// # Rust Concepts for Beginners
//
// ## Binary Search (used in this module)
//
// The `binary_search` function efficiently finds where a value fits in a sorted array:
//
// ```rust
// // Example: Finding a value in a sorted array
// let sorted_array = vec[0, 100, 200, 300, 400];
// let search_value = 250;
//
// // Binary search returns Err(index) if not found
// // where index is where it WOULD be inserted
// match sorted_array.binary_search(&search_value) {
//     Ok(idx) => println("Found at index: {}", idx),
//     Err(idx) => println("Would be inserted at index: {}", idx),
//     // In this case: Err(3) because 250 would go at position 3
// }
// ```
//
// ## Comparison to Python
//
// Python equivalent:
// ```python
// import bisect
// sorted_array = [0, 100, 200, 300, 400]
// search_value = 250
// idx = bisect.bisect_left(sorted_array, search_value)
// # idx = 3
// ```
//
// # RFC 6330 Reference
//
// See RFC 6330 Section 5.3.5.2 for the complete algorithm specification.

pub mod degree_gen;
pub mod rand_num_gen;
pub mod rfc6330_degree_set;
pub mod tuple_gen;

pub use degree_gen::*;
pub use rand_num_gen::*;
pub use rfc6330_degree_set::RFC6330DegreeSet;
pub use tuple_gen::*;

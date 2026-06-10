use fountain_raptor_q::generators::tuple_gen::generate_tuple;
/// Tests for the Tuple Generator (RFC 6330 Section 5.3.5.4)
///
/// This test file verifies the correct implementation of the tuple generator
/// and its helper functions.
use fountain_raptor_q::params_table;

#[test]
fn test_generate_tuple() {
    // Test with k=100 (a typical value)
    let k = 100;

    // Generate tuple for ESI = 0
    let (d, a, b, d1, a1, b1) = generate_tuple(0, k);

    // Basic sanity checks
    assert!(d > 0, "Degree d should be positive");
    assert!(a > 0, "Parameter a should be positive");
    assert!(d1 >= 2, "LDPC degree d1 should be at least 2");
    assert!(a1 > 0, "LDPC parameter a1 should be positive");

    println!(
        "Tuple for k={}, X=0: d={}, a={}, b={}, d1={}, a1={}, b1={}",
        k, d, a, b, d1, a1, b1
    );
}

#[test]
fn test_generate_multiple_tuples() {
    // Generate several tuples and verify they're different
    let k = 100;

    let tuple0 = generate_tuple(0, k);
    let tuple1 = generate_tuple(1, k);
    let tuple2 = generate_tuple(2, k);

    // Tuples for different ESIs should generally be different
    assert_ne!(tuple0, tuple1, "Tuples for different ESIs should differ");
    assert_ne!(tuple1, tuple2, "Tuples for different ESIs should differ");

    println!("Tuple 0: {:?}", tuple0);
    println!("Tuple 1: {:?}", tuple1);
    println!("Tuple 2: {:?}", tuple2);
}

#[test]
fn test_tuple_deterministic() {
    // Verify that the tuple generator is deterministic
    // Same inputs should always produce same outputs
    let k = 50;
    let x = 42;

    let tuple1 = generate_tuple(x, k);
    let tuple2 = generate_tuple(x, k);

    assert_eq!(tuple1, tuple2, "Tuple generator should be deterministic");
}

#[test]
fn test_tuple_with_different_k_values() {
    // Test with various k values to ensure robustness
    let test_k_values = vec![10, 50, 100, 500, 1000];

    for k in test_k_values {
        let (d, a, b, d1, a1, b1) = generate_tuple(0, k);

        // Verify basic constraints
        assert!(d > 0, "Degree d should be positive for k={}", k);
        assert!(a > 0, "Parameter a should be positive for k={}", k);
        assert!(
            d1 >= 2 && d1 <= 4,
            "LDPC degree d1 should be 2, 3, or 4 for k={}",
            k
        );
        assert!(a1 > 0, "LDPC parameter a1 should be positive for k={}", k);
    }
}

#[test]
fn test_d1_value_based_on_degree() {
    // Test that d1 is correctly computed based on d
    let k = 100;
    let params = params_table::get_params(k).unwrap();

    // Generate many tuples to find cases where d < 4 and d >= 4
    let mut found_small_degree = false;
    let mut found_large_degree = false;

    for x in 0..1000 {
        let (d, _a, _b, d1, _a1, _b1) = generate_tuple(x, k);

        if d < 4 {
            // When d < 4, d1 should be 2, 3, or 4 (2 + Rand[X, 3, 2])
            assert!(d1 >= 2 && d1 <= 4, "d1 should be in range [2,4] when d < 4");
            found_small_degree = true;
        } else {
            // When d >= 4, d1 should always be 2
            assert_eq!(d1, 2, "d1 should be 2 when d >= 4");
            found_large_degree = true;
        }

        if found_small_degree && found_large_degree {
            break;
        }
    }

    // We should have found at least one case of each
    assert!(found_large_degree, "Should find cases where d >= 4");
}

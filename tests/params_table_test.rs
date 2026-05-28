use fountain_raptor_q::{get_inactive_number, get_k_prime, get_params, get_systematic_index};

#[test]
fn test_get_params() {
    let params = get_params(10).unwrap();
    assert_eq!(params.k_prime, 10);
    assert_eq!(params.l, 7);
    assert_eq!(params.h, 10);
    assert_eq!(params.j, 254);

    // Test that it finds the smallest K' >= k
    let params = get_params(15).unwrap();
    assert_eq!(params.k_prime, 18);
}

#[test]
fn test_get_inactive_number() {
    let pi = get_inactive_number(10).unwrap();
    // PI = K' + S + H - W = 10 + 7 + 10 - 17 = 10
    assert_eq!(pi, 10);
}

#[test]
fn test_get_k_prime() {
    assert_eq!(get_k_prime(10).unwrap(), 10);
    assert_eq!(get_k_prime(15).unwrap(), 18);
    assert_eq!(get_k_prime(100).unwrap(), 101);
}

#[test]
fn test_get_systematic_index() {
    assert_eq!(get_systematic_index(10).unwrap(), 254);
    assert_eq!(get_systematic_index(15).unwrap(), 682);
}

#[test]
fn test_edge_cases() {
    // Very small k
    let params = get_params(1).unwrap();
    assert_eq!(params.k_prime, 10); // Should return the first entry

    // Large k
    let params = get_params(1000).unwrap();
    assert!(params.k_prime >= 1000);
}

#[test]
fn test_params_consistency() {
    let params = get_params(50).unwrap();

    // Verify total_l = K' + L + H
    assert_eq!(params.total_l, params.k_prime + params.l + params.h);

    // Verify PI is less than total_l
    assert!(params.pi < params.total_l);
}

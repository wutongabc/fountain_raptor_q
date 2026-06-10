//! RaptorQ public API compatibility tests.
//!
//! These tests intentionally stay at the API boundary. Full round-trip coverage
//! for larger historical failure cases lives in targeted examples so the unit
//! suite stays quick.

use fountain_engine::{CodeScheme, CodeType};
use fountain_raptor_q::{
    LDPCType, RFC6330_GF256_PRIMITIVE_POLYNOMIAL, raptor_q_main::raptor_q_main,
};

fn default_code(k: usize) -> raptor_q_main {
    raptor_q_main::new(k, 30.min(k), LDPCType::ReversedLDPC)
}

#[test]
fn test_raptor_q_api_creation() {
    let k = 50;
    let code = default_code(k);
    let params = code.get_params();

    assert!(params.k >= k);
    assert!(params.l > 0);
    assert!(params.h > 0);
    assert_eq!(code.code_type(), CodeType::Systematic);
}

#[test]
fn test_degree_set_adapter_returns_flat_public_indices() {
    let code = default_code(100);
    let params = code.get_params();
    let mut degree_set_fn = code.create_degree_set_fn();

    let indices = degree_set_fn(params.num_total());

    assert!(!indices.is_empty());
    for index in indices {
        assert!(
            index < params.num_total(),
            "degree set index {} out of public variable range [0, {})",
            index,
            params.num_total()
        );
    }
}

#[test]
fn test_precode_api_with_reversed_ldpc() {
    let code = default_code(80);
    let (hdpc, ldpc) = code.create_precode();

    assert!(hdpc.is_some());
    assert!(ldpc.is_some());
}

#[test]
fn test_precode_api_with_rq_ldpc() {
    let code = raptor_q_main::new(80, 30, LDPCType::RQLDPC);
    let (hdpc, ldpc) = code.create_precode();

    assert!(hdpc.is_some());
    assert!(ldpc.is_some());
}

#[test]
fn test_decoding_config_uses_current_public_api() {
    let code = default_code(120);
    let params = code.get_params();
    let config = code.decoding_config();

    assert!(config.max_inactive_num >= params.num_pre_inactive());
}

#[test]
fn test_rfc_field_configuration_reaches_engine_managers() {
    let code = raptor_q_main::new_with_default_setting(149);
    let encoder = code.new_encoder();
    let decoder = code.new_decoder();

    assert_eq!(
        encoder.manager.gf256().unwrap().primitive_polynomial(),
        RFC6330_GF256_PRIMITIVE_POLYNOMIAL
    );
    assert_eq!(
        decoder.manager.gf256().unwrap().primitive_polynomial(),
        RFC6330_GF256_PRIMITIVE_POLYNOMIAL
    );
}

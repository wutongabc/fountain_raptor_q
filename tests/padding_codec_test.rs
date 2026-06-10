//! Round-trip tests for [`RaptorQEncoder`] / [`RaptorQDecoder`] padding wrappers.

use fountain_engine::traits::DataOperator;
use fountain_engine::types::{CodeType, DecodeStatus};
use fountain_engine::CodeScheme;
use fountain_utility::VecDataOperater;
use fountain_raptor_q::{LDPCType, RaptorQDecoder, RaptorQEncoder, raptor_q_main::raptor_q_main};

#[test]
fn padded_round_trip_k_smaller_than_k_prime() {
    let source_k = 11;
    let t = 16;
    let code = raptor_q_main::new(source_k, 30, LDPCType::RQLDPC);
    assert!(code.num_padding() > 0, "test needs K < K′ from RFC table");

    let mut messages = vec![vec![0u8; t]; source_k];
    for (i, row) in messages.iter_mut().enumerate() {
        for (j, byte) in row.iter_mut().enumerate() {
            *byte = ((i * 3 + j * 5) % 251) as u8;
        }
    }

    let mut enc_op = VecDataOperater::new(t);
    for (i, v) in messages.iter().enumerate() {
        enc_op.insert_vector(v, i);
    }

    let mut encoder = RaptorQEncoder::new_with_operator(code.clone(), Box::new(enc_op), t);
    let params = code.get_params();

    let mut coded_ids: Vec<usize> = (0..source_k).collect();
    coded_ids.extend(params.num_total()..params.num_total() + source_k + source_k / 2);

    let mut coded_payload = Vec::new();
    for &coded_id in &coded_ids {
        if let Some(data_id) = encoder.encode_coded_vector(coded_id) {
            coded_payload.push((coded_id, encoder.get_data_vector(data_id).to_vec()));
        }
    }

    let enc_op = encoder.inner.manager.move_operator();
    let dec_op = VecDataOperater::new(t);
    let mut decoder = RaptorQDecoder::new_with_operator(code, Box::new(dec_op), t);

    let mut decoded = false;
    for (coded_id, payload) in coded_payload {
        if decoder.add_coded_vector(coded_id, &payload) == DecodeStatus::Decoded {
            decoded = true;
            break;
        }
    }
    assert!(decoded, "decoder should succeed with padding + LT symbols");

    let dec_op = decoder.inner.manager.move_operator();
    for (i, expected) in messages.iter().enumerate() {
        assert_eq!(dec_op.get_vector(i), expected.as_slice(), "message {i}");
    }
}

#[test]
fn encoder_new_without_external_operator_when_k_less_than_k_prime() {
    let code = raptor_q_main::new(11, 30, LDPCType::RQLDPC);
    assert!(code.num_padding() > 0);
    let enc = RaptorQEncoder::new(code);
    assert_eq!(enc.num_padding(), 1);
}

#[test]
fn unpadded_encoder_matches_engine_when_k_equals_k_prime() {
    let k = 10;
    let code = raptor_q_main::new(k, 30, LDPCType::RQLDPC);
    assert_eq!(code.num_padding(), 0);

    let wrapped = RaptorQEncoder::new(code.clone());
    assert_eq!(wrapped.block_symbols(), code.block_symbols());
    assert_eq!(code.code_type(), CodeType::Systematic);
}

//! End-to-end coverage for ordinary (non-systematic) RaptorQ encoding.

use fountain_engine::{CodeScheme, CodeType, DataOperator, DecodeStatus};
use fountain_raptor_q::raptor_q_main::raptor_q_main;
use fountain_utility::VecDataOperater;

const K: usize = 10;
const SYMBOL_SIZE: usize = 8;

fn source_vectors() -> Vec<Vec<u8>> {
    (0..K)
        .map(|source_id| {
            (0..SYMBOL_SIZE)
                .map(|byte_id| ((source_id * 17 + byte_id * 11) % 251) as u8)
                .collect()
        })
        .collect()
}

fn source_operator(source: &[Vec<u8>]) -> VecDataOperater {
    let mut operator = VecDataOperater::new(SYMBOL_SIZE);
    for (source_id, vector) in source.iter().enumerate() {
        operator.insert_vector(vector, source_id);
    }
    operator
}

#[test]
fn ordinary_packet_ids_start_after_all_precode_symbols() {
    let source = source_vectors();
    let code = raptor_q_main::new_with_default_setting(K).with_code_type(CodeType::Ordinary);
    let params = code.get_params();
    assert_eq!(params.k, K, "this boundary test requires K = K-prime");

    let mut encoder = code.new_encoder_with_operator(Box::new(source_operator(source.as_slice())));

    assert!(
        encoder.encode_coded_vector(0).is_none(),
        "ordinary encoding must not expose source ESI 0"
    );
    assert!(
        encoder.encode_coded_vector(params.k).is_none(),
        "precode constraint IDs must not be sent as ordinary packets"
    );
    assert!(
        encoder
            .encode_coded_vector(params.num_total() - 1)
            .is_none(),
        "the last precode constraint ID must still be rejected"
    );
    assert!(
        encoder.encode_coded_vector(params.num_total()).is_some(),
        "the first ordinary LT packet must start at params.num_total()"
    );
}

#[test]
fn ordinary_round_trip_uses_only_lt_packet_ids() {
    let source = source_vectors();
    let code = raptor_q_main::new_with_default_setting(K).with_code_type(CodeType::Ordinary);
    let params = code.get_params();
    let first_packet_id = params.num_total();
    let packet_ids = first_packet_id..first_packet_id + 2 * params.k;

    let mut encoder = code.new_encoder_with_operator(Box::new(source_operator(source.as_slice())));
    let mut packets = Vec::new();
    for packet_id in packet_ids {
        let data_id = encoder
            .encode_coded_vector(packet_id)
            .unwrap_or_else(|| panic!("ordinary ESI {packet_id} should be encodable"));
        packets.push((packet_id, encoder.get_data_vector(data_id).to_vec()));
    }

    assert!(
        packets
            .iter()
            .all(|(packet_id, _)| *packet_id >= first_packet_id),
        "ordinary packets must not use source or precode-constraint IDs"
    );

    let mut decoder = code.new_decoder_with_operator(Box::new(VecDataOperater::new(SYMBOL_SIZE)));
    let mut decoded = false;
    for (packet_id, payload) in packets {
        if decoder.add_coded_vector(packet_id, payload.as_slice()) == DecodeStatus::Decoded {
            decoded = true;
            break;
        }
    }

    assert!(decoded, "ordinary decoder did not recover K source symbols");
    for (source_id, expected) in source.iter().enumerate() {
        assert_eq!(
            decoder.manager.get_data_vector(source_id),
            expected.as_slice(),
            "ordinary round trip changed source symbol {source_id}"
        );
    }
}

#[test]
fn ordinary_round_trip_representative_k_values() {
    for source_k in [10usize, 11, 50, 149, 1_000] {
        let code = raptor_q_main::new_with_default_setting(source_k)
            .with_code_type(CodeType::Ordinary);
        let params = code.get_params();
        let block_k = params.k;
        let messages: Vec<Vec<u8>> = (0..block_k)
            .map(|source_id| {
                (0..SYMBOL_SIZE)
                    .map(|byte_id| ((source_id * 19 + byte_id * 23) % 251) as u8)
                    .collect()
            })
            .collect();

        let mut operator = VecDataOperater::new(SYMBOL_SIZE);
        for (source_id, message) in messages.iter().enumerate() {
            operator.insert_vector(message, source_id);
        }

        let first_ordinary_esi = params.num_total();
        let mut encoder = code.new_encoder_with_operator(Box::new(operator));
        let mut packets = Vec::new();
        for esi in first_ordinary_esi..first_ordinary_esi + 2 * block_k {
            let data_id = encoder
                .encode_coded_vector(esi)
                .unwrap_or_else(|| panic!("ordinary ESI {esi} should encode for K={source_k}"));
            packets.push((esi, encoder.get_data_vector(data_id).to_vec()));
        }

        let mut decoder =
            code.new_decoder_with_operator(Box::new(VecDataOperater::new(SYMBOL_SIZE)));
        for (esi, payload) in packets {
            if decoder.add_coded_vector(esi, &payload) == DecodeStatus::Decoded {
                break;
            }
        }

        assert_eq!(
            decoder.decode_status(),
            DecodeStatus::Decoded,
            "ordinary decode failed for K={source_k}, K′={block_k}"
        );
        for (source_id, expected) in messages.iter().enumerate() {
            assert_eq!(
                decoder.manager.get_data_vector(source_id),
                expected.as_slice(),
                "ordinary data mismatch for K={source_k}, source={source_id}"
            );
        }
    }
}

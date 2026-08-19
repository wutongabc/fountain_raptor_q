//! Comparison of the packet-ID ranges used by systematic and ordinary RaptorQ.

use fountain_engine::{CodeScheme, CodeType, DataOperator, DecodeStatus};
use fountain_raptor_q::raptor_q_main::raptor_q_main;
use fountain_utility::VecDataOperater;

const K: usize = 10;
const SYMBOL_SIZE: usize = 4;

fn source_vectors() -> Vec<Vec<u8>> {
    (0..K)
        .map(|source_id| {
            (0..SYMBOL_SIZE)
                .map(|byte_id| ((source_id * 13 + byte_id * 7) % 251) as u8)
                .collect()
        })
        .collect()
}

fn round_trip(code_type: CodeType) -> (Vec<usize>, Vec<Vec<u8>>) {
    let source = source_vectors();
    let code = raptor_q_main::new_with_default_setting(K).with_code_type(code_type);
    let params = code.get_params();

    let mut operator = VecDataOperater::new(SYMBOL_SIZE);
    for (source_id, vector) in source.iter().enumerate() {
        operator.insert_vector(vector, source_id);
    }

    let candidate_ids: Vec<usize> = match code_type {
        CodeType::Systematic => (0..params.k).collect(),
        CodeType::Ordinary => (params.num_total()..params.num_total() + 2 * params.k).collect(),
    };

    let mut encoder = code.new_encoder_with_operator(Box::new(operator));
    let packets: Vec<(usize, Vec<u8>)> = candidate_ids
        .into_iter()
        .map(|packet_id| {
            let data_id = encoder
                .encode_coded_vector(packet_id)
                .unwrap_or_else(|| panic!("{code_type:?} ESI {packet_id} should be encodable"));
            (packet_id, encoder.get_data_vector(data_id).to_vec())
        })
        .collect();

    let mut decoder = code.new_decoder_with_operator(Box::new(VecDataOperater::new(SYMBOL_SIZE)));
    let mut used_ids = Vec::new();
    for (packet_id, payload) in packets {
        used_ids.push(packet_id);
        if decoder.add_coded_vector(packet_id, payload.as_slice()) == DecodeStatus::Decoded {
            break;
        }
    }

    assert_eq!(
        decoder.decode_status(),
        DecodeStatus::Decoded,
        "{code_type:?} decoder did not recover the source block"
    );
    let recovered = (0..params.k)
        .map(|source_id| decoder.manager.get_data_vector(source_id).to_vec())
        .collect();
    (used_ids, recovered)
}

#[test]
fn systematic_and_ordinary_use_different_packet_id_ranges() {
    let params = raptor_q_main::new_with_default_setting(K).get_params();
    let expected = source_vectors();
    let (systematic_ids, systematic_data) = round_trip(CodeType::Systematic);
    let (ordinary_ids, ordinary_data) = round_trip(CodeType::Ordinary);

    assert_eq!(systematic_ids, (0..params.k).collect::<Vec<_>>());
    assert!(
        ordinary_ids
            .iter()
            .all(|packet_id| *packet_id >= params.num_total()),
        "ordinary ESI values must start at params.num_total()"
    );
    assert_eq!(systematic_data, expected);
    assert_eq!(ordinary_data, expected);
    assert_eq!(ordinary_data, systematic_data);
}

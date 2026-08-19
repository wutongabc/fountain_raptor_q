//! Public API checks for the Fountain 2.x dependency migration.
//!
//! Integration tests compile this crate as a downstream consumer would. The
//! explicit type annotations below ensure the public API is connected to the
//! 2.x Fountain types selected by this package's dependencies.

use fountain_engine::{
    CodeScheme, DataOperator, Decoder, Encoder, HDPC, SubstitutionMethod,
};
use fountain_raptor_q::{
    RaptorQDecoder, RaptorQEncoder, raptor_q_main::raptor_q_main, rfc6330_hdpc,
};
use fountain_utility::BlockSizePolicy;

fn accepts_fountain_v2_scheme<T: CodeScheme + BlockSizePolicy>(_: &T) {}

#[test]
fn public_api_uses_fountain_v2_types() {
    let code = raptor_q_main::new_with_default_setting(10);
    accepts_fountain_v2_scheme(&code);

    let _: fn(raptor_q_main, SubstitutionMethod) -> raptor_q_main = raptor_q_main::with_subs_method;
    let _: fn(&raptor_q_main) -> Encoder = raptor_q_main::new_encoder;
    let _: fn(&raptor_q_main) -> Decoder = raptor_q_main::new_decoder;
    let _: fn(&raptor_q_main, Box<dyn DataOperator>) -> Encoder =
        raptor_q_main::new_encoder_with_operator;
    let _: fn(&raptor_q_main, Box<dyn DataOperator>) -> Decoder =
        raptor_q_main::new_decoder_with_operator;

    let hdpc: Box<dyn HDPC> = rfc6330_hdpc(code.get_params().h);
    assert_eq!(
        hdpc.gf_poly(),
        fountain_raptor_q::RFC6330_GF256_PRIMITIVE_POLYNOMIAL
    );
}

#[test]
fn public_padding_aliases_construct_with_fountain_utility_v2() {
    let code = raptor_q_main::new_with_default_setting(11);
    let encoder: RaptorQEncoder = code.clone().padded_encoder();
    let decoder: RaptorQDecoder = code.padded_decoder();

    assert_eq!(encoder.source_symbols(), 11);
    assert_eq!(decoder.source_symbols(), 11);
    assert_eq!(encoder.num_padding(), 1);
    assert_eq!(decoder.num_padding(), 1);
}

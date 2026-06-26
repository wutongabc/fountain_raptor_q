pub mod params_table;
pub mod generators;
pub mod raptor_q_main;
#[path = "RQLDPC.rs"]
pub mod rql_dpc;
#[path = "RQHDPC.rs"]
pub mod rqh_dpc;

pub use params_table::*;
pub use generators::*;
pub use raptor_q_main::*;
pub use rql_dpc::RQLDPC;
pub use rqh_dpc::RQHDPC;
pub use raptor_q_main::{rfc6330_hdpc, RFC6330_GF256_PRIMITIVE_POLYNOMIAL};

use fountain_utility::{PaddedDecoder, PaddedEncoder, PaddedRealSymbolSession};

/// Padding-aware RaptorQ encoder ([`fountain_utility::PaddedEncoder`]).
pub type RaptorQEncoder = PaddedEncoder<crate::raptor_q_main::raptor_q_main>;

/// Padding-aware RaptorQ decoder ([`fountain_utility::PaddedDecoder`]).
pub type RaptorQDecoder = PaddedDecoder<crate::raptor_q_main::raptor_q_main>;

/// [`RealSymbolSession`](fountain_utility::RealSymbolSession) for RFC 6330 padding.
pub const RaptorQRealSymbolSession: PaddedRealSymbolSession = PaddedRealSymbolSession;

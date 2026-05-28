use crate::generators::rand_num_gen::rand;
use crate::generators::RFC6330DegreeSet;
use crate::RQLDPC;
use crate::params_table;
use fountain_engine::traits::{CodeScheme, HDPC, LDPC};
use fountain_engine::types::{CodeParams, CodeType, DecodingConfig};
use fountain_engine::{Decoder, Encoder};
use fountain_engine::traits::DataOperator;
use fountain_scheme::precodes::{rq_hdpc, ReversedLDPC};

/// RFC 6330 octet field: GF(256) with primitive polynomial `0x11D` (also [`GenericRQHDPC`](fountain_scheme::precodes::GenericRQHDPC) default).
pub const RFC6330_GF256_PRIMITIVE_POLYNOMIAL: u16 = 0x11D;

#[derive(Clone, Debug)]
pub enum LDPCType {
    RQLDPC,
    ReversedLDPC,
    CodeSchemeRQLDPC,
}

impl LDPCType {
    fn create(&self, params: &CodeParams) -> Box<dyn LDPC> {
        match self {
            LDPCType::RQLDPC => Box::new(RQLDPC::new(params)),
            LDPCType::ReversedLDPC => Box::new(ReversedLDPC::new(params)),
            LDPCType::CodeSchemeRQLDPC => Box::new(fountain_scheme::precodes::RQLDPC::new(params)),
        }
    }
}

/// RaptorQ Systematic Code with RFC 6330 Degree Set
///
/// This struct provides a systematic fountain code implementation using RaptorQ parameters,
/// RFC 6330's degree-set generator, RFC-aligned LDPC relationships, and RFC octet arithmetic.
///
/// GF(256) for HDPC and inactivation LU is configured automatically during precoding via
/// [`HDPC::gf_poly`](fountain_engine::traits::HDPC::gf_poly) on the RFC 6330 HDPC instance
/// from [`fountain_scheme::precodes::rq_hdpc`].
#[derive(Clone)]
pub struct raptor_q_main {
    params: CodeParams,
    ldpc_type: LDPCType,
    k: usize, // Store k for RFC6330DegreeSet creation
}

impl raptor_q_main {
    /// Create a new RaptorQ Systematic Code with RFC 6330 degree set
    ///
    /// # Arguments
    ///
    /// * `k` - Number of source symbols
    /// * `_dmax` - Maximum degree for LT encoding (not used but kept for API compatibility)
    /// * `ldpc_type` - LDPC type
    ///
    /// # Returns
    ///
    /// A new `RaptorQSysCodeRFC6330` instance
    ///
    /// # Panics
    ///
    /// Panics if k is too large and parameters are not found in the RaptorQ table
    pub fn new(k: usize, _dmax: usize, ldpc_type: LDPCType) -> Self {
        let rq_params = params_table::get_params(k)
            .unwrap_or_else(|| panic!("RaptorQ parameters not found for k={}", k));
        let w = params_table::find_w_for_k(k)
            .unwrap_or_else(|| panic!("RaptorQ W parameter not found for k={}", k));

        let active_message_symbols = w
            .checked_sub(rq_params.l)
            .unwrap_or_else(|| panic!("Invalid RaptorQ parameters for k={}: W < S", k));
        let params = CodeParams::new(
            rq_params.k_prime,
            active_message_symbols,
            rq_params.l,
            rq_params.h,
        );

        Self {
            params,
            ldpc_type,
            k,
        }
    }

    /// Create a new RaptorQ Systematic Code with RFC 6330 degree set and default settings
    pub fn new_with_default_setting(k: usize) -> Self {
        Self::new(k, 30.min(k), LDPCType::RQLDPC)
    }

    /// Creates an encoder. GF(256) is set during precoding from the HDPC `gf_poly()`.
    pub fn new_encoder(&self) -> Encoder {
        Encoder::new(self.clone())
    }

    /// Creates an encoder with a data operator. Field sync uses `config_finite_field` at precoding.
    pub fn new_encoder_with_operator(&self, operator: Box<dyn DataOperator>) -> Encoder {
        Encoder::new_with_operator(self.clone(), operator)
    }

    /// Creates a decoder. GF(256) is set when the system solver configures the HDPC field.
    pub fn new_decoder(&self) -> Decoder {
        Decoder::new(self.clone())
    }

    /// Creates a decoder with a data operator.
    pub fn new_decoder_with_operator(&self, operator: Box<dyn DataOperator>) -> Decoder {
        Decoder::new_with_operator(self.clone(), operator)
    }

    fn dynamic_inactivation_budget(&self) -> usize {
        self.params.num_pre_inactive() + self.params.k / 2 + 100
    }
}

/// RFC 6330 HDPC (搂5.3.3.4): 螖-column rows from the RFC `Rand` function.
#[must_use]
pub fn rfc6330_hdpc(h: usize) -> Box<dyn HDPC> {
    let delta_column_fn = Box::new(move |j: usize| {
        let r1 = rand((j + 1) as u32, 6, h as u32) as usize;
        let r2 = (r1 + rand((j + 1) as u32, 7, (h - 1) as u32) as usize + 1) % h;
        vec![r1, r2]
    });
    rq_hdpc(delta_column_fn)
}

impl CodeScheme for raptor_q_main {
    fn get_params(&self) -> CodeParams {
        self.params.clone()
    }

    fn code_type(&self) -> CodeType {
        CodeType::Systematic
    }

    fn create_degree_set_fn(&self) -> Box<dyn FnMut(usize) -> Vec<usize>> {
        let mut degree_set = RFC6330DegreeSet::new(self.k);
        let inactive_offset = self.params.num_active();
        Box::new(move |coded_id| {
            let (mut active_indices, inactive_indices) = degree_set.degree_set(coded_id);
            active_indices.extend(inactive_indices.into_iter().map(|i| i + inactive_offset));
            active_indices
        })
    }

    fn create_precode(&self) -> (Option<Box<dyn HDPC>>, Option<Box<dyn LDPC>>) {
        let hdpc = if self.params.h == 0 {
            None
        } else {
            Some(rfc6330_hdpc(self.params.h))
        };
        let ldpc = if self.params.l == 0 {
            None
        } else {
            Some(self.ldpc_type.create(&self.params))
        };
        (hdpc, ldpc)
    }

    fn decoding_config(&self) -> DecodingConfig {
        DecodingConfig::default().with_max_inact_num(self.dynamic_inactivation_budget())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_raptorq_sys_rfc6330_creation() {
        let k = 50;
        let dmax = 30;
        let ldpc_type = LDPCType::ReversedLDPC;

        let code = raptor_q_main::new(k, dmax, ldpc_type);

        assert!(code.get_params().k >= k);
        assert!(code.get_params().h > 0);
        assert!(code.get_params().l > 0);
        assert_eq!(code.code_type(), CodeType::Systematic);
    }

    #[test]
    fn test_raptorq_sys_rfc6330_with_default_setting() {
        let k = 100;
        let code = raptor_q_main::new_with_default_setting(k);

        assert!(code.get_params().k >= k);
        assert!(code.get_params().h > 0);
        assert!(code.get_params().l > 0);
        assert_eq!(code.code_type(), CodeType::Systematic);
    }

    #[test]
    fn test_degree_set_function() {
        let k = 20;
        let code = raptor_q_main::new_with_default_setting(k);
        let mut degree_set_fn = code.create_degree_set_fn();

        let sources = degree_set_fn(0);
        assert!(!sources.is_empty());
    }

    #[test]
    fn test_precode_creation() {
        let k = 25;
        let dmax = 15;
        let ldpc_type = LDPCType::ReversedLDPC;

        let code = raptor_q_main::new(k, dmax, ldpc_type);
        let (hdpc, ldpc) = code.create_precode();

        assert!(hdpc.is_some());
        assert!(ldpc.is_some());
    }

    #[test]
    fn test_flat_degree_set_boundary_matches_rfc_w() {
        let k = 36479;
        let code = raptor_q_main::new_with_default_setting(k);
        let params = code.get_params();
        let rq_params = params_table::get_params(k).unwrap();
        let w = params_table::find_w_for_k(k).unwrap();

        assert_eq!(params.num_active(), w);
        assert_eq!(params.num_inactive(), rq_params.total_l - w);
    }
}

use crate::RQLDPC;
use crate::RQHDPC;
use crate::generators::RFC6330DegreeSet;
use crate::generators::rand_num_gen::rand;
use crate::params_table;
use fountain_engine::traits::DataOperator;
use fountain_engine::traits::{CodeScheme, HDPC, LDPC};
use fountain_engine::types::{CodeParams, CodeType, DecodingConfig, SubstitutionMethod};
use fountain_engine::{Decoder, Encoder};
use fountain_scheme::precodes::{ReversedLDPC, rq_hdpc};
use fountain_utility::{BlockSizePolicy, PaddedDecoder, PaddedEncoder};
use std::sync::Arc;
use std::sync::OnceLock;

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

#[derive(Clone, Debug)]
pub enum HDPCType {
    RQHDPC,
    CodeSchemeRQHDPC,
}

impl HDPCType {
    fn create(&self, params: &CodeParams, cache: Arc<OnceLock<(CodeParams, Vec<usize>, Vec<Vec<u8>>)>>) -> Box<dyn HDPC> {
        let h = params.h;
        let delta = rfc6330_delta_fn(h);
        match self {
            HDPCType::RQHDPC => Box::new(RQHDPC::new(delta, cache)),
            HDPCType::CodeSchemeRQHDPC => rq_hdpc(delta),
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
    hdpc_type: HDPCType,
    code_type: CodeType,
    k: usize, // Store k for RFC6330DegreeSet creation
    subs_method: Option<SubstitutionMethod>,
    cached_hdpc_lu: Arc<OnceLock<(CodeParams,Vec<usize>, Vec<Vec<u8>>)>>, // Cache for LU decomposition
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
            hdpc_type: HDPCType::RQHDPC,  //也就是说HDPC默认使用RFC 6330的Δ-column生成方式
            code_type: CodeType::Systematic, // 默认使用系统码
            k,
            subs_method: None,
            cached_hdpc_lu: Arc::new(OnceLock::new()), // Initialize the LU cache
        }
    }

    /// Create a new RaptorQ Systematic Code with RFC 6330 degree set and default settings
    pub fn new_with_default_setting(k: usize) -> Self {
        Self::new(k, 30.min(k), LDPCType::RQLDPC)
    }

    /// Override the back-substitution method used during decoding.
    pub fn with_subs_method(mut self, subs_method: SubstitutionMethod) -> Self {
        self.subs_method = Some(subs_method);
        self
    }

    /// Override the code type (default is [`CodeType::Systematic`](fountain_engine::types::CodeType::Systematic)).
    pub fn with_code_type(mut self, code_type: CodeType) -> Self {
        self.code_type = code_type;
        self
    }

    /// Override the HDPC type (default is [`HDPCType::RQHDPC`](HDPCType::RQHDPC)).
    pub fn with_hdpc_type(mut self, hdpc_type: HDPCType) -> Self {
        self.hdpc_type = hdpc_type;
        self
    }

    /// Application source block size K (RFC 6330 source symbols).
    pub fn source_symbols(&self) -> usize {
        self.k
    }

    /// RFC block size K′ used internally (`CodeParams.k`).
    pub fn block_symbols(&self) -> usize {
        self.params.k
    }

    /// Number of implicit zero padding symbols (`K′ − K`).
    pub fn num_padding(&self) -> usize {
        self.block_symbols().saturating_sub(self.source_symbols())
    }

    /// Creates an encoder. GF(256) is set during precoding from the HDPC `gf_poly()`.
    pub fn new_encoder(&self) -> Encoder {
        Encoder::new(self)
    }

    /// Creates an encoder with a data operator. Field sync uses `config_finite_field` at precoding.
    pub fn new_encoder_with_operator(&self, operator: Box<dyn DataOperator>) -> Encoder {
        Encoder::new_with_operator(self, operator)
    }

    /// Creates a decoder. GF(256) is set when the system solver configures the HDPC field.
    pub fn new_decoder(&self) -> Decoder {
        Decoder::new(self)
    }

    /// Creates a decoder with a data operator.
    pub fn new_decoder_with_operator(&self, operator: Box<dyn DataOperator>) -> Decoder {
        Decoder::new_with_operator(self, operator)
    }

    /// Creates a decoder with a data operator in execute-only mode (no operation log).
    pub fn new_decoder_with_operator_execute_only(
        &self,
        operator: Box<dyn DataOperator>,
    ) -> Decoder {
        Decoder::new_with_operator_execute_only(self, operator)
    }

    /// Padding-aware encoder ([`PaddedEncoder`](fountain_utility::PaddedEncoder)); same as [`Self::new_encoder`] when `K = K′`.
    pub fn padded_encoder(&self) -> PaddedEncoder<Self> {
        PaddedEncoder::new(self.clone())
    }

    /// Padding-aware encoder with a data operator.
    ///
    /// `symbol_len` may be `0` when `K > 0` to infer from `operator.get_vector(0)`.
    pub fn padded_encoder_with_operator(
        &self,
        operator: Box<dyn DataOperator>,
        symbol_len: usize,
    ) -> PaddedEncoder<Self> {
        PaddedEncoder::new_with_operator(self.clone(), operator, symbol_len)
    }

    /// Padding-aware decoder ([`PaddedDecoder`](fountain_utility::PaddedDecoder)).
    pub fn padded_decoder(&self) -> PaddedDecoder<Self> {
        PaddedDecoder::new(self.clone())
    }

    /// Padding-aware decoder with a data operator.
    pub fn padded_decoder_with_operator(
        &self,
        operator: Box<dyn DataOperator>,
        symbol_len: usize,
    ) -> PaddedDecoder<Self> {
        PaddedDecoder::new_with_operator(self.clone(), operator, symbol_len)
    }

    fn dynamic_inactivation_budget(&self) -> usize {
        self.params.num_pre_inactive() + self.params.k / 2 + 100
    }
}

/// RFC 6330 §5.3.3.4 Δ-matrix column generator (shared by both HDPC paths).
fn rfc6330_delta_fn(h: usize) -> Box<dyn Fn(usize) -> Vec<usize>> {
    Box::new(move |j| {
        let r1 = rand((j + 1) as u32, 6, h as u32) as usize;
        let r2 = (r1 + rand((j + 1) as u32, 7, (h - 1) as u32) as usize + 1) % h;
        vec![r1, r2]
    })
}

/// RFC 6330 HDPC (§5.3.3.4): Δ-column rows from the RFC `Rand` function.
#[must_use]
pub fn rfc6330_hdpc(h: usize) -> Box<dyn HDPC> {
    let delta_column_fn = rfc6330_delta_fn(h);
    rq_hdpc(delta_column_fn)
}

impl CodeScheme for raptor_q_main {
    fn get_params(&self) -> CodeParams {
        self.params.clone()
    }

    fn code_type(&self) -> CodeType {
        self.code_type
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
            Some(self.hdpc_type.create(&self.params, self.cached_hdpc_lu.clone()))
        };
        let ldpc = if self.params.l == 0 {
            None
        } else {
            Some(self.ldpc_type.create(&self.params))
        };
        (hdpc, ldpc)
    }

    fn decoding_config(&self) -> DecodingConfig {
        let mut config = DecodingConfig::default();
        //config.inac_strategy = InactivationStrategy::TrailRun;
        if let Some(subs_method) = self.subs_method {
            config.subs_method = subs_method;
        } else if self.params.k > 500 {
            config.subs_method = SubstitutionMethod::Original;
        }
        config.max_inactive_num = self.dynamic_inactivation_budget();
        config
    }
}

impl BlockSizePolicy for raptor_q_main {
    fn source_symbols(&self) -> usize {
        raptor_q_main::source_symbols(self)
    }

    fn block_symbols(&self) -> usize {
        raptor_q_main::block_symbols(self)
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
    fn test_raptorq_ord_rfc6330_creation() {
        let k = 50;
        let dmax = 30;
        let ldpc_type = LDPCType::ReversedLDPC;

        let code = raptor_q_main::new(k, dmax, ldpc_type)
            .with_code_type(CodeType::Ordinary);

        assert!(code.get_params().k >= k);
        assert!(code.get_params().h > 0);
        assert!(code.get_params().l > 0);
        assert_eq!(code.code_type(), CodeType::Ordinary);
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
    fn padding_counts_match_rfc_table() {
        let code = raptor_q_main::new(11, 11, LDPCType::RQLDPC);
        assert_eq!(code.source_symbols(), 11);
        assert_eq!(code.block_symbols(), 12);
        assert_eq!(code.num_padding(), 1);
    }

    #[test]
    fn no_padding_when_k_equals_k_prime() {
        let code = raptor_q_main::new(10, 10, LDPCType::RQLDPC);
        assert_eq!(code.num_padding(), 0);
    }

    #[test]
    fn substitution_method_override_takes_precedence() {
        let code = raptor_q_main::new_with_default_setting(1000)
            .with_subs_method(SubstitutionMethod::Direct);

        assert_eq!(
            code.decoding_config().subs_method,
            SubstitutionMethod::Direct
        );
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

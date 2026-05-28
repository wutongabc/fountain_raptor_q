use fountain_engine::traits::LDPC;
use fountain_engine::types::CodeParams;

/// RFC 6330 LDPC pre-coding relationships.
///
/// This implementation assumes the engine variable order used by `raptor_q_main`:
///
/// ```text
/// active variables   = B | S
/// inactive variables = U | H
/// ```
///
/// RFC 6330 Figure 5 writes the LDPC block as `G_LDPC,1 | I_S | G_LDPC,2`.
/// The engine adds the `I_S` column separately in `add_ldpc_constraint_by_id`,
/// so this type returns only `G_LDPC,1` from `active_row` and `G_LDPC,2`
/// from `inactive_row`.
#[derive(Debug, Clone)]
pub struct RQLDPC {
    b: usize,
    s: usize,
    p: usize,
}

impl RQLDPC {
    #[must_use]
    pub fn new(params: &CodeParams) -> Self {
        Self {
            b: params.a,
            s: params.l,
            p: params.num_inactive(),
        }
    }
}

fn toggle_index(indices: &mut Vec<usize>, value: usize) {
    if let Some(pos) = indices.iter().position(|&x| x == value) {
        indices.remove(pos);
    } else {
        indices.push(value);
    }
}

impl LDPC for RQLDPC {
    /// RFC 6330 Section 5.3.3.3, first LDPC loop:
    /// for B-column `i`, add it to rows `b`, `b+a`, and `b+2a` modulo S.
    #[inline]
    fn active_column(&self, var_col: usize) -> Vec<usize> {
        if self.s == 0 || var_col >= self.b {
            return Vec::new();
        }

        let a = 1 + var_col / self.s;
        let b = var_col % self.s;
        let mut rows = Vec::with_capacity(3);
        toggle_index(&mut rows, b);
        toggle_index(&mut rows, (b + a) % self.s);
        toggle_index(&mut rows, (b + 2 * a) % self.s);
        rows.sort_unstable();
        rows
    }

    /// Row view of `G_LDPC,1` over the B block.
    #[inline]
    fn active_row(&self, check_row: usize) -> Vec<usize> {
        if self.s == 0 || check_row >= self.s {
            return Vec::new();
        }

        let mut cols = Vec::new();
        for col in 0..self.b {
            let a = 1 + col / self.s;
            let b = col % self.s;
            if check_row == b || check_row == (b + a) % self.s || check_row == (b + 2 * a) % self.s
            {
                toggle_index(&mut cols, col);
            }
        }
        cols.sort_unstable();
        cols
    }

    /// RFC 6330 Section 5.3.3.3, second LDPC loop:
    /// row `i` uses PI columns `i mod P` and `(i + 1) mod P`.
    #[inline]
    fn inactive_row(&self, check_row: usize) -> Vec<usize> {
        if self.p == 0 || check_row >= self.s {
            return Vec::new();
        }

        let mut cols = Vec::with_capacity(2);
        toggle_index(&mut cols, check_row % self.p);
        toggle_index(&mut cols, (check_row + 1) % self.p);
        cols.sort_unstable();
        cols
    }

    /// Column view of `G_LDPC,2` over the P block.
    #[inline]
    fn inactive_column(&self, var_col: usize) -> Vec<usize> {
        if self.p == 0 || var_col >= self.p {
            return Vec::new();
        }

        let mut rows = Vec::new();
        let prev = (var_col + self.p - 1) % self.p;
        let mut row = var_col;
        while row < self.s {
            toggle_index(&mut rows, row);
            row += self.p;
        }
        row = prev;
        while row < self.s {
            toggle_index(&mut rows, row);
            row += self.p;
        }
        rows.sort_unstable();
        rows
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::params_table;
    use crate::raptor_q_main::raptor_q_main;
    use fountain_engine::{CodeScheme, DataOperator, DecodeStatus};
    use fountain_utility::VecDataOperater;

    #[test]
    fn k149_rows_match_rfc_6330_examples() {
        let rq_params = params_table::get_params(149).unwrap();
        let w = params_table::find_w_for_k(149).unwrap();
        let params = CodeParams::new(rq_params.k_prime, w - rq_params.l, rq_params.l, rq_params.h);
        let ldpc = RQLDPC::new(&params);

        assert_eq!(
            ldpc.active_row(0),
            vec![
                0, 21, 22, 23, 42, 44, 46, 63, 66, 69, 84, 88, 92, 105, 110, 115, 126, 132, 138
            ]
        );
        assert_eq!(ldpc.inactive_row(0), vec![0, 1]);
        assert_eq!(ldpc.active_column(0), vec![0, 1, 2]);
    }

    #[test]
    fn k149_round_trips_with_default_rfc_ldpc() {
        let k = 149;
        let symbol_size = 4;
        let config = raptor_q_main::new_with_default_setting(k);
        let params = config.get_params();
        let k_prime = params.k;

        let mut message_vectors = vec![vec![0u8; symbol_size]; k_prime];
        for (i, vector) in message_vectors.iter_mut().enumerate() {
            for (j, value) in vector.iter_mut().enumerate() {
                *value = ((i * j + i + j) % 256) as u8;
            }
        }

        let mut vec_data_operater = VecDataOperater::new(symbol_size);
        for (i, vector) in message_vectors.iter().enumerate() {
            vec_data_operater.insert_vector(vector, i);
        }

        let mut encoder = config.new_encoder_with_operator(Box::new(vec_data_operater));
        for coded_id in 0..k_prime {
            encoder.encode_coded_vector(coded_id);
        }

        let mut decoder = config.new_decoder_with_operator(Box::new(VecDataOperater::new(symbol_size)));
        let mut decoded = false;
        for coded_id in 0..k_prime {
            let coded_vector = encoder.manager.get_coded_vector(coded_id);
            if decoder.add_coded_vector(coded_id, &coded_vector) == DecodeStatus::Decoded {
                decoded = true;
                break;
            }
        }

        assert!(decoded);
        for (i, expected) in message_vectors.iter().enumerate() {
            assert_eq!(decoder.manager.get_data_vector(i), *expected);
        }
    }
}

// Run with:
// `cargo run --example diag_ids_singular`

use fountain_engine::CodeScheme;
use fountain_engine::algebra::finite_field::GF256;
use fountain_engine::algebra::linear_algebra::Matrix;
// use fountain_engine::traits::{HDPC, LDPC};
use fountain_raptor_q::{LDPCType, raptor_q_main::raptor_q_main};

fn diag(k: usize, ldpc_type: LDPCType) {
    let config = raptor_q_main::new(k, 30, ldpc_type.clone());
    let params = config.get_params();
    let (hdpc, ldpc) = config.create_precode();
    let hdpc = hdpc.unwrap();
    let ldpc = ldpc.unwrap();

    println!(
        "k={} K'={} a={} b={} l={} h={} kl={}  ldpc={:?}",
        k, params.k, params.a, params.b, params.l, params.h, params.num_message_ldpc(), ldpc_type
    );

    let gf = GF256::default();
    let sh_column = |row: usize| -> Vec<usize> {
        ldpc.inactive_row(row)
            .into_iter()
            .filter(|&id| id >= params.b)
            .map(|id| id - params.b)
            .collect()
    };
    let kl = params.num_message_ldpc();
    let mut rows = vec![vec![0u8; params.h]; kl];
    for i in 0..params.l {
        for &col in &sh_column(i) {
            rows[params.a + i][col] ^= 1;
        }
    }
    let mut m = hdpc.mul_binary_from_rows(Some(&gf), &params, &rows);
    for (i, row) in m.iter_mut().enumerate().take(params.h) {
        row[i] ^= 1;
    }
    let (_, r) = Matrix::lu_decomp(&gf, &mut m);
    println!("rank = {}, h = {}  -> {}\n", r, params.h, if r == params.h { "OK" } else { "FAIL" });
}

fn main() {
    for &k in &[728, 9497, 347, 15808, 15977, 23252] {
        diag(k, LDPCType::RQLDPC);
        diag(k, LDPCType::ReversedLDPC);
    }
}

// RQLDPC unable: 728, 9497
// ReversedLDPC unable: 347, 15808, 15977, 23252

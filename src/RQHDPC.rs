use fountain_engine::DataManager;
use fountain_engine::algebra::finite_field::{GF2, GF256};
use fountain_engine::algebra::linear_algebra::{Vector, Matrix};
use fountain_engine::traits::{HDPC, LDPC};
use fountain_engine::types::CodeParams;
use fountain_scheme::precodes::GenericRQHDPC;
use std::cell::RefCell;

pub struct RQHDPC {
    inner: GenericRQHDPC,
    cached_lu: RefCell<Option<(Vec<usize>, Vec<Vec<u8>>)>>, 
}

impl RQHDPC {
    #[must_use]
    pub fn new(delta_column_fn: Box<dyn Fn(usize) -> Vec<usize>>) -> Self {
        Self {
            inner: GenericRQHDPC::new(delta_column_fn),
            cached_lu: RefCell::new(None),
        }
    }

}

impl HDPC for RQHDPC {
    fn gf_poly(&self) -> u16 { 
        self.inner.gf_poly() 
    }

    fn mul_data(
        &self,
        manager: &mut DataManager,
        params: &CodeParams,
        x_ids: &[usize],
        y_ids: &[usize],
    ) {
        self.inner.mul_data(manager, params, x_ids, y_ids)
    }

    fn mul_binary(
        &self,
        gf: Option<&GF256>,
        params: &CodeParams,
        n: usize,
        v: &dyn Fn(usize) -> Vec<u8>,
    ) -> Vec<Vec<u8>> {
        self.inner.mul_binary(gf, params, n, v)
    }
    
    fn mul_binary_from_rows(
        &self,
        gf: Option<&GF256>,
        params: &CodeParams,
        rows: &[Vec<u8>],
    ) -> Vec<Vec<u8>> {
        self.inner.mul_binary_from_rows(gf, params, rows)
    }

    fn mul_sparse(
        &self,
        gf: Option<&GF256>,
        params: &CodeParams,
        n: usize,
        s: &dyn Fn(usize) -> Vec<usize>,
    ) -> Vec<Vec<u8>> {
        self.inner.mul_sparse(gf, params, n, s)
    }

    fn mul_sparse_sh(
        &self,
        gf: Option<&GF256>,
        params: &CodeParams,
        s: &dyn Fn(usize) -> Vec<usize>,
    ) -> Vec<Vec<u8>> {
        self.inner.mul_sparse_sh(gf, params, s)
    }


    fn lu_idssh(
        &self,
        gf: Option<&GF256>,
        params: &CodeParams,
        ldpc: &dyn LDPC,
    ) -> (Vec<usize>, Vec<Vec<u8>>) {
        if let Some(ref cached) = *self.cached_lu.borrow() {
            return cached.clone();
        }

        let (p, m) = self.inner.lu_idssh(gf, params, ldpc);
        *self.cached_lu.borrow_mut() = Some((p.clone(), m.clone()));

        // let sh_column = |row: usize| {
        //     ldpc.inactive_row(row)
        //         .into_iter()
        //         .filter(|&id| id >= params.b)
        //         .map(|id| id - params.b)
        //         .collect::<Vec<_>>()
        // };

        // // 第一步：尝试稀疏路径
        // let mut m = self.mul_sparse_sh(gf, params, &sh_column);
        // for (i, row) in m.iter_mut().enumerate().take(params.h) {
        //     row[i] ^= 1; // GF(256) addition is XOR in this codebase
        // }

        // let (p, r) = match gf {
        //     Some(gf) => Matrix::lu_decomp(gf, &mut m),
        //     None => Matrix::lu_decomp(&GF2::new(), &mut m),
        // };

        // // 第二步：稀疏路径满秩 → 直接缓存返回
        // if r == params.h {
        //     *self.cached_lu.borrow_mut() = Some((p.clone(), m.clone()));
        //     return (p, m);
        // }

        // // 第三步：回退到稠密路径
        // eprintln!("I' + D_s S_h singular (sparse), rank {r} < {h}, falling back to dense", r = r, h = params.h);        
        // let kl = params.num_message_ldpc();
        // let mut rows = vec![vec![0u8; params.h]; kl];
        // for i in 0..params.l {
        //     for &col in &sh_column(i) {
        //         rows[params.a + i][col] ^= 1;
        //     }
        // }
        // m = self.mul_binary_from_rows(gf, params, &rows);
        // for (i, row) in m.iter_mut().enumerate().take(params.h) {
        //     row[i] ^= 1;
        // }
        
        // let (p, r) = match gf {
        //     Some(gf) => Matrix::lu_decomp(gf, &mut m),
        //     None => Matrix::lu_decomp(&GF2::new(), &mut m),
        // };    

        // *self.cached_lu.borrow_mut() = Some((p.clone(), m.clone()));
        // assert_eq!(r, params.h, "I' + D_s S_h singular, rank {r} (dense fallback also failed)");

        (p, m)
    }

}
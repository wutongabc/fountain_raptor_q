use std::sync::OnceLock;

/// RaptorQ parameters structure
#[derive(Debug, Clone, Copy)]
pub struct RaptorQParams {
    pub k: usize,       // Original symbol count
    pub k_prime: usize, // Extended source symbols (K')
    pub l: usize,       // LDPC symbols (S in CSV)
    pub h: usize,       // HDPC symbols (H in CSV)
    pub pi: usize,      // Preinactive number (K'+S+H-W)
    pub total_l: usize, // Total intermediate symbols (K'+S+H)
    pub j: usize,       // Systematic index (J)
}

/// CSV row data
#[derive(Debug)]
struct CsvRow {
    k_prime: usize,
    j: usize,
    s: usize,
    h: usize,
    w: usize,
}

/// The raw CSV content of the parameters table
pub const PARAMS_CSV: &str = include_str!("raptor_q_para.csv");

/// Global params table cache
static PARAMS_TABLE: OnceLock<Vec<CsvRow>> = OnceLock::new();

/// Load CSV data into memory
fn load_csv() -> Vec<CsvRow> {
    PARAMS_CSV
        .lines()
        .skip(1) // Skip header
        .filter(|line| !line.trim().is_empty())
        .filter_map(|line| {
            let parts: Vec<&str> = line.split(',').collect();
            if parts.len() >= 5 {
                Some(CsvRow {
                    k_prime: parts[0].trim().parse().ok()?,
                    j: parts[1].trim().parse().ok()?,
                    s: parts[2].trim().parse().ok()?,
                    h: parts[3].trim().parse().ok()?,
                    w: parts[4].trim().parse().ok()?,
                })
            } else {
                None
            }
        })
        .collect()
}

/// Get params table (lazy initialization)
fn get_table() -> &'static Vec<CsvRow> {
    PARAMS_TABLE.get_or_init(load_csv)
}

/// Find the smallest K' that is at least k
fn find_params(k: usize) -> Option<&'static CsvRow> {
    get_table().iter().find(|row| row.k_prime >= k)
}

/// Get RaptorQ parameters for given k
pub fn get_params(k: usize) -> Option<RaptorQParams> {
    let row = find_params(k)?;

    Some(RaptorQParams {
        k,
        k_prime: row.k_prime,
        l: row.s,
        h: row.h,
        pi: row.k_prime + row.s + row.h - row.w,
        total_l: row.k_prime + row.s + row.h,
        j: row.j,
    })
}

/// Get preinactive number for given k
pub fn get_inactive_number(k: usize) -> Option<usize> {
    let row = find_params(k)?;
    Some(row.k_prime + row.s + row.h - row.w)
}

/// Get K' for given k
pub fn get_k_prime(k: usize) -> Option<usize> {
    find_params(k).map(|row| row.k_prime)
}

/// Get systematic index J for given k
pub fn get_systematic_index(k: usize) -> Option<usize> {
    find_params(k).map(|row| row.j)
}

/// Get W parameter for given k
pub fn find_w_for_k(k: usize) -> Option<usize> {
    let row = find_params(k)?;
    Some(row.w)
}

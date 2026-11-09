//! `sampling_systematic(n)`: keep every k-th row, the smallest k that leaves at
//! most `n` rows (`k = ceil(row_count / n)`), starting with the first row.
//!
//! lets-plot computes its step differently, so it may keep a few rows fewer or
//! more; both spread the kept rows evenly.

use serde_json::Map;

use super::{Sampler, check_options, count};
use crate::Result;
use crate::spec::Value;

/// Keeps every k-th row.
#[derive(Debug)]
pub struct SystematicSampler {
    n: usize,
}

impl Sampler for SystematicSampler {
    fn keep(&self, row_count: usize) -> Option<Vec<u32>> {
        if self.n >= row_count {
            return None;
        }
        let step = row_count.div_ceil(self.n);
        Some((0..row_count).step_by(step).map(|row| row as u32).collect())
    }
}

pub(super) fn parse(options: &Map<String, Value>, path: &str) -> Result<Box<dyn Sampler>> {
    const FUNCTION: &str = "sampling_systematic";
    check_options(options, &["n"], FUNCTION, path)?;
    Ok(Box::new(SystematicSampler {
        n: count(options, FUNCTION, path)?,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_kth_row_at_most_n() {
        let keep = |n, row_count| SystematicSampler { n }.keep(row_count);
        assert_eq!(keep(3, 30), Some(vec![0, 10, 20]));
        assert_eq!(keep(50, 1000).map(|rows| rows.len()), Some(50));
        // ceil(1001 / 50) = 21: 48 rows.
        assert_eq!(keep(50, 1001).map(|rows| rows.len()), Some(48));
        assert_eq!(keep(1, 5), Some(vec![0]));
        assert_eq!(keep(5, 5), None);
        assert_eq!(keep(9, 5), None);
    }
}

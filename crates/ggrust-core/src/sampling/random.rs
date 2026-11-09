//! `sampling_random(n, seed)`: keep `n` rows chosen uniformly at random.
//!
//! Floyd's algorithm over a bit set: `n` draws, `row_count / 8` bytes, and the
//! kept rows come out ascending. The same seed gives the same rows. The picks are
//! ggrust's own (Kotlin's `Random` sequence, but not lets-plot's algorithm).

use serde_json::Map;

use super::{Sampler, check_options, count, seed};
use crate::Result;
use crate::position::random::KotlinRandom;
use crate::spec::Value;

/// Keeps `n` random rows.
#[derive(Debug)]
pub struct RandomSampler {
    n: usize,
    seed: i64,
}

impl Sampler for RandomSampler {
    fn keep(&self, row_count: usize) -> Option<Vec<u32>> {
        (self.n < row_count).then(|| pick(row_count, self.n, self.seed))
    }
}

pub(super) fn parse(options: &Map<String, Value>, path: &str) -> Result<Box<dyn Sampler>> {
    const FUNCTION: &str = "sampling_random";
    check_options(options, &["n", "seed"], FUNCTION, path)?;
    Ok(Box::new(RandomSampler {
        n: count(options, FUNCTION, path)?,
        seed: seed(options, FUNCTION, path)?,
    }))
}

/// `n` distinct rows out of `row_count` (`n < row_count`), ascending.
pub(super) fn pick(row_count: usize, n: usize, seed: i64) -> Vec<u32> {
    let mut random = KotlinRandom::new(seed);
    let mut chosen = vec![0u64; row_count.div_ceil(64)];
    for candidate in row_count - n..row_count {
        // Uniform in [0, candidate].
        let draw = (random.next_double() * (candidate + 1) as f64) as usize;
        let row = draw.min(candidate);
        let taken = chosen[row / 64] & (1 << (row % 64)) != 0;
        let row = if taken { candidate } else { row };
        chosen[row / 64] |= 1 << (row % 64);
    }
    let mut rows = Vec::with_capacity(n);
    for (word_index, &word) in chosen.iter().enumerate() {
        let mut bits = word;
        while bits != 0 {
            rows.push((word_index * 64 + bits.trailing_zeros() as usize) as u32);
            bits &= bits - 1;
        }
    }
    rows
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keeps_n_distinct_ascending_rows() {
        for (row_count, n) in [(10, 1), (10, 9), (1000, 37), (100_000, 5000), (65, 64)] {
            let rows = pick(row_count, n, 7);
            assert_eq!(rows.len(), n);
            assert!(rows.windows(2).all(|pair| pair[0] < pair[1]));
            assert!(rows.iter().all(|&row| (row as usize) < row_count));
        }
    }

    #[test]
    fn same_seed_same_rows() {
        assert_eq!(pick(10_000, 100, 3), pick(10_000, 100, 3));
        assert_ne!(pick(10_000, 100, 3), pick(10_000, 100, 4));
    }

    #[test]
    fn spread_over_all_rows() {
        // Half of 100k rows: each half of the input keeps about a quarter.
        let rows = pick(100_000, 50_000, 1);
        let low = rows.iter().filter(|&&row| row < 50_000).count();
        assert!((24_000..26_000).contains(&low), "{low}");
    }

    #[test]
    fn n_at_or_above_the_row_count_keeps_all() {
        let sampler = RandomSampler { n: 10, seed: 0 };
        assert_eq!(sampler.keep(10), None);
        assert_eq!(sampler.keep(3), None);
        assert_eq!(sampler.keep(11).map(|rows| rows.len()), Some(10));
    }
}

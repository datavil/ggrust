//! When work goes to rayon's thread pool, and an order-keeping map that runs
//! on it or on the calling thread.
//!
//! Waking the pool from outside costs tens of microseconds per call, more than
//! small plots take to build or write, so parallel paths start at
//! [`MIN_PARALLEL_ITEMS`] rows (or marks).
//!
//! What it does not do: split work inside one item (stats do that per group).

use rayon::prelude::*;

/// Rows or marks below which work stays on the calling thread.
pub const MIN_PARALLEL_ITEMS: usize = 10_000;

/// Whether work over `items` rows or marks is worth the thread pool.
pub fn is_worth(items: usize) -> bool {
    items >= MIN_PARALLEL_ITEMS
}

/// `items` mapped by `map`, results in input order: in parallel when `parallel`
/// and there is more than one item, else on the calling thread.
pub fn ordered_map<T: Send, R: Send>(
    items: Vec<T>,
    parallel: bool,
    map: impl Fn(T) -> R + Sync + Send,
) -> Vec<R> {
    if parallel && items.len() > 1 {
        items.into_par_iter().map(map).collect()
    } else {
        items.into_iter().map(map).collect()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn keeps_order_both_ways() {
        let items: Vec<usize> = (0..100).collect();
        let doubled: Vec<usize> = (0..100).map(|item| item * 2).collect();
        assert_eq!(ordered_map(items.clone(), true, |item| item * 2), doubled);
        assert_eq!(ordered_map(items, false, |item| item * 2), doubled);
    }

    #[test]
    fn threshold() {
        assert!(!is_worth(MIN_PARALLEL_ITEMS - 1));
        assert!(is_worth(MIN_PARALLEL_ITEMS));
    }
}

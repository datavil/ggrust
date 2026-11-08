//! numpy-compatible arithmetic shared by the ports that must match numpy bit for bit
//! (`np.add.reduce`, `np.linspace`, `np.interp`).
//!
//! What it does not do: anything vectorised or general-purpose; each helper mirrors
//! one numpy routine's operation order for float64 only.

/// numpy's `add.reduce` over a contiguous axis: a pairwise sum (blocks of 8
/// up to 128 values, sequential below 8).
pub(crate) fn pairwise_sum(values: &[f64]) -> f64 {
    let n = values.len();
    if n < 8 {
        values.iter().fold(0.0, |sum, value| sum + value)
    } else if n <= 128 {
        let mut partial = [0.0; 8];
        partial.copy_from_slice(&values[..8]);
        let blocks = n - n % 8;
        for block in values[8..blocks].chunks_exact(8) {
            for (sum, value) in partial.iter_mut().zip(block) {
                *sum += value;
            }
        }
        let mut sum = ((partial[0] + partial[1]) + (partial[2] + partial[3]))
            + ((partial[4] + partial[5]) + (partial[6] + partial[7]));
        for value in &values[blocks..] {
            sum += value;
        }
        sum
    } else {
        let half = n / 2;
        let half = half - half % 8;
        pairwise_sum(&values[..half]) + pairwise_sum(&values[half..])
    }
}

/// `np.linspace(start, stop, num)` (endpoint included), `num >= 2`.
pub(crate) fn linspace(start: f64, stop: f64, num: usize) -> Vec<f64> {
    let delta = stop - start;
    let divisor = (num - 1) as f64;
    let step = delta / divisor;
    let mut values: Vec<f64> = if step == 0.0 {
        // numpy's zero-step branch: `i / div * delta`.
        (0..num)
            .map(|index| index as f64 / divisor * delta + start)
            .collect()
    } else {
        (0..num).map(|index| index as f64 * step + start).collect()
    };
    values[num - 1] = stop;
    values
}

/// `np.interp(position, np.arange(len(values)), values)`: `values` sampled at the
/// integers `0..len`, read at a fractional `position` (clamped to the ends).
///
/// numpy's slope form `slope * (x - xp[j]) + fp[j]`, contracted to a fused
/// multiply-add as clang compiles numpy on arm64.
pub(crate) fn interp_index(position: f64, values: &[f64]) -> f64 {
    let last = values.len() - 1;
    if position.is_nan() {
        return position;
    }
    if position <= 0.0 {
        return values[0];
    }
    if position >= last as f64 {
        return values[last];
    }
    let index = position.floor() as usize;
    let offset = position - index as f64;
    if offset == 0.0 {
        return values[index];
    }
    let slope = values[index + 1] - values[index];
    let result = slope.mul_add(offset, values[index]);
    if result.is_nan() {
        let result = slope.mul_add(position - (index + 1) as f64, values[index + 1]);
        if result.is_nan() && values[index] == values[index + 1] {
            return values[index];
        }
        return result;
    }
    result
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn pairwise_sum_matches_sequential_on_small_inputs() {
        assert_eq!(pairwise_sum(&[1.0, 2.0, 3.0]), 6.0);
        let values: Vec<f64> = (0..300).map(f64::from).collect();
        assert_eq!(pairwise_sum(&values), 44850.0);
    }

    #[test]
    fn linspace_ends_exactly() {
        let values = linspace(-1.5, 2.25, 7);
        assert_eq!(values.len(), 7);
        assert_eq!(values[0], -1.5);
        assert_eq!(values[6], 2.25);
        assert_eq!(linspace(3.0, 3.0, 3), vec![3.0, 3.0, 3.0]);
    }

    #[test]
    fn interp_index_reads_between_samples() {
        let values = [0.0, 10.0, 30.0];
        assert_eq!(interp_index(0.5, &values), 5.0);
        assert_eq!(interp_index(1.25, &values), 15.0);
        assert_eq!(interp_index(2.0, &values), 30.0);
        assert_eq!(interp_index(-1.0, &values), 0.0);
        assert_eq!(interp_index(1.0, &values), 10.0);
    }
}

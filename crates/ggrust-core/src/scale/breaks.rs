//! "Nice" break positions for continuous scales (port of lets-plot `LinearBreaksHelper`).

/// lets-plot `SeriesUtil.MAX_DECIMAL_PLACES`: beyond this, steps are lost in f64 noise.
const MAX_DECIMAL_PLACES: f64 = 12.0;

/// Whether `delta` is too small to be resolved next to `base` (lets-plot
/// `isBeyondPrecision`, using magnitudes so negative bases are handled too).
pub fn is_beyond_precision(base: f64, delta: f64) -> bool {
    if base == 0.0 {
        return false;
    }
    base.abs().log10() - delta.abs().log10() > MAX_DECIMAL_PLACES
}

/// Whether a range is too narrow to be a real range (zero-width or noise).
pub fn is_degenerate_range(low: f64, high: f64) -> bool {
    let delta = high - low;
    delta < 1e-50 || is_beyond_precision(low, delta) || is_beyond_precision(high, delta)
}

/// About `target_count` evenly spaced breaks with a step of 1, 2 or 5 x 10^k, inside
/// `[low, high]`. Zero is always a break when the range spans it.
pub fn linear_breaks(low: f64, high: f64, target_count: usize) -> Vec<f64> {
    let (low, high) = if low <= high {
        (low, high)
    } else {
        (high, low)
    };
    if !(low.is_finite() && high.is_finite()) || target_count == 0 {
        return Vec::new();
    }
    let step = nice_step(high - low, target_count);
    if !(step.is_finite() && step > 0.0)
        || is_beyond_precision(low, step)
        || is_beyond_precision(high, step)
    {
        return vec![low];
    }
    let breaks = breaks_with_step(low, high, step);
    if breaks.is_empty() { vec![low] } else { breaks }
}

/// A step that is a multiple of 10, 5 or 2 (lets-plot `computeNiceStep`).
pub fn nice_step(span: f64, count: usize) -> f64 {
    let raw = span / count as f64;
    let step = 10_f64.powf(raw.log10().floor());
    let error = step * count as f64 / span;
    if error <= 0.15 {
        step * 10.0
    } else if error <= 0.35 {
        step * 5.0
    } else if error <= 0.75 {
        step * 2.0
    } else {
        step
    }
}

/// Multiples of `step` within `[low, high]` (a tolerance of step / 10000 absorbs
/// floating point error). Values are computed as `k * step` and rounded to the
/// step's decimals, so 0.1 x 3 gives 0.3, never 0.30000000000000004.
pub fn breaks_with_step(low: f64, high: f64, step: f64) -> Vec<f64> {
    let tolerance = step / 10_000.0;
    let first = ((low - tolerance) / step).ceil();
    let last = ((high + tolerance) / step).floor();
    if !(first.is_finite() && last.is_finite()) || last - first > 100_000.0 {
        return Vec::new();
    }
    // Round each break one digit finer than the step, relative to the step's
    // magnitude, so tiny steps (p-values, 1e-20) keep their precision.
    let decimals = 1 - step.log10().floor() as i32;
    let mut breaks = Vec::new();
    let mut multiple = first;
    while multiple <= last {
        let value = round_to_decimals(multiple * step, decimals);
        // -0.0 -> 0.0, and keep breaks inside the domain.
        let value = (value + 0.0).clamp(low, high);
        if breaks.last() != Some(&value) {
            breaks.push(value);
        }
        multiple += 1.0;
    }
    breaks
}

/// Round `value` to `decimals` decimal places (negative: to tens, hundreds, ...).
/// Powers of ten up to 1e22 are exact in f64; beyond that the value is kept as is.
fn round_to_decimals(value: f64, decimals: i32) -> f64 {
    if !value.is_finite() || decimals.abs() > 22 {
        return value;
    }
    let rounded = if decimals >= 0 {
        let scale = 10_f64.powi(decimals);
        (value * scale).round() / scale
    } else {
        let unit = 10_f64.powi(-decimals);
        (value / unit).round() * unit
    };
    if rounded.is_finite() { rounded } else { value }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn steps_are_one_two_or_five() {
        assert_eq!(nice_step(3.85, 10), 0.5);
        assert_eq!(nice_step(10.0, 5), 2.0);
        assert_eq!(nice_step(1e7, 5), 2e6);
    }

    #[test]
    fn breaks_match_lets_plot_examples() {
        // x 1..4.5 expanded by 5%, 11 target breaks (the lets-plot reference plot).
        assert_eq!(
            linear_breaks(0.825, 4.675, 11),
            [1.0, 1.5, 2.0, 2.5, 3.0, 3.5, 4.0, 4.5]
        );
        assert_eq!(
            breaks_with_step(0.0, 10.0, 2.0),
            [0.0, 2.0, 4.0, 6.0, 8.0, 10.0]
        );
        assert_eq!(breaks_with_step(0.5, 10.5, 2.0), [2.0, 4.0, 6.0, 8.0, 10.0]);
        assert_eq!(
            breaks_with_step(-10.0, -2.0, 2.0),
            [-10.0, -8.0, -6.0, -4.0, -2.0]
        );
        assert_eq!(
            breaks_with_step(-5.0, 5.0, 2.0),
            [-4.0, -2.0, 0.0, 2.0, 4.0]
        );
        assert_eq!(
            breaks_with_step(0.0, 1.0, 0.25),
            [0.0, 0.25, 0.5, 0.75, 1.0]
        );
    }

    #[test]
    fn zero_is_exact_and_positive() {
        let breaks = linear_breaks(-0.1675000000000001, 0.4374999999999998, 15);
        assert!(breaks.contains(&0.0));
        assert!(linear_breaks(0.0, 10.0, 10)[0].is_sign_positive());
        assert!(linear_breaks(-1.0e-290, 1.0e-290, 3).contains(&0.0));
    }

    #[test]
    fn noisy_unit_range_gives_clean_breaks() {
        let breaks = linear_breaks(4.8174916649430757e-144, 0.9999999999998934, 5);
        assert_eq!(breaks.len(), 6, "{breaks:?}");
        assert_eq!(&breaks[1..5], [0.2, 0.4, 0.6, 0.8]);
    }

    #[test]
    fn beyond_precision_gives_one_break() {
        assert_eq!(linear_breaks(1.0, 1.0 + 1e-13, 5), [1.0]);
        assert!(is_degenerate_range(1.0, 1.0));
        assert!(is_degenerate_range(1e6, 1e6 + 1e-9));
        assert!(!is_degenerate_range(0.0, 1e-9));
    }

    #[test]
    fn tiny_magnitudes_keep_their_breaks() {
        assert_eq!(
            linear_breaks(1e-20, 5e-20, 5),
            [1e-20, 2e-20, 3e-20, 4e-20, 5e-20]
        );
        assert_eq!(linear_breaks(0.0, 3e-15, 5).len(), 7);
        let pvalues = linear_breaks(1.65e-17, 9.35e-17, 10);
        assert_eq!(pvalues.len(), 8, "{pvalues:?}");
        assert_eq!(pvalues[0], 2e-17);
    }

    #[test]
    fn large_and_small_magnitudes_scale() {
        let large = linear_breaks(0.825e5, 4.675e5, 11);
        assert_eq!(large, [1e5, 1.5e5, 2e5, 2.5e5, 3e5, 3.5e5, 4e5, 4.5e5]);
        let small = linear_breaks(0.825e-5, 4.675e-5, 11);
        assert_eq!(small.len(), 8);
        assert!((small[1] - 1.5e-5).abs() < 1e-18);
    }
}

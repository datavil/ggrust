//! Distribution functions behind the hypothesis tests of `stat/hypothesis.rs`:
//! normal, Student's t, F and chi-squared tails, and the exact null
//! distributions of the Wilcoxon rank sum and signed rank statistics, Kendall's
//! T and Spearman's S, as R's `pnorm`, `pt`, `pf`, `pchisq`, `pwilcox`,
//! `psignrank`, `pkendall` and `prho` (AS 89) give them.
//!
//! Tails are computed directly (not as `1 - cdf`), so small p-values keep their
//! relative precision. Exact distributions count arrangements in `f64` with
//! additions only (no cancellation).
//!
//! Quantiles for Q-Q plots: `qnorm` (Wichura's AS 241), the regularized gamma's
//! inverse by safeguarded Newton steps, and `t_quantile` from `smooth.rs`.
//!
//! What it does not do: non-central distributions.

use super::smooth::{ln_gamma, regularized_beta};

/// `P(Z > z)` for a standard normal `Z`.
pub(super) fn normal_upper(z: f64) -> f64 {
    if z.is_nan() {
        return f64::NAN;
    }
    if z < 0.0 {
        return 1.0 - normal_upper(-z);
    }
    // erfc(z / sqrt 2) / 2 = Q(1/2, z^2 / 2) / 2.
    0.5 * regularized_gamma_upper(0.5, z * z / 2.0)
}

/// `P(Z < z)` for a standard normal `Z`.
pub(super) fn normal_lower(z: f64) -> f64 {
    normal_upper(-z)
}

/// `P(|T| > |t|)` for Student's t with `freedom` degrees of freedom.
pub(super) fn t_two_sided(t: f64, freedom: f64) -> f64 {
    if t.is_nan() || freedom.is_nan() || freedom <= 0.0 {
        return f64::NAN;
    }
    if t.is_infinite() {
        return 0.0;
    }
    let squared = t * t;
    // I_x(df/2, 1/2) with x = df / (df + t^2) and 1 - x = t^2 / (df + t^2).
    beta_lower(
        freedom / (freedom + squared),
        squared / (freedom + squared),
        freedom / 2.0,
        0.5,
    )
}

/// `P(F > f)` for the F distribution with `(d1, d2)` degrees of freedom.
pub(super) fn f_upper(f: f64, d1: f64, d2: f64) -> f64 {
    if f.is_nan() || d1.is_nan() || d2.is_nan() || d1 <= 0.0 || d2 <= 0.0 {
        return f64::NAN;
    }
    if f <= 0.0 {
        return 1.0;
    }
    if f.is_infinite() {
        return 0.0;
    }
    beta_lower(
        d2 / (d2 + d1 * f),
        d1 * f / (d2 + d1 * f),
        d2 / 2.0,
        d1 / 2.0,
    )
}

/// `I_x(a, b)` given `x` and `y = 1 - x` (each computed without cancellation):
/// the continued fraction on the side of the mean where it is the smaller tail,
/// so neither tail loses precision to `1 - ...`.
fn beta_lower(x: f64, y: f64, a: f64, b: f64) -> f64 {
    if x <= (a + 1.0) / (a + b + 2.0) {
        regularized_beta(x, a, b)
    } else {
        1.0 - regularized_beta(y, b, a)
    }
}

/// `P(X > x)` for chi-squared with `freedom` degrees of freedom.
pub(super) fn chi_squared_upper(x: f64, freedom: f64) -> f64 {
    if x.is_nan() || freedom.is_nan() || freedom <= 0.0 {
        return f64::NAN;
    }
    if x <= 0.0 {
        return 1.0;
    }
    regularized_gamma_upper(freedom / 2.0, x / 2.0)
}

/// The standard normal quantile (Wichura's AS 241, R's `qnorm`), `0 < p < 1`.
pub(super) fn normal_quantile(p: f64) -> f64 {
    let q = p - 0.5;
    if q.abs() <= 0.425 {
        let r = 0.180625 - q * q;
        return q * rational(
            r,
            &[
                3.387132872796366_6,
                133.141_667_891_784_38,
                1_971.590_950_306_551_3,
                13_731.693_765_509_46,
                45_921.953_931_549_87,
                67_265.770_927_008_7,
                33_430.575_583_588_13,
                2_509.080_928_730_122_7,
            ],
            &[
                1.0,
                42.313_330_701_600_91,
                687.187_007_492_057_9,
                5_394.196_021_424_751,
                21_213.794_301_586_597,
                39_307.895_800_092_71,
                28_729.085_735_721_943,
                5_226.495_278_852_545,
            ],
        );
    }
    let tail = if q < 0.0 { p } else { 1.0 - p };
    let r = (-tail.ln()).sqrt();
    let value = if r <= 5.0 {
        rational(
            r - 1.6,
            &[
                1.423_437_110_749_683_5,
                4.630_337_846_156_546,
                5.769_497_221_460_691,
                3.647_848_324_763_204_5,
                1.270_458_252_452_368_4,
                0.241_780_725_177_450_6,
                0.022_723_844_989_269_184,
                7.745_450_142_783_414e-4,
            ],
            &[
                1.0,
                2.053_191_626_637_759,
                1.676_384_830_183_803_8,
                0.689_767_334_985_1,
                0.148_103_976_427_480_08,
                0.015_198_666_563_616_457,
                5.475_938_084_995_345e-4,
                1.050_750_071_644_416_9e-9,
            ],
        )
    } else {
        rational(
            r - 5.0,
            &[
                6.657_904_643_501_103,
                5.463_784_911_164_114,
                1.784_826_539_917_291_3,
                0.296_560_571_828_504_87,
                0.026_532_189_526_576_124,
                0.001_242_660_947_388_078_4,
                2.711_555_568_743_487_6e-5,
                2.010_334_399_292_288_1e-7,
            ],
            &[
                1.0,
                0.599_832_206_555_888,
                0.136_929_880_922_735_8,
                0.014_875_361_290_850_615,
                7.868_691_311_456_133e-4,
                1.846_318_317_510_054_8e-5,
                1.421_511_758_316_446e-7,
                2.044_263_103_389_939_8e-15,
            ],
        )
    };
    if q < 0.0 { -value } else { value }
}

/// `numerator(x) / denominator(x)`, coefficients from degree 0 up.
fn rational(x: f64, numerator: &[f64], denominator: &[f64]) -> f64 {
    let horner = |coefficients: &[f64]| {
        coefficients
            .iter()
            .rev()
            .fold(0.0, |sum, coefficient| sum * x + coefficient)
    };
    horner(numerator) / horner(denominator)
}

/// The quantile of a gamma with shape `a` and rate 1, `0 < p < 1`: Newton steps on
/// `ln P(a, e^u)`, which is concave in `u`, from `P(a, x) <= x^a / Gamma(a + 1)`, a
/// start left of the root: the steps rise to it without overshooting. An answer
/// below the smallest `f64` is 0, as R's `qgamma`.
pub(super) fn gamma_quantile(p: f64, a: f64) -> f64 {
    let target = p.ln();
    let mut u = (target + ln_gamma(a + 1.0)) / a;
    for _ in 0..500 {
        let x = u.exp();
        if x == 0.0 {
            return 0.0;
        }
        let lower = regularized_gamma_lower(a, x);
        let slope = (a * u - x - ln_gamma(a)).exp() / lower;
        let step = (target - lower.ln()) / slope;
        u += step;
        if step.abs() <= 1e-14 * u.abs().max(1.0) {
            break;
        }
    }
    u.exp()
}

/// The regularized lower incomplete gamma function `P(a, x)`: summed directly
/// below `x = a + 1`, so tiny values keep their precision, else `1 - Q`.
fn regularized_gamma_lower(a: f64, x: f64) -> f64 {
    if x >= a + 1.0 {
        return 1.0 - regularized_gamma_upper(a, x);
    }
    let (mut term, mut sum, mut denominator) = (1.0 / a, 1.0 / a, a);
    for _ in 0..10_000 {
        denominator += 1.0;
        term *= x / denominator;
        sum += term;
        if term.abs() < sum.abs() * 1e-17 {
            break;
        }
    }
    sum * (-x + a * x.ln() - ln_gamma(a)).exp()
}

/// The regularized upper incomplete gamma function `Q(a, x)`: a series for
/// `x < a + 1` (as `1 - P`), else a continued fraction (modified Lentz).
fn regularized_gamma_upper(a: f64, x: f64) -> f64 {
    if x <= 0.0 {
        return 1.0;
    }
    if x.is_infinite() {
        return 0.0;
    }
    let front = (-x + a * x.ln() - ln_gamma(a)).exp();
    if x < a + 1.0 {
        let (mut term, mut sum, mut denominator) = (1.0 / a, 1.0 / a, a);
        for _ in 0..10_000 {
            denominator += 1.0;
            term *= x / denominator;
            sum += term;
            if term.abs() < sum.abs() * 1e-17 {
                break;
            }
        }
        return 1.0 - sum * front;
    }
    const TINY: f64 = 1e-300;
    let mut b = x + 1.0 - a;
    let mut c = 1.0 / TINY;
    let mut d = 1.0 / b;
    let mut result = d;
    for i in 1..10_000 {
        let i = i as f64;
        let an = -i * (i - a);
        b += 2.0;
        d = an * d + b;
        if d.abs() < TINY {
            d = TINY;
        }
        c = b + an / c;
        if c.abs() < TINY {
            c = TINY;
        }
        d = 1.0 / d;
        let delta = d * c;
        result *= delta;
        if (delta - 1.0).abs() < 1e-16 {
            break;
        }
    }
    front * result
}

/// Counts of an exact null distribution over integer statistic values
/// `0..counts.len()`, with both tails summed directly.
#[derive(Debug, Clone)]
pub(super) struct Counts {
    counts: Vec<f64>,
    total: f64,
}

impl Counts {
    fn new(counts: Vec<f64>) -> Self {
        let total = counts.iter().sum();
        Counts { counts, total }
    }

    /// `P(S <= q)` (R floors `q + 1e-7`).
    pub(super) fn lower(&self, q: f64) -> f64 {
        let q = (q + 1e-7).floor();
        if q < 0.0 {
            return 0.0;
        }
        let last = (q as usize).min(self.counts.len() - 1);
        self.counts[..=last].iter().sum::<f64>() / self.total
    }

    /// `P(S >= q)`.
    pub(super) fn upper(&self, q: f64) -> f64 {
        let q = (q - 1e-7).ceil().max(0.0);
        if q as usize >= self.counts.len() {
            return 0.0;
        }
        self.counts[q as usize..].iter().sum::<f64>() / self.total
    }
}

/// Null distribution of the Wilcoxon rank sum `W` (Mann-Whitney U) for sample
/// sizes `m` and `n` (R `dwilcox`): `W` from 0 to `m n`.
pub(super) fn wilcox_counts(m: usize, n: usize) -> Counts {
    let (small, total) = (m.min(n), m + n);
    // ways[j][s]: j-subsets of the ranks seen so far with rank sum s.
    let max_sum = small * total;
    let mut ways = vec![vec![0.0_f64; max_sum + 1]; small + 1];
    ways[0][0] = 1.0;
    for rank in 1..=total {
        for chosen in (1..=small.min(rank)).rev() {
            let (below, here) = ways.split_at_mut(chosen);
            let (previous, current) = (&below[chosen - 1], &mut here[0]);
            for sum in (rank..=max_sum).rev() {
                current[sum] += previous[sum - rank];
            }
        }
    }
    let offset = small * (small + 1) / 2;
    Counts::new(ways[small][offset..=offset + m * n].to_vec())
}

/// Null distribution of the Wilcoxon signed rank `V` for `n` non-zero
/// differences (R `dsignrank`): `V` from 0 to `n (n + 1) / 2`.
pub(super) fn signrank_counts(n: usize) -> Counts {
    let max_sum = n * (n + 1) / 2;
    let mut ways = vec![0.0_f64; max_sum + 1];
    ways[0] = 1.0;
    for rank in 1..=n {
        for sum in (rank..=max_sum).rev() {
            ways[sum] += ways[sum - rank];
        }
    }
    Counts::new(ways)
}

/// Null distribution of Kendall's `T` (concordant pairs) for `n` untied pairs
/// (R `ckendall`): permutations by inversion count, `T` from 0 to `n (n - 1) / 2`.
pub(super) fn kendall_counts(n: usize) -> Counts {
    let mut ways = vec![1.0_f64];
    for size in 2..=n {
        let mut next = vec![0.0_f64; ways.len() + size - 1];
        // next[k] = sum of ways[k - j] for j in 0..size (a sliding window).
        let mut window = 0.0;
        for (k, slot) in next.iter_mut().enumerate() {
            if k < ways.len() {
                window += ways[k];
            }
            if k >= size && k - size < ways.len() {
                window -= ways[k - size];
            }
            *slot = window;
        }
        ways = next;
    }
    Counts::new(ways)
}

/// `P(S >= s)` (or `P(S < s)` with `lower`) for Spearman's `S = sum d^2` of `n`
/// untied pairs, by Best and Roberts' algorithm AS 89 (R `prho`): exact
/// enumeration up to `n = 9`, an Edgeworth series above.
pub(super) fn spearman_tail(n: usize, s: f64, lower: bool) -> f64 {
    let outside = if lower { 0.0 } else { 1.0 };
    if n <= 1 || s <= 0.0 {
        return outside;
    }
    let nf = n as f64;
    if s > nf * (nf * nf - 1.0) / 3.0 {
        return 1.0 - outside;
    }
    if n <= 9 {
        let (at_least, total) = spearman_exact(n, s);
        return if lower {
            (total - at_least) / total
        } else {
            at_least / total
        };
    }
    const C: [f64; 12] = [
        0.2274, 0.2531, 0.1745, 0.0758, 0.1033, 0.3932, 0.0879, 0.0151, 0.0072, 0.0831, 0.0131,
        4.6e-4,
    ];
    let b = 1.0 / nf;
    let x = (6.0 * (s - 1.0) * b / (nf * nf - 1.0) - 1.0) * (1.0 / b - 1.0).sqrt();
    let y = x * x;
    let u = x
        * b
        * (C[0]
            + b * (C[1] + C[2] * b)
            + y * (-C[3] + b * (C[4] + C[5] * b)
                - y * b * (C[6] + C[7] * b - y * (C[8] - C[9] * b + y * b * (C[10] - C[11] * y)))));
    let correction = u / (y / 2.0).exp();
    let value = if lower {
        -correction + normal_lower(x)
    } else {
        correction + normal_upper(x)
    };
    value.clamp(0.0, 1.0)
}

/// Permutations of `1..=n` with `sum (i - p_i)^2 >= s`, and `n!`.
fn spearman_exact(n: usize, s: f64) -> (f64, f64) {
    let mut permutation: Vec<usize> = (0..n).collect();
    let mut state = vec![0_usize; n];
    let score = |permutation: &[usize]| -> f64 {
        permutation
            .iter()
            .enumerate()
            .map(|(i, &p)| {
                let d = i as f64 - p as f64;
                d * d
            })
            .sum()
    };
    let (mut at_least, mut total) = (0.0, 0.0);
    let mut count = |permutation: &[usize]| {
        total += 1.0;
        if score(permutation) >= s {
            at_least += 1.0;
        }
    };
    // Heap's algorithm.
    count(&permutation);
    let mut i = 0;
    while i < n {
        if state[i] < i {
            if i % 2 == 0 {
                permutation.swap(0, i);
            } else {
                permutation.swap(state[i], i);
            }
            count(&permutation);
            state[i] += 1;
            i = 0;
        } else {
            state[i] = 0;
            i += 1;
        }
    }
    (at_least, total)
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(actual: f64, expected: f64) -> bool {
        (actual - expected).abs() <= 1e-9 * expected.abs() + 1e-15
    }

    #[test]
    fn tails_match_r() {
        // pnorm(-c(0.5, 3, 8, 20)), pt(-c(1.2, 7), df), pf, pchisq (lower.tail = FALSE).
        for (z, r) in [
            (0.5, 0.308537538725987),
            (3.0, 0.00134989803163009),
            (8.0, 6.22096057427178e-16),
            (20.0, 2.75362411860623e-89),
        ] {
            assert!(close(normal_upper(z), r), "pnorm {z}: {}", normal_upper(z));
        }
        assert!(close(t_two_sided(1.2, 7.0) / 2.0, 0.134585968413603));
        assert!(close(t_two_sided(7.0, 3.5) / 2.0, 0.00177641809990213));
        assert!(close(f_upper(4.2, 2.0, 27.0), 0.0258146819565804));
        assert!(close(chi_squared_upper(40.6, 2.0), 1.52694015912661e-09));
        assert!(close(chi_squared_upper(0.3, 3.0), 0.960028480306878));
    }

    #[test]
    fn exact_distributions_match_r() {
        // pwilcox(c(3, 10), 4, 6); psignrank(5, 10); pkendall via cor.test.
        let wilcox = wilcox_counts(4, 6);
        assert!(close(wilcox.lower(3.0), 0.0333333333333333));
        assert!(close(wilcox.upper(11.0), 1.0 - 0.380952380952381));
        assert_eq!(wilcox_counts(6, 4).counts.len(), 25);
        assert!(close(signrank_counts(10).lower(5.0), 0.009765625));
        // 1 - P(T <= 2) for n = 4: permutations with at most 2 inversions are 9/24.
        assert!(close(kendall_counts(4).lower(2.0), 9.0 / 24.0));
        // .Call(stats:::C_pRho, s, n, lower.tail).
        assert!(close(spearman_tail(4, 18.0, false), 4.0 / 24.0));
        assert!(close(spearman_tail(4, 20.0, true), 23.0 / 24.0));
        for (s, n, lower, r) in [
            (250.0, 12, false, 0.658317232857697),
            (251.0, 12, false, 0.654238597501872),
            (252.0, 12, true, 0.349856110365861),
            (5000.0, 30, true, 0.723502117694013),
        ] {
            assert!(close(spearman_tail(n, s, lower), r), "prho {s} {n}");
        }
    }
}

//! Classical hypothesis tests and correlation tests, two-sided, with the
//! p-values R's `stats` package gives (`t.test`, `wilcox.test`, one-way
//! `anova`, `kruskal.test`, `cor.test`) and R's `p.adjust` methods.
//!
//! - `t.test`: Welch's two-sample test (R's default `var.equal = FALSE`), or the
//!   paired test on the differences.
//! - `wilcox.test`: the rank sum test (exact when both samples have fewer than 50
//!   values and no ties, else the normal approximation with continuity and tie
//!   corrections), or the signed rank test on paired differences (zeros dropped;
//!   exact below 50 differences without ties or zeros).
//! - `anova`: one-way F test; `kruskal.test`: tie-corrected H against chi-squared.
//! - `cor.test`: Pearson (t test), Spearman (exact AS 89 below 1291 untied pairs,
//!   else a t approximation) and Kendall's tau-b (exact below 50 untied pairs,
//!   else the tie-corrected normal approximation).
//!
//! Missing values are dropped first (pairwise for paired and correlation tests).
//! Inputs R rejects (too few values, constant data for t) are errors with R's
//! reason; degenerate inputs R answers with `NA` give a NaN p-value.
//!
//! What it does not do: one-sided alternatives, `mu`, `var.equal = TRUE`,
//! confidence intervals, exact tests with ties (R neither).

use super::distribution::{
    Counts, chi_squared_upper, f_upper, kendall_counts, normal_lower, normal_upper,
    signrank_counts, spearman_tail, t_two_sided, wilcox_counts,
};

/// Why a test cannot run (R's error, in words).
pub(super) type TestResult<T> = std::result::Result<T, String>;

/// A test comparing groups of values (ggpubr `method`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Method {
    TTest,
    WilcoxTest,
    Anova,
    KruskalTest,
}

impl Method {
    /// Accepted names, ggpubr's aliases included.
    pub(super) const NAMES: [&'static str; 4] = ["t.test", "wilcox.test", "anova", "kruskal.test"];

    /// [`Self::NAMES`] and ggpubr's aliases, for suggestions.
    pub(super) const ALIASES: [&'static str; 10] = [
        "t.test",
        "wilcox.test",
        "anova",
        "kruskal.test",
        "t",
        "student",
        "wilcox",
        "wilcoxon",
        "aov",
        "kruskal",
    ];

    /// The method of a ggpubr name or alias.
    pub(super) fn parse(name: &str) -> Option<Method> {
        match name {
            "t.test" | "t" | "student" => Some(Method::TTest),
            "wilcox.test" | "wilcox" | "wilcoxon" => Some(Method::WilcoxTest),
            "anova" | "aov" => Some(Method::Anova),
            "kruskal.test" | "kruskal" => Some(Method::KruskalTest),
            _ => None,
        }
    }

    /// ggpubr's label name.
    pub(super) fn label(self) -> &'static str {
        match self {
            Method::TTest => "T-test",
            Method::WilcoxTest => "Wilcoxon",
            Method::Anova => "Anova",
            Method::KruskalTest => "Kruskal-Wallis",
        }
    }

    /// Whether the method compares exactly two groups.
    pub(super) fn is_two_sample(self) -> bool {
        matches!(self, Method::TTest | Method::WilcoxTest)
    }

    /// Two-sided p-value of the groups (two for t and Wilcoxon tests; `paired`
    /// pairs the two groups' values in order).
    pub(super) fn p_value(self, groups: &[&[f64]], paired: bool) -> TestResult<f64> {
        let present: Vec<Vec<f64>> = groups
            .iter()
            .map(|values| values.iter().copied().filter(|v| !v.is_nan()).collect())
            .collect();
        if paired {
            let [x, y] = groups else {
                return Err(format!(
                    "a paired test compares 2 groups, got {}",
                    groups.len()
                ));
            };
            if x.len() != y.len() {
                return Err(format!(
                    "a paired test needs groups of equal size (pairs in row order), got {} and {} \
                     values",
                    x.len(),
                    y.len()
                ));
            }
            let differences: Vec<f64> = x
                .iter()
                .zip(*y)
                .filter(|(a, b)| !a.is_nan() && !b.is_nan())
                .map(|(a, b)| a - b)
                .collect();
            return match self {
                Method::TTest => paired_t_test(&differences),
                Method::WilcoxTest => signed_rank_test(&differences),
                Method::Anova | Method::KruskalTest => Err(format!(
                    "paired comparisons are only available for t.test and wilcox.test, not '{}'",
                    Method::NAMES[self as usize]
                )),
            };
        }
        match self {
            Method::TTest | Method::WilcoxTest => {
                let [x, y] = present.as_slice() else {
                    return Err(format!(
                        "{} compares 2 groups, got {}",
                        Method::NAMES[self as usize],
                        present.len()
                    ));
                };
                match self {
                    Method::TTest => welch_t_test(x, y),
                    _ => rank_sum_test(x, y),
                }
            }
            Method::Anova => one_way_anova(&present),
            Method::KruskalTest => kruskal_test(&present),
        }
    }
}

fn mean(values: &[f64]) -> f64 {
    let n = values.len() as f64;
    let first = values.iter().sum::<f64>() / n;
    // R refines the mean with a second pass.
    first + values.iter().map(|v| v - first).sum::<f64>() / n
}

/// Sample variance (n - 1).
fn variance(values: &[f64], center: f64) -> f64 {
    values.iter().map(|v| (v - center).powi(2)).sum::<f64>() / (values.len() as f64 - 1.0)
}

/// Welch's two-sample t test.
fn welch_t_test(x: &[f64], y: &[f64]) -> TestResult<f64> {
    if x.len() < 2 || y.len() < 2 {
        return Err("not enough observations for t.test (each group needs 2)".to_owned());
    }
    let (mean_x, mean_y) = (mean(x), mean(y));
    let error_x = variance(x, mean_x) / x.len() as f64;
    let error_y = variance(y, mean_y) / y.len() as f64;
    let error = (error_x + error_y).sqrt();
    if error < 10.0 * f64::EPSILON * mean_x.abs().max(mean_y.abs()) {
        return Err("data are essentially constant; t.test cannot compare them".to_owned());
    }
    let freedom = error.powi(4)
        / (error_x.powi(2) / (x.len() as f64 - 1.0) + error_y.powi(2) / (y.len() as f64 - 1.0));
    Ok(t_two_sided((mean_x - mean_y) / error, freedom))
}

/// One-sample t test of paired differences against 0.
fn paired_t_test(differences: &[f64]) -> TestResult<f64> {
    if differences.len() < 2 {
        return Err("not enough pairs for a paired t.test (needs 2)".to_owned());
    }
    let center = mean(differences);
    let n = differences.len() as f64;
    let error = (variance(differences, center) / n).sqrt();
    if error < 10.0 * f64::EPSILON * center.abs() {
        return Err("data are essentially constant; t.test cannot compare them".to_owned());
    }
    Ok(t_two_sided(center / error, n - 1.0))
}

/// Average ranks (R `rank`, ties `"average"`), 1-based.
pub(super) fn average_ranks(values: &[f64]) -> Vec<f64> {
    let mut order: Vec<usize> = (0..values.len()).collect();
    order.sort_by(|&a, &b| values[a].total_cmp(&values[b]));
    let mut ranks = vec![0.0; values.len()];
    let mut start = 0;
    while start < order.len() {
        let mut end = start + 1;
        while end < order.len() && values[order[end]] == values[order[start]] {
            end += 1;
        }
        let rank = (start + end + 1) as f64 / 2.0;
        for &index in &order[start..end] {
            ranks[index] = rank;
        }
        start = end;
    }
    ranks
}

/// Sizes of the groups of equal values (size 1 included).
fn tie_sizes(values: &[f64]) -> Vec<f64> {
    let mut sorted = values.to_vec();
    sorted.sort_by(f64::total_cmp);
    let mut sizes = Vec::new();
    let mut start = 0;
    while start < sorted.len() {
        let mut end = start + 1;
        while end < sorted.len() && sorted[end] == sorted[start] {
            end += 1;
        }
        sizes.push((end - start) as f64);
        start = end;
    }
    sizes
}

/// Two-sided exact p-value: the tail on the statistic's side of the centre, doubled.
fn exact_two_sided(counts: &Counts, statistic: f64, center: f64) -> f64 {
    let tail = if statistic > center {
        counts.upper(statistic)
    } else {
        counts.lower(statistic)
    };
    (2.0 * tail).min(1.0)
}

/// Two-sided normal p-value of a centred statistic with continuity correction.
fn normal_two_sided(centered: f64, sigma: f64) -> f64 {
    // R's sign(0) is 0: no correction at the centre.
    let z = (centered - 0.5 * centered.signum_or_zero()) / sigma;
    if z.is_nan() {
        return f64::NAN;
    }
    2.0 * normal_lower(z).min(normal_upper(z))
}

/// Wilcoxon rank sum test.
fn rank_sum_test(x: &[f64], y: &[f64]) -> TestResult<f64> {
    if x.is_empty() || y.is_empty() {
        return Err("not enough observations for wilcox.test (each group needs 1)".to_owned());
    }
    let all: Vec<f64> = x.iter().chain(y).copied().collect();
    let ranks = average_ranks(&all);
    let (n_x, n_y) = (x.len() as f64, y.len() as f64);
    let statistic = ranks[..x.len()].iter().sum::<f64>() - n_x * (n_x + 1.0) / 2.0;
    let ties = tie_sizes(&all);
    let tied = ties.len() != all.len();
    if x.len() < 50 && y.len() < 50 && !tied {
        let counts = wilcox_counts(x.len(), y.len());
        return Ok(exact_two_sided(&counts, statistic, n_x * n_y / 2.0));
    }
    let tie_sum: f64 = ties.iter().map(|t| t * t * t - t).sum();
    let total = n_x + n_y;
    let sigma = ((n_x * n_y / 12.0) * ((total + 1.0) - tie_sum / (total * (total - 1.0)))).sqrt();
    Ok(normal_two_sided(statistic - n_x * n_y / 2.0, sigma))
}

/// Wilcoxon signed rank test of paired differences against 0.
fn signed_rank_test(differences: &[f64]) -> TestResult<f64> {
    if differences.is_empty() {
        return Err("not enough pairs for a paired wilcox.test (needs 1)".to_owned());
    }
    let zeros = differences.contains(&0.0);
    let nonzero: Vec<f64> = differences.iter().copied().filter(|d| *d != 0.0).collect();
    let n = nonzero.len() as f64;
    let magnitudes: Vec<f64> = nonzero.iter().map(|d| d.abs()).collect();
    let ranks = average_ranks(&magnitudes);
    let statistic: f64 = ranks
        .iter()
        .zip(&nonzero)
        .filter(|(_, d)| **d > 0.0)
        .map(|(rank, _)| rank)
        .sum();
    let ties = tie_sizes(&ranks);
    let tied = ties.len() != ranks.len();
    if nonzero.len() < 50 && !tied && !zeros {
        let counts = signrank_counts(nonzero.len());
        return Ok(exact_two_sided(&counts, statistic, n * (n + 1.0) / 4.0));
    }
    let tie_sum: f64 = ties.iter().map(|t| t * t * t - t).sum();
    let sigma = (n * (n + 1.0) * (2.0 * n + 1.0) / 24.0 - tie_sum / 48.0).sqrt();
    Ok(normal_two_sided(statistic - n * (n + 1.0) / 4.0, sigma))
}

/// Non-empty groups, or R's error.
fn non_empty_groups<'a>(groups: &'a [Vec<f64>], name: &str) -> TestResult<Vec<&'a [f64]>> {
    let groups: Vec<&[f64]> = groups
        .iter()
        .filter(|group| !group.is_empty())
        .map(Vec::as_slice)
        .collect();
    if groups.len() < 2 {
        return Err(format!(
            "{name} needs at least 2 groups with values, got {}",
            groups.len()
        ));
    }
    Ok(groups)
}

/// One-way analysis of variance F test.
fn one_way_anova(groups: &[Vec<f64>]) -> TestResult<f64> {
    let groups = non_empty_groups(groups, "anova")?;
    let all: Vec<f64> = groups
        .iter()
        .flat_map(|group| group.iter().copied())
        .collect();
    let (n, k) = (all.len() as f64, groups.len() as f64);
    if n <= k {
        return Err(format!(
            "anova needs more values ({n}) than groups ({k}) to estimate the residual variance"
        ));
    }
    let grand = mean(&all);
    let (mut between, mut within) = (0.0, 0.0);
    for group in &groups {
        let center = mean(group);
        between += group.len() as f64 * (center - grand).powi(2);
        within += group.iter().map(|v| (v - center).powi(2)).sum::<f64>();
    }
    let f = (between / (k - 1.0)) / (within / (n - k));
    Ok(f_upper(f, k - 1.0, n - k))
}

/// Kruskal-Wallis rank sum test.
fn kruskal_test(groups: &[Vec<f64>]) -> TestResult<f64> {
    let groups = non_empty_groups(groups, "kruskal.test")?;
    let all: Vec<f64> = groups
        .iter()
        .flat_map(|group| group.iter().copied())
        .collect();
    let ranks = average_ranks(&all);
    let n = all.len() as f64;
    let mut statistic = 0.0;
    let mut start = 0;
    for group in &groups {
        let sum: f64 = ranks[start..start + group.len()].iter().sum();
        statistic += sum * sum / group.len() as f64;
        start += group.len();
    }
    let tie_sum: f64 = tie_sizes(&all).iter().map(|t| t * t * t - t).sum();
    let statistic =
        (12.0 * statistic / (n * (n + 1.0)) - 3.0 * (n + 1.0)) / (1.0 - tie_sum / (n * n * n - n));
    Ok(chi_squared_upper(statistic, groups.len() as f64 - 1.0))
}

/// A correlation method (R `cor.test` `method`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Correlation {
    Pearson,
    Spearman,
    Kendall,
}

impl Correlation {
    pub(super) const NAMES: [&'static str; 3] = ["pearson", "spearman", "kendall"];

    pub(super) fn parse(name: &str) -> Option<Correlation> {
        match name {
            "pearson" => Some(Correlation::Pearson),
            "spearman" => Some(Correlation::Spearman),
            "kendall" => Some(Correlation::Kendall),
            _ => None,
        }
    }

    /// The coefficient and its two-sided p-value for the complete pairs.
    pub(super) fn test(self, x: &[f64], y: &[f64]) -> TestResult<(f64, f64)> {
        let (x, y): (Vec<f64>, Vec<f64>) = x
            .iter()
            .zip(y)
            .filter(|(a, b)| a.is_finite() && b.is_finite())
            .map(|(a, b)| (*a, *b))
            .unzip();
        let minimum = if self == Correlation::Pearson { 3 } else { 2 };
        if x.len() < minimum {
            return Err(format!(
                "not enough complete (x, y) pairs for a {} correlation test: {} (needs {minimum})",
                Correlation::NAMES[self as usize],
                x.len()
            ));
        }
        Ok(match self {
            Correlation::Pearson => pearson_test(&x, &y),
            Correlation::Spearman => spearman_test(&x, &y),
            Correlation::Kendall => kendall_test(&x, &y),
        })
    }
}

/// Pearson's r as R's `cor` (two-pass means, clamped to [-1, 1]).
fn pearson(x: &[f64], y: &[f64]) -> f64 {
    let (mean_x, mean_y) = (mean(x), mean(y));
    let (mut xy, mut xx, mut yy) = (0.0, 0.0, 0.0);
    for (a, b) in x.iter().zip(y) {
        let (dx, dy) = (a - mean_x, b - mean_y);
        xy += dx * dy;
        xx += dx * dx;
        yy += dy * dy;
    }
    (xy / (xx.sqrt() * yy.sqrt())).clamp(-1.0, 1.0)
}

fn pearson_test(x: &[f64], y: &[f64]) -> (f64, f64) {
    let r = pearson(x, y);
    let freedom = x.len() as f64 - 2.0;
    let t = freedom.sqrt() * r / (1.0 - r * r).sqrt();
    (r, t_two_sided(t, freedom))
}

/// Whether either variable has repeated values (R's `TIES` for rank correlations).
fn has_ties(x: &[f64], y: &[f64]) -> bool {
    tie_sizes(x).len().min(tie_sizes(y).len()) < x.len()
}

fn spearman_test(x: &[f64], y: &[f64]) -> (f64, f64) {
    let rho = pearson(&average_ranks(x), &average_ranks(y));
    if rho.is_nan() {
        return (f64::NAN, f64::NAN);
    }
    let n = x.len() as f64;
    let q = (n * n * n - n) * (1.0 - rho) / 6.0;
    let upper_side = q > (n * n * n - n) / 6.0;
    let p = if x.len() <= 1290 && !has_ties(x, y) {
        match upper_side {
            true => spearman_tail(x.len(), q.round(), false),
            false => spearman_tail(x.len(), q.round() + 2.0, true),
        }
    } else {
        let r = 1.0 - q / (n * (n * n - 1.0) / 6.0);
        t_two_sided(r / ((1.0 - r * r) / (n - 2.0)).sqrt(), n - 2.0) / 2.0
    };
    (rho, (2.0 * p).min(1.0))
}

fn kendall_test(x: &[f64], y: &[f64]) -> (f64, f64) {
    let n = x.len();
    let mut score = 0.0;
    for i in 0..n {
        for j in 0..i {
            score += (x[i] - x[j]).signum_or_zero() * (y[i] - y[j]).signum_or_zero();
        }
    }
    let nf = n as f64;
    let pairs = nf * (nf - 1.0) / 2.0;
    let ties_x: Vec<f64> = tie_sizes(x).into_iter().filter(|t| *t > 1.0).collect();
    let ties_y: Vec<f64> = tie_sizes(y).into_iter().filter(|t| *t > 1.0).collect();
    let tied_pairs = |ties: &[f64]| ties.iter().map(|t| t * (t - 1.0)).sum::<f64>() / 2.0;
    let (tied_x, tied_y) = (tied_pairs(&ties_x), tied_pairs(&ties_y));
    let tau = score / ((pairs - tied_x) * (pairs - tied_y)).sqrt();
    if !tau.is_finite() {
        return (f64::NAN, f64::NAN);
    }
    if n < 50 && !has_ties(x, y) {
        let q = ((tau + 1.0) * nf * (nf - 1.0) / 4.0).round();
        let counts = kendall_counts(n);
        return (tau, exact_two_sided(&counts, q, nf * (nf - 1.0) / 4.0));
    }
    let sum = |ties: &[f64], f: &dyn Fn(f64) -> f64| ties.iter().map(|t| f(*t)).sum::<f64>();
    let s = tau * ((pairs - tied_x) * (pairs - tied_y)).sqrt();
    let v0 = nf * (nf - 1.0) * (2.0 * nf + 5.0);
    let vt = sum(&ties_x, &|t| t * (t - 1.0) * (2.0 * t + 5.0));
    let vu = sum(&ties_y, &|t| t * (t - 1.0) * (2.0 * t + 5.0));
    let v1 = sum(&ties_x, &|t| t * (t - 1.0)) * sum(&ties_y, &|t| t * (t - 1.0));
    let v2 =
        sum(&ties_x, &|t| t * (t - 1.0) * (t - 2.0)) * sum(&ties_y, &|t| t * (t - 1.0) * (t - 2.0));
    let variance = (v0 - vt - vu) / 18.0
        + v1 / (2.0 * nf * (nf - 1.0))
        + v2 / (9.0 * nf * (nf - 1.0) * (nf - 2.0));
    let z = s / variance.sqrt();
    (tau, 2.0 * normal_lower(z).min(normal_upper(z)))
}

/// `signum` with 0 for 0 (Rust's `f64::signum` gives 1 for +0).
trait SignumOrZero {
    fn signum_or_zero(self) -> f64;
}

impl SignumOrZero for f64 {
    fn signum_or_zero(self) -> f64 {
        if self > 0.0 {
            1.0
        } else if self < 0.0 {
            -1.0
        } else {
            0.0
        }
    }
}

/// A multiple-comparison adjustment (R `p.adjust` `method`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(super) enum Adjustment {
    None,
    Holm,
    Hochberg,
    Hommel,
    Bonferroni,
    BenjaminiHochberg,
    BenjaminiYekutieli,
}

impl Adjustment {
    pub(super) const NAMES: [&'static str; 8] = [
        "none",
        "holm",
        "hochberg",
        "hommel",
        "bonferroni",
        "BH",
        "BY",
        "fdr",
    ];

    pub(super) fn parse(name: &str) -> Option<Adjustment> {
        match name {
            "none" => Some(Adjustment::None),
            "holm" => Some(Adjustment::Holm),
            "hochberg" => Some(Adjustment::Hochberg),
            "hommel" => Some(Adjustment::Hommel),
            "bonferroni" => Some(Adjustment::Bonferroni),
            "BH" | "fdr" => Some(Adjustment::BenjaminiHochberg),
            "BY" => Some(Adjustment::BenjaminiYekutieli),
            _ => None,
        }
    }

    /// Adjusted p-values (NaN stay NaN and do not count), as R's `p.adjust`.
    pub(super) fn adjust(self, p_values: &[f64]) -> Vec<f64> {
        let present: Vec<usize> = (0..p_values.len())
            .filter(|&i| !p_values[i].is_nan())
            .collect();
        let p: Vec<f64> = present.iter().map(|&i| p_values[i]).collect();
        let adjusted = self.adjust_present(&p);
        let mut result = vec![f64::NAN; p_values.len()];
        for (index, value) in present.into_iter().zip(adjusted) {
            result[index] = value;
        }
        result
    }

    fn adjust_present(self, p: &[f64]) -> Vec<f64> {
        let n = p.len();
        if n <= 1 {
            return p.to_vec();
        }
        let nf = n as f64;
        // Stable orders, as R's `order`.
        let mut increasing: Vec<usize> = (0..n).collect();
        increasing.sort_by(|&a, &b| p[a].total_cmp(&p[b]));
        let mut decreasing: Vec<usize> = (0..n).collect();
        decreasing.sort_by(|&a, &b| p[b].total_cmp(&p[a]));
        let mut result = vec![0.0; n];
        match self {
            Adjustment::None => return p.to_vec(),
            Adjustment::Bonferroni => return p.iter().map(|v| (nf * v).min(1.0)).collect(),
            Adjustment::Holm => {
                let mut running = f64::NEG_INFINITY;
                for (i, &index) in increasing.iter().enumerate() {
                    running = running.max((nf - i as f64) * p[index]);
                    result[index] = running.min(1.0);
                }
            }
            Adjustment::Hochberg
            | Adjustment::BenjaminiHochberg
            | Adjustment::BenjaminiYekutieli => {
                let harmonic: f64 = (1..=n).map(|i| 1.0 / i as f64).sum();
                let mut running = f64::INFINITY;
                for (position, &index) in decreasing.iter().enumerate() {
                    let i = (n - position) as f64;
                    let factor = match self {
                        Adjustment::Hochberg => nf + 1.0 - i,
                        Adjustment::BenjaminiHochberg => nf / i,
                        _ => harmonic * nf / i,
                    };
                    running = running.min(factor * p[index]);
                    result[index] = running.min(1.0);
                }
            }
            Adjustment::Hommel => {
                let sorted: Vec<f64> = increasing.iter().map(|&index| p[index]).collect();
                let initial = (0..n)
                    .map(|i| nf * sorted[i] / (i + 1) as f64)
                    .fold(f64::INFINITY, f64::min);
                let mut q = vec![initial; n];
                let mut adjusted = vec![initial; n];
                for m in (2..n).rev() {
                    let mf = m as f64;
                    // i1 = 1..=n-m+1, i2 = n-m+2..=n (1-based).
                    let q1 = (n - m + 1..n)
                        .enumerate()
                        .map(|(k, i)| mf * sorted[i] / (k + 2) as f64)
                        .fold(f64::INFINITY, f64::min);
                    for i in 0..=n - m {
                        q[i] = (mf * sorted[i]).min(q1);
                    }
                    for i in n - m + 1..n {
                        q[i] = q[n - m];
                    }
                    for i in 0..n {
                        adjusted[i] = adjusted[i].max(q[i]);
                    }
                }
                for (position, &index) in increasing.iter().enumerate() {
                    result[index] = adjusted[position].max(sorted[position]);
                }
            }
        }
        result
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn close(actual: f64, expected: f64) -> bool {
        (actual - expected).abs() <= 1e-6 * expected.abs() + 1e-14
    }

    #[test]
    fn centred_statistics_get_no_correction() {
        // wilcox.test(c(1, 2, 2, 3), c(1, 2, 2, 3))$p.value is 1; paired all-zero
        // differences leave no pairs: NaN (R: NA).
        let same = [1.0, 2.0, 2.0, 3.0];
        let wilcox = Method::WilcoxTest;
        assert_eq!(wilcox.p_value(&[&same, &same], false).unwrap(), 1.0);
        let x = [1.0, 2.0, 3.0, 4.0, 5.0];
        assert!(wilcox.p_value(&[&x, &x], true).unwrap().is_nan());
    }

    #[test]
    fn ranks_average_ties() {
        assert_eq!(
            average_ranks(&[3.0, 1.0, 3.0, 2.0]),
            vec![3.5, 1.0, 3.5, 2.0]
        );
    }

    #[test]
    fn adjustments_match_r() {
        // p.adjust(c(0.01, 0.04, 0.03, 0.2, 0.01), method).
        let p = [0.01, 0.04, 0.03, 0.2, 0.01];
        let cases: [(&str, [f64; 5]); 6] = [
            ("holm", [0.05, 0.09, 0.09, 0.2, 0.05]),
            ("hochberg", [0.04, 0.08, 0.08, 0.2, 0.04]),
            ("hommel", [0.04, 0.08, 0.06, 0.2, 0.04]),
            ("bonferroni", [0.05, 0.2, 0.15, 1.0, 0.05]),
            ("BH", [0.025, 0.05, 0.05, 0.2, 0.025]),
            (
                "BY",
                [
                    0.0570833333333333,
                    0.114166666666667,
                    0.114166666666667,
                    0.456666666666667,
                    0.0570833333333333,
                ],
            ),
        ];
        for (name, expected) in cases {
            let adjusted = Adjustment::parse(name).unwrap().adjust(&p);
            for (a, e) in adjusted.iter().zip(expected) {
                assert!(close(*a, e), "{name}: {adjusted:?}");
            }
        }
    }

    /// tests/fixtures/pubr_reference.json, from generate_pubr_reference.R.
    fn reference() -> serde_json::Value {
        serde_json::from_str(include_str!(
            "../../../../tests/fixtures/pubr_reference.json"
        ))
        .unwrap()
    }

    fn numbers(value: &serde_json::Value) -> Vec<f64> {
        match value {
            serde_json::Value::Array(items) => items.iter().map(|v| v.as_f64().unwrap()).collect(),
            other => vec![other.as_f64().unwrap()],
        }
    }

    #[test]
    fn group_tests_match_r() {
        let reference = reference();
        for case in reference["tests"].as_array().unwrap() {
            let groups: Vec<Vec<f64>> = case["groups"]
                .as_array()
                .unwrap()
                .iter()
                .map(numbers)
                .collect();
            let slices: Vec<&[f64]> = groups.iter().map(Vec::as_slice).collect();
            let method = Method::parse(case["method"].as_str().unwrap()).unwrap();
            let paired = case["paired"].as_bool().unwrap();
            let p = method.p_value(&slices, paired).unwrap();
            let expected = case["p"].as_f64().unwrap();
            assert!(close(p, expected), "{}: {p} vs R {expected}", case["name"]);
        }
    }

    #[test]
    fn correlation_tests_match_r() {
        let reference = reference();
        for case in reference["correlations"].as_array().unwrap() {
            let method = Correlation::parse(case["method"].as_str().unwrap()).unwrap();
            let (estimate, p) = method
                .test(&numbers(&case["x"]), &numbers(&case["y"]))
                .unwrap();
            let (r_estimate, r_p) = (
                case["estimate"].as_f64().unwrap(),
                case["p"].as_f64().unwrap(),
            );
            assert!(
                close(estimate, r_estimate),
                "{}: estimate {estimate} vs R {r_estimate}",
                case["name"]
            );
            assert!(close(p, r_p), "{}: p {p} vs R {r_p}", case["name"]);
        }
    }

    #[test]
    fn adjustments_and_p_formats_match_r() {
        let reference = reference();
        let p = numbers(&reference["adjust"]["p"]);
        for (name, expected) in reference["adjust"]["methods"].as_object().unwrap() {
            let adjusted = Adjustment::parse(name).unwrap().adjust(&p);
            for (a, e) in adjusted.iter().zip(numbers(expected)) {
                assert!(close(*a, e), "{name}: {adjusted:?}");
            }
        }
        let formats = &reference["format_pval"];
        for (p, text) in numbers(&formats["p"])
            .iter()
            .zip(formats["text"].as_array().unwrap())
        {
            assert_eq!(
                super::super::pubr_labels::p_format(*p),
                text.as_str().unwrap(),
                "{p}"
            );
        }
    }
}

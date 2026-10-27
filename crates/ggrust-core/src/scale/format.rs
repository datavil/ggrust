//! Tick label formatting for numeric breaks (port of lets-plot
//! `NumericBreakFormatter` + `NumberFormat` type `g`, with thousands commas and
//! trimmed zeros). Exponents use lets-plot's default "power" style: the label is
//! `2·10` with superscript `6` (`−9` for negative exponents, with U+2212 as lets-plot).

/// lets-plot `NumericBreakFormatter.DEF_MIN_EXP`.
const MIN_EXPONENT: i32 = -7;
/// lets-plot `NumericBreakFormatter.DEF_MAX_EXP`.
const MAX_EXPONENT: i32 = 6;

/// A tick label: text, plus an exponent drawn as a superscript after it, plus
/// text after the exponent (`5.0·10` + `8` + ` m`).
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Label {
    /// Main text, e.g. `2·10` or `1,500`.
    pub text: String,
    /// Superscript exponent, e.g. `6` or `−9`.
    pub superscript: Option<String>,
    /// Text after the superscript, at the base size (`format="{.1e} m"`).
    pub suffix: Option<String>,
}

impl Label {
    /// A label without superscript.
    pub fn plain(text: impl Into<String>) -> Self {
        Label {
            text: text.into(),
            superscript: None,
            suffix: None,
        }
    }
}

impl std::fmt::Display for Label {
    /// `text`, `text^superscript` or `text^superscript suffix`.
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        formatter.write_str(&self.text)?;
        if let Some(superscript) = &self.superscript {
            write!(formatter, "^{superscript}")?;
        }
        formatter.write_str(self.suffix.as_deref().unwrap_or(""))
    }
}

/// Labels for a list of breaks, all formatted with one precision chosen from the
/// largest magnitude and the step between breaks.
pub fn format_breaks(breaks: &[f64]) -> Vec<Label> {
    let precision = break_precision(breaks);
    breaks
        .iter()
        .map(|&value| format_general(value, precision))
        .collect()
}

/// Labels for unevenly spaced breaks (transformed scales): each break formatted
/// with the step to its neighbour (lets-plot `NonlinearBreaksHelper.createMultiFormatter`).
pub fn format_breaks_individually(breaks: &[f64]) -> Vec<Label> {
    breaks
        .iter()
        .enumerate()
        .map(|(index, &value)| {
            let step = match (breaks.len(), index) {
                (1, _) => value / 10.0,
                (_, 0) => value - breaks[1],
                _ => value - breaks[index - 1],
            };
            format_general(value, precision(value.abs(), step.abs()))
        })
        .collect()
}

/// Significant digits needed to tell the breaks apart (lets-plot logic).
fn break_precision(breaks: &[f64]) -> i32 {
    let (reference, step) = match breaks {
        [] => (0.0, 0.5),
        [only] => (only.abs(), only.abs() / 10.0),
        [first, second, ..] => {
            let last = breaks[breaks.len() - 1];
            (first.abs().max(last.abs()), (second - first).abs())
        }
    };
    precision(reference, step)
}

/// lets-plot `NumericBreakFormatter`: significant digits for a value of magnitude
/// `reference` next to breaks `step` apart.
fn precision(reference: f64, step: f64) -> i32 {
    let reference = if reference == 0.0 {
        f64::MIN_POSITIVE * 10.0
    } else {
        reference
    };
    let step = if step == 0.0 { reference / 10.0 } else { step };
    let domain_power = reference.log10();
    let step_power = step.log10();
    // Tiny values with tiny steps, or huge values with large steps: scientific
    // notation, one digit per order of magnitude between value and step.
    let scientific = (domain_power < 0.0 && step_power <= f64::from(MIN_EXPONENT))
        || (domain_power >= f64::from(MAX_EXPONENT) && step_power > 2.0);
    let precision = if scientific {
        domain_power - step_power + 1.0
    } else if step_power > 0.0 {
        domain_power.ceil()
    } else {
        domain_power.ceil() - step_power
    };
    (precision - 0.001).ceil() as i32
}

/// `g` formatting: `precision` significant digits, decimal notation when the
/// exponent is within (MIN_EXPONENT, MAX_EXPONENT), else `m·10ⁿ`.
fn format_general(value: f64, precision: i32) -> Label {
    if !value.is_finite() {
        return Label::plain(value.to_string());
    }
    let (mantissa_digits, exponent) = rounded_digits(value, (precision - 1).max(0) as usize);
    if mantissa_digits.bytes().all(|byte| byte == b'0') {
        return Label::plain("0");
    }
    let mantissa = match mantissa_digits.split_at(1) {
        (first, "") => first.to_owned(),
        (first, rest) => format!("{first}.{rest}"),
    };
    let sign = if value < 0.0 { "-" } else { "" };
    if exponent > MIN_EXPONENT && exponent < MAX_EXPONENT {
        Label::plain(format!(
            "{sign}{}",
            decimal_from_digits(&mantissa_digits, exponent)
        ))
    } else {
        let mantissa = trim_fraction(mantissa.trim_start_matches('-'));
        let text = match mantissa.as_str() {
            "1" => format!("{sign}10"),
            _ => format!("{sign}{mantissa}·10"),
        };
        Label {
            text,
            superscript: Some(exponent.to_string().replace('-', "\u{2212}")),
            suffix: None,
        }
    }
}

/// Significant digits and decimal exponent of `|value|` as lets-plot's `BigFloat`
/// sees them: the shortest round-trip digits (Kotlin `Double.toString`), rounded
/// half-up to one digit plus `fraction_digits` (so 0.125 at 2 digits is 0.13).
fn rounded_digits(value: f64, fraction_digits: usize) -> (String, i32) {
    let shortest = format!("{:e}", value.abs());
    let (mantissa, exponent) = shortest.split_once('e').unwrap_or((&shortest, "0"));
    let mut exponent: i32 = exponent.parse().unwrap_or(0);
    let mut digits: Vec<u8> = mantissa.bytes().filter(u8::is_ascii_digit).collect();
    let keep = 1 + fraction_digits;
    if digits.len() > keep {
        let round_up = digits[keep] >= b'5';
        digits.truncate(keep);
        if round_up {
            let mut index = keep;
            loop {
                if index == 0 {
                    // 9.99 -> 10.0: one more integer digit.
                    digits.insert(0, b'1');
                    digits.pop();
                    exponent += 1;
                    break;
                }
                index -= 1;
                if digits[index] == b'9' {
                    digits[index] = b'0';
                } else {
                    digits[index] += 1;
                    break;
                }
            }
        }
    }
    (String::from_utf8(digits).unwrap_or_default(), exponent)
}

/// Place the decimal point into significant `digits` for a number `d.ddd x 10^exponent`,
/// group the integer part with commas, and trim trailing fraction zeros.
fn decimal_from_digits(digits: &str, exponent: i32) -> String {
    let point = exponent + 1; // digits before the decimal point
    let (integer, fraction) = if point <= 0 {
        (
            "0".to_owned(),
            format!("{}{digits}", "0".repeat((-point) as usize)),
        )
    } else if point as usize >= digits.len() {
        (
            format!("{digits}{}", "0".repeat(point as usize - digits.len())),
            String::new(),
        )
    } else {
        let (integer, fraction) = digits.split_at(point as usize);
        (integer.to_owned(), fraction.to_owned())
    };
    let fraction = fraction.trim_end_matches('0');
    let integer = group_thousands(&integer);
    if fraction.is_empty() {
        integer
    } else {
        format!("{integer}.{fraction}")
    }
}

fn group_thousands(integer: &str) -> String {
    let mut grouped = String::with_capacity(integer.len() + integer.len() / 3);
    for (index, character) in integer.chars().enumerate() {
        if index > 0 && (integer.len() - index) % 3 == 0 {
            grouped.push(',');
        }
        grouped.push(character);
    }
    grouped
}

fn trim_fraction(mantissa: &str) -> String {
    match mantissa.split_once('.') {
        Some((integer, fraction)) => {
            let fraction = fraction.trim_end_matches('0');
            if fraction.is_empty() {
                integer.to_owned()
            } else {
                format!("{integer}.{fraction}")
            }
        }
        None => mantissa.to_owned(),
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Labels as `text^superscript` strings.
    fn labels(breaks: &[f64]) -> Vec<String> {
        format_breaks(breaks).iter().map(Label::to_string).collect()
    }

    #[test]
    fn matches_lets_plot_axis_labels() {
        // Labels taken from lets-plot 4.11 `to_svg()` output for the same breaks.
        assert_eq!(
            labels(&[1.0, 1.5, 2.0, 2.5, 3.0, 3.5, 4.0, 4.5]),
            ["1", "1.5", "2", "2.5", "3", "3.5", "4", "4.5"]
        );
        assert_eq!(
            labels(&[0.0, 2000.0, 4000.0, 6000.0, 8000.0, 10000.0, 12000.0]),
            ["0", "2,000", "4,000", "6,000", "8,000", "10,000", "12,000"]
        );
        assert_eq!(
            labels(&[1000.0, 1500.0, 2000.0]),
            ["1,000", "1,500", "2,000"]
        );
        assert_eq!(
            labels(&[-3.0, -2.0, -1.0, 0.0, 1.0]),
            ["-3", "-2", "-1", "0", "1"]
        );
        assert_eq!(
            labels(&[0.001, 0.0012, 0.0014, 0.0016, 0.0018, 0.002]),
            ["0.001", "0.0012", "0.0014", "0.0016", "0.0018", "0.002"]
        );
        assert_eq!(
            labels(&[0.0, 0.000002, 0.000004, 0.00001]),
            ["0", "0.000002", "0.000004", "0.00001"]
        );
        assert_eq!(
            labels(&[0.0, 2e6, 4e6, 6e6, 8e6, 1e7]),
            ["0", "2·10^6", "4·10^6", "6·10^6", "8·10^6", "10^7"]
        );
    }

    #[test]
    fn noisy_breaks_are_formatted_cleanly() {
        assert_eq!(
            labels(&[0.0, 0.2, 0.4, 0.6000000000000001, 0.8, 0.9999999999998934]),
            ["0", "0.2", "0.4", "0.6", "0.8", "1"]
        );
    }

    #[test]
    fn negative_exponents_use_the_minus_sign() {
        assert_eq!(
            labels(&[2e-17, 4e-17, 6e-17]),
            ["2·10^\u{2212}17", "4·10^\u{2212}17", "6·10^\u{2212}17"]
        );
        assert_eq!(labels(&[1e-9, 2e-9])[0], "10^\u{2212}9");
    }

    #[test]
    fn single_and_empty() {
        assert_eq!(labels(&[5.0]), ["5"]);
        assert_eq!(labels(&[0.0]), ["0"]);
        assert!(labels(&[]).is_empty());
    }
}

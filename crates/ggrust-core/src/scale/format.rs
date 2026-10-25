//! Tick label formatting for numeric breaks (port of lets-plot
//! `NumericBreakFormatter` + `NumberFormat` type `g`, with thousands commas and
//! trimmed zeros). Exponents use lets-plot's default "power" style, written with
//! Unicode superscripts: `2·10⁶`, `10⁷`.

/// lets-plot `NumericBreakFormatter.DEF_MIN_EXP`.
const MIN_EXPONENT: i32 = -7;
/// lets-plot `NumericBreakFormatter.DEF_MAX_EXP`.
const MAX_EXPONENT: i32 = 6;

/// Labels for a list of breaks, all formatted with one precision chosen from the
/// largest magnitude and the step between breaks.
pub fn format_breaks(breaks: &[f64]) -> Vec<String> {
    let precision = break_precision(breaks);
    breaks
        .iter()
        .map(|&value| format_general(value, precision))
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
fn format_general(value: f64, precision: i32) -> String {
    if !value.is_finite() {
        return value.to_string();
    }
    let digits = (precision - 1).clamp(0, 16) as usize;
    // Rust's exponent formatting rounds the exact decimal expansion correctly.
    let scientific = format!("{value:.digits$e}");
    let (mantissa, exponent) = scientific.split_once('e').unwrap_or((&scientific, "0"));
    let exponent: i32 = exponent.parse().unwrap_or(0);
    let negative = mantissa.starts_with('-');
    let mantissa_digits: String = mantissa.chars().filter(char::is_ascii_digit).collect();
    if mantissa_digits.bytes().all(|byte| byte == b'0') {
        return "0".into();
    }
    let sign = if negative { "-" } else { "" };
    if exponent > MIN_EXPONENT && exponent < MAX_EXPONENT {
        format!("{sign}{}", decimal_from_digits(&mantissa_digits, exponent))
    } else {
        let mantissa = trim_fraction(mantissa.trim_start_matches('-'));
        let power = superscript(exponent);
        match mantissa.as_str() {
            "1" => format!("{sign}10{power}"),
            _ => format!("{sign}{mantissa}·10{power}"),
        }
    }
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

fn superscript(exponent: i32) -> String {
    exponent
        .to_string()
        .chars()
        .map(|character| match character {
            '-' => '⁻',
            '0' => '⁰',
            '1' => '¹',
            '2' => '²',
            '3' => '³',
            '4' => '⁴',
            '5' => '⁵',
            '6' => '⁶',
            '7' => '⁷',
            '8' => '⁸',
            _ => '⁹',
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn labels(breaks: &[f64]) -> Vec<String> {
        format_breaks(breaks)
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
            ["0", "2·10⁶", "4·10⁶", "6·10⁶", "8·10⁶", "10⁷"]
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
    fn single_and_empty() {
        assert_eq!(labels(&[5.0]), ["5"]);
        assert_eq!(labels(&[0.0]), ["0"]);
        assert!(labels(&[]).is_empty());
    }
}

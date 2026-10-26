//! lets-plot's number format (`NumberFormat`, a d3-format port): patterns like `.2f`,
//! `,d`, `+$,.2f`, `~s`, `e`, `%`, with lets-plot's exponent styles (`&E`, `&P`,
//! `&F`) and limits (`{min,max}`).
//!
//! Numbers are formatted from their shortest decimal digits (as lets-plot's
//! `BigFloat`), so rounding is decimal: `0.445` with `.2f` is `0.45`. Powers come
//! out as lets-plot's markup `·\(10^{n}\)`; turning that into a superscript is the
//! caller's job. Conformance vectors come from lets-plot's own tests.
//!
//! What it does not do: datetime patterns, locales.

/// lets-plot `NumberFormat.DEF_MIN_EXP`.
const DEFAULT_MIN_EXPONENT: i32 = -7;
/// lets-plot `NumberFormat.DEF_PRECISION`.
const DEFAULT_PRECISION: i32 = 6;
/// lets-plot `NumberFormat.MULT_SIGN`.
const MULTIPLY: &str = "·";

/// How exponents are written (`&E`, `&P`, `&F`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum ExponentStyle {
    /// `1e+6`.
    E,
    /// `\(10^{6}\)`, `2·\(10^{6}\)`.
    Power,
    /// `1·\(10^{6}\)`.
    PowerFull,
}

/// A parsed pattern (lets-plot `NumberFormat.Spec`, normalized).
#[derive(Debug, Clone, PartialEq)]
pub struct NumberFormat {
    fill: char,
    align: char,
    sign: char,
    symbol: Option<char>,
    zero: bool,
    width: Option<usize>,
    comma: bool,
    precision: i32,
    kind: char,
    trim: bool,
    /// Exponent style.
    pub exponent_style: ExponentStyle,
    /// Smallest exponent written as a plain number (exclusive).
    pub min_exponent: i32,
    /// Largest exponent written as a plain number (exclusive).
    pub max_exponent: i32,
}

impl NumberFormat {
    /// Parse a pattern; `None` if it is not a number format (lets-plot
    /// `NUMBER_REGEX`): `[[fill]align][sign][symbol][0][width][,][.precision][~][type]
    /// [&exponent style][{min,max}]`.
    pub fn parse(pattern: &str) -> Option<NumberFormat> {
        let characters: Vec<char> = pattern.chars().collect();
        let mut at = 0;
        let is_align = |c: char| matches!(c, '<' | '>' | '=' | '^');
        let (mut fill, mut align) = (' ', '>');
        if characters.len() >= 2 && is_align(characters[1]) && !matches!(characters[0], '{' | '}') {
            fill = characters[0];
            align = characters[1];
            at = 2;
        } else if characters.first().copied().is_some_and(is_align) {
            align = characters[0];
            at = 1;
        }
        let mut take = |accept: &dyn Fn(char) -> bool| -> Option<char> {
            let next = *characters.get(at)?;
            accept(next).then(|| {
                at += 1;
                next
            })
        };
        let sign = take(&|c| matches!(c, '+' | ' ' | '-')).unwrap_or('-');
        let symbol = take(&|c| matches!(c, '#' | '$'));
        let zero = take(&|c| c == '0').is_some();
        let mut digits = String::new();
        while let Some(digit) = take(&|c| c.is_ascii_digit()) {
            digits.push(digit);
        }
        let width = if digits.is_empty() {
            None
        } else {
            Some(digits.parse().ok()?)
        };
        let comma = take(&|c| c == ',').is_some();
        let mut precision = None;
        if take(&|c| c == '.').is_some() {
            let mut digits = String::new();
            while let Some(digit) = take(&|c| c.is_ascii_digit()) {
                digits.push(digit);
            }
            precision = Some(digits.parse::<i32>().ok()?);
        }
        let trim = take(&|c| c == '~').is_some();
        let kind = take(&|c| "%bcdefgosXx".contains(c));
        let mut exponent_style = ExponentStyle::E;
        if take(&|c| c == '&').is_some() {
            exponent_style = match take(&|c| matches!(c, 'E' | 'P' | 'F'))? {
                'P' => ExponentStyle::Power,
                'F' => ExponentStyle::PowerFull,
                _ => ExponentStyle::E,
            };
        }
        let (mut min_exponent, mut max_exponent) = (None, None);
        if take(&|c| c == '{').is_some() {
            let rest: String = characters[at..].iter().collect();
            let inner = rest.strip_suffix('}')?;
            let (low, high) = inner.split_once(',')?;
            let number = |text: &str| -> Option<Option<i32>> {
                if text.is_empty() {
                    Some(None)
                } else {
                    text.parse().ok().map(Some)
                }
            };
            min_exponent = number(low)?;
            max_exponent = number(high)?;
            at = characters.len();
        }
        if at != characters.len() {
            return None;
        }
        let precision = precision.unwrap_or(DEFAULT_PRECISION);
        let mut format = NumberFormat {
            fill,
            align,
            sign,
            symbol,
            zero,
            width,
            comma,
            precision,
            kind: kind.unwrap_or(' '),
            trim,
            exponent_style,
            min_exponent: min_exponent.unwrap_or(DEFAULT_MIN_EXPONENT),
            max_exponent: max_exponent.unwrap_or(precision),
        };
        // lets-plot `normalizeSpec`.
        if kind.is_none() {
            format.kind = 'g';
            format.trim = true;
        }
        if format.zero || (format.fill == '0' && format.align == '=') {
            format.zero = true;
            format.fill = '0';
            format.align = '=';
        }
        Some(format)
    }

    /// Format a number (lets-plot `NumberFormat.apply`).
    pub fn apply(&self, value: f64) -> String {
        if value.is_nan() {
            return "NaN".to_owned();
        }
        if value.is_infinite() {
            return if value > 0.0 {
                "+Infinity"
            } else {
                "-Infinity"
            }
            .to_owned();
        }
        let number = Decimal::from_f64(value);
        let mut body = self.body(&number);
        if self.trim && !body.fraction.is_empty() {
            body.fraction = body.fraction.trim_end_matches('0').to_owned();
        }
        let is_zero = body
            .integer
            .chars()
            .chain(body.fraction.chars())
            .all(|c| c == '0');
        let sign = if number.negative && !is_zero {
            "-".to_owned()
        } else if self.sign != '-' {
            self.sign.to_string()
        } else {
            String::new()
        };
        let prefix = match self.symbol {
            Some('$') => "$".to_owned(),
            Some('#') if "boxX".contains(self.kind) => {
                format!("0{}", self.kind.to_ascii_lowercase())
            }
            _ => String::new(),
        };
        let suffix = if self.kind == '%' { "%" } else { "" };
        if self.comma && !self.zero {
            self.group(&mut body, "");
        }
        let length = sign.chars().count()
            + prefix.chars().count()
            + body.full_length()
            + suffix.chars().count();
        let mut padding = match self.width {
            Some(width) if length < width => self.fill.to_string().repeat(width - length),
            _ => String::new(),
        };
        if self.comma && self.zero {
            self.group(&mut body, &padding);
            padding.clear();
        }
        let body = body.to_string();
        match self.align {
            '<' => format!("{sign}{prefix}{body}{suffix}{padding}"),
            '=' => format!("{sign}{prefix}{padding}{body}{suffix}"),
            '^' => {
                let characters: Vec<char> = padding.chars().collect();
                let (left, right) = characters.split_at(characters.len() / 2);
                let left: String = left.iter().collect();
                let right: String = right.iter().collect();
                format!("{left}{sign}{prefix}{body}{suffix}{right}")
            }
            _ => format!("{padding}{sign}{prefix}{body}{suffix}"),
        }
    }

    fn body(&self, number: &Decimal) -> Formatted {
        let style = self.exponent_style;
        let (min, max) = (self.min_exponent, self.max_exponent);
        let whole = |text: String| Formatted::plain(text, String::new(), style);
        let radix = |radix: u32, upper: bool| {
            let digits = number.whole_part();
            match digits.parse::<u128>() {
                Ok(value) => {
                    let text = to_radix(value, radix);
                    whole(if upper { text.to_uppercase() } else { text })
                }
                Err(_) => whole(digits),
            }
        };
        match self.kind {
            'e' => exponent_notation(number, self.precision, min, max, style),
            'f' => decimal_notation(number, self.precision, style),
            'g' => general_notation(number, self.precision, min, max, style),
            's' => si_notation(number, self.precision, style),
            '%' => decimal_notation(&number.shifted(2), self.precision, style),
            'd' => whole(number.to_decimal_precision(0).whole_part()),
            'c' => whole(number.whole_part()),
            'b' => radix(2, false),
            'o' => radix(8, false),
            'X' => radix(16, true),
            _ => radix(16, false),
        }
    }

    /// Thousands separators (lets-plot `applyGroup`), with zero padding folded in.
    fn group(&self, body: &mut Formatted, zero_padding: &str) {
        let full = format!("{zero_padding}{}", body.integer);
        let digits = full.chars().count();
        let commas = digits.div_ceil(3).saturating_sub(1);
        let width = self
            .width
            .map_or(i64::MIN, |width| {
                width as i64 - body.fraction_length() as i64 - body.exponent_length() as i64
            })
            .max((body.integer.chars().count() + commas) as i64) as usize;
        let mut grouped = group_thousands(&full);
        let length = grouped.chars().count();
        if length > width {
            grouped = grouped.chars().skip(length - width).collect();
            if grouped.starts_with(',') {
                grouped.insert(0, '0');
            }
        }
        body.integer = grouped;
    }
}

fn group_thousands(digits: &str) -> String {
    let characters: Vec<char> = digits.chars().collect();
    let mut grouped = String::with_capacity(characters.len() + characters.len() / 3);
    for (index, character) in characters.iter().enumerate() {
        if index > 0 && (characters.len() - index) % 3 == 0 {
            grouped.push(',');
        }
        grouped.push(*character);
    }
    grouped
}

fn to_radix(mut value: u128, radix: u32) -> String {
    if value == 0 {
        return "0".to_owned();
    }
    let mut digits = Vec::new();
    while value > 0 {
        let digit = (value % u128::from(radix)) as u32;
        digits.push(char::from_digit(digit, radix).unwrap_or('?'));
        value /= u128::from(radix);
    }
    digits.iter().rev().collect()
}

/// lets-plot `FormattedNumber`.
#[derive(Debug, Clone)]
struct Formatted {
    integer: String,
    fraction: String,
    exponent: String,
    style: ExponentStyle,
}

impl Formatted {
    fn plain(integer: String, fraction: String, style: ExponentStyle) -> Self {
        Formatted {
            integer,
            fraction,
            exponent: String::new(),
            style,
        }
    }

    /// `1·\(10^{n}\)` is written `\(10^{n}\)` in the power style.
    fn omits_unit(&self) -> bool {
        self.style == ExponentStyle::Power
            && self.integer == "1"
            && self.fraction.is_empty()
            && !self.exponent.is_empty()
    }

    fn fraction_length(&self) -> usize {
        if self.fraction.is_empty() {
            0
        } else {
            self.fraction.chars().count() + 1
        }
    }

    /// Visible length: a power counts as `10` plus its digits (and `·`).
    fn exponent_length(&self) -> usize {
        let degree = self
            .exponent
            .strip_prefix(MULTIPLY)
            .and_then(|rest| rest.strip_prefix("\\(10^{"))
            .and_then(|rest| rest.strip_suffix("}\\)"));
        match degree {
            Some(degree) => 2 + degree.chars().count() + usize::from(!self.omits_unit()),
            None => self.exponent.chars().count(),
        }
    }

    fn full_length(&self) -> usize {
        let integer = if self.omits_unit() {
            0
        } else {
            self.integer.chars().count()
        };
        integer + self.fraction_length() + self.exponent_length()
    }
}

impl std::fmt::Display for Formatted {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let point = if self.fraction.is_empty() { "" } else { "." };
        let full = format!("{}{point}{}{}", self.integer, self.fraction, self.exponent);
        if self.omits_unit() {
            formatter.write_str(&full.replacen(&format!("1{MULTIPLY}"), "", 1))
        } else {
            formatter.write_str(&full)
        }
    }
}

fn exponent_text(exponent: i32, min: i32, max: i32, style: ExponentStyle) -> String {
    match style {
        ExponentStyle::E => format!("e{}{exponent}", if exponent >= 0 { "+" } else { "" }),
        ExponentStyle::Power | ExponentStyle::PowerFull => {
            if exponent == 0 && min < 0 && max > 0 {
                String::new()
            } else if exponent == 1 && min < 1 && max > 1 {
                format!("{MULTIPLY}10")
            } else {
                format!("{MULTIPLY}\\(10^{{{exponent}}}\\)")
            }
        }
    }
}

/// lets-plot `FormatNotationUtil.formatExponentNotation`.
fn exponent_notation(
    number: &Decimal,
    precision: i32,
    min: i32,
    max: i32,
    style: ExponentStyle,
) -> Formatted {
    if precision > -1 {
        let rounded = number.to_precision(precision as usize);
        let (significand, fraction) = rounded.scientific(Some(precision as usize));
        return Formatted {
            integer: significand,
            fraction,
            exponent: exponent_text(rounded.exponent, min, max, style),
            style,
        };
    }
    if number.is_zero() {
        return Formatted::plain("0".to_owned(), String::new(), style);
    }
    let (significand, fraction) = number.scientific(None);
    Formatted {
        integer: significand,
        fraction: if fraction == "0" {
            String::new()
        } else {
            fraction
        },
        exponent: exponent_text(number.exponent, min, max, style),
        style,
    }
}

/// lets-plot `FormatNotationUtil.formatDecimalNotation`.
fn decimal_notation(number: &Decimal, precision: i32, style: ExponentStyle) -> Formatted {
    let length = precision.max(0) as usize;
    let (integer, fraction) = number.to_decimal_precision(precision).decimal(Some(length));
    let fraction = if precision <= 0 {
        String::new()
    } else {
        fraction
    };
    Formatted::plain(integer, fraction, style)
}

/// lets-plot `FormatNotationUtil.formatGeneralNotation`.
fn general_notation(
    number: &Decimal,
    precision: i32,
    min: i32,
    max: i32,
    style: ExponentStyle,
) -> Formatted {
    let max = if precision == 0 && max == 0 { 1 } else { max };
    let significant = (precision - 1).max(0);
    let rounded = number.to_precision(significant as usize);
    if rounded.exponent > min && rounded.exponent < max {
        let length = significant - rounded.exponent;
        let (integer, fraction) = rounded.decimal((length >= 0).then_some(length as usize));
        Formatted::plain(integer, fraction, style)
    } else {
        exponent_notation(&rounded, significant, min, max, style)
    }
}

/// SI prefixes by exponent range start (lets-plot `SiPrefix`).
const SI_PREFIXES: [(&str, i32); 17] = [
    ("Y", 24),
    ("Z", 21),
    ("E", 18),
    ("P", 15),
    ("T", 12),
    ("G", 9),
    ("M", 6),
    ("k", 3),
    ("", 0),
    ("m", -3),
    ("µ", -6),
    ("n", -9),
    ("p", -12),
    ("f", -15),
    ("a", -18),
    ("z", -21),
    ("y", -24),
];

/// lets-plot `FormatNotationUtil.formatSiNotation`.
fn si_notation(number: &Decimal, precision: i32, style: ExponentStyle) -> Formatted {
    let significant = (precision - 1).max(0);
    let rounded = number.to_precision(significant as usize);
    let (symbol, base) = SI_PREFIXES
        .iter()
        .find(|(_, base)| (*base..base + 3).contains(&rounded.exponent))
        .copied()
        .unwrap_or(if rounded.exponent < 0 {
            ("y", -24)
        } else {
            ("Y", 24)
        });
    let scaled = rounded.shifted(-base);
    let decimals = if scaled.exponent >= 0 {
        significant - scaled.exponent
    } else {
        significant
    };
    let mut formatted = decimal_notation(&scaled, decimals, style);
    formatted.exponent = symbol.to_owned();
    formatted
}

/// lets-plot `BigFloat`: one significand digit, fraction digits (no trailing zeros,
/// `"0"` when none) and a decimal exponent.
#[derive(Debug, Clone, PartialEq)]
struct Decimal {
    significand: u8,
    fraction: String,
    exponent: i32,
    negative: bool,
}

impl Decimal {
    fn new(significand: u8, fraction: &str, exponent: i32, negative: bool) -> Self {
        let trimmed = fraction.trim_end_matches('0');
        let fraction = if trimmed.is_empty() { "0" } else { trimmed };
        let zero = significand == 0 && fraction == "0";
        Decimal {
            significand,
            fraction: fraction.to_owned(),
            exponent: if zero { 0 } else { exponent },
            negative,
        }
    }

    /// From the shortest digits that round-trip (as lets-plot's `toString()`).
    fn from_f64(value: f64) -> Self {
        if value == 0.0 {
            return Decimal::new(0, "0", 0, false);
        }
        let text = format!("{:e}", value.abs());
        let (mantissa, exponent) = text.split_once('e').unwrap_or((&text, "0"));
        let (significand, fraction) = mantissa.split_once('.').unwrap_or((mantissa, ""));
        Decimal::new(
            significand.parse().unwrap_or(0),
            fraction,
            exponent.parse().unwrap_or(0),
            value < 0.0,
        )
    }

    fn is_zero(&self) -> bool {
        self.significand == 0 && self.fraction == "0"
    }

    /// Digits after the decimal point.
    fn decimal_part(&self) -> String {
        let exponent = self.exponent;
        if exponent == 0 {
            self.fraction.clone()
        } else if exponent < 0 {
            let fraction = if self.fraction == "0" {
                ""
            } else {
                &self.fraction
            };
            format!(
                "{}{}{fraction}",
                "0".repeat((-exponent - 1) as usize),
                self.significand
            )
        } else if self.fraction == "0" {
            "0".to_owned()
        } else {
            let rest: String = self.fraction.chars().skip(exponent as usize).collect();
            if rest.is_empty() {
                "0".to_owned()
            } else {
                rest
            }
        }
    }

    /// Digits before the decimal point.
    fn whole_part(&self) -> String {
        if self.exponent < 0 {
            return "0".to_owned();
        }
        let exponent = self.exponent as usize;
        let mut whole = self.significand.to_string();
        whole.extend(self.fraction.chars().take(exponent));
        while whole.len() < exponent + 1 {
            whole.push('0');
        }
        whole
    }

    /// Whole part and `length` decimals (all of them when `None`).
    fn decimal(&self, length: Option<usize>) -> (String, String) {
        let decimals = self.decimal_part();
        let decimals = match length {
            None => decimals,
            Some(length) => pad_to(decimals.chars().take(length).collect(), length),
        };
        (self.whole_part(), decimals)
    }

    /// Significand and `length` fraction digits (all of them when `None`).
    fn scientific(&self, length: Option<usize>) -> (String, String) {
        let fraction = match length {
            None => self.fraction.clone(),
            Some(length) => pad_to(self.fraction.chars().take(length).collect(), length),
        };
        (self.significand.to_string(), fraction)
    }

    fn to_decimal_precision(&self, precision: i32) -> Decimal {
        self.to_precision((precision + self.exponent).max(0) as usize)
    }

    /// Rounded to `precision` fraction digits (half up on the decimal digits).
    fn to_precision(&self, precision: usize) -> Decimal {
        if precision > self.fraction.len() {
            return self.clone();
        }
        let (rounded, carry) = round_digits(&self.fraction, precision);
        if !carry {
            Decimal::new(self.significand, &rounded, self.exponent, self.negative)
        } else if self.significand == 9 {
            Decimal::new(1, &format!("0{rounded}"), self.exponent + 1, self.negative)
        } else {
            Decimal::new(self.significand + 1, &rounded, self.exponent, self.negative)
        }
    }

    fn shifted(&self, delta: i32) -> Decimal {
        Decimal::new(
            self.significand,
            &self.fraction,
            self.exponent + delta,
            self.negative,
        )
    }
}

fn pad_to(mut text: String, length: usize) -> String {
    while text.chars().count() < length {
        text.push('0');
    }
    text
}

/// lets-plot `Arithmetic.round`: the first `precision` digits, rounded on the next
/// one; the flag says the digits overflowed (a carry into the significand).
fn round_digits(digits: &str, precision: usize) -> (String, bool) {
    let (kept, rest) = digits.split_at(precision.min(digits.len()));
    let round_up = rest.chars().next().is_some_and(|digit| digit >= '5');
    if kept.is_empty() {
        return (String::new(), round_up);
    }
    if !round_up {
        return (kept.to_owned(), false);
    }
    let mut result: Vec<u8> = kept.bytes().map(|digit| digit - b'0').collect();
    for digit in result.iter_mut().rev() {
        if *digit == 9 {
            *digit = 0;
        } else {
            *digit += 1;
            return (
                result.iter().map(|digit| (b'0' + digit) as char).collect(),
                false,
            );
        }
    }
    (
        result.iter().map(|digit| (b'0' + digit) as char).collect(),
        true,
    )
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn matches_lets_plot_test_vectors() {
        let vectors: Vec<serde_json::Value> =
            serde_json::from_str(include_str!("../../tests/fixtures/number_format.json"))
                .expect("valid fixture");
        let mut failures = Vec::new();
        for vector in &vectors {
            let spec = vector["spec"].as_str().unwrap_or_default();
            let value = match &vector["value"] {
                serde_json::Value::String(text) => text.parse::<f64>().unwrap_or(f64::NAN),
                number => number.as_f64().unwrap_or(f64::NAN),
            };
            let expected = vector["expected"].as_str().unwrap_or_default();
            let actual = NumberFormat::parse(spec).map(|format| format.apply(value));
            if actual.as_deref() != Some(expected) {
                failures.push(format!("{spec:?} {value:e}: {actual:?} != {expected:?}"));
            }
        }
        assert!(
            failures.is_empty(),
            "{} of {} failed:\n{}",
            failures.len(),
            vectors.len(),
            failures.join("\n")
        );
    }

    #[test]
    fn invalid_patterns_are_rejected() {
        for pattern in ["{d}", "abc", ".f.", "&Q", ".2f{1,2", "d cells"] {
            assert_eq!(NumberFormat::parse(pattern), None, "{pattern}");
        }
        assert!(NumberFormat::parse("").is_some());
    }

    #[test]
    fn rounding_is_decimal_half_up() {
        assert_eq!(round_digits("449", 2), ("45".to_owned(), false));
        assert_eq!(round_digits("99", 1), ("0".to_owned(), true));
        assert_eq!(round_digits("5", 0), (String::new(), true));
    }
}

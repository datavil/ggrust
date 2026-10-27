//! Label patterns from `scale_*(format=...)` (lets-plot `FormatterUtil.byPattern` and
//! `StringFormat`): a bare number format (`.2f`), a template with `{...}` fields
//! (`{.1f} %`, `{d} cells`, `{}`), or literal text. `{{` and `}}` are escaped braces.
//!
//! Numbers use [`NumberFormat`] with lets-plot's axis exponent style (`2·10` with a
//! superscript power). What it does not do: datetime patterns (`%Y-%m-%d`).

use super::format::Label;
use super::number_format::{ExponentStyle, NumberFormat};

/// A parsed label pattern.
#[derive(Debug, Clone, PartialEq)]
pub struct LabelPattern {
    parts: Vec<Part>,
}

#[derive(Debug, Clone, PartialEq)]
enum Part {
    /// Literal text (escapes resolved).
    Text(String),
    /// `{}`: the value as text.
    Value,
    /// `{.2f}`: the value as a formatted number.
    Number(NumberFormat),
    /// A field after the first: lets-plot formats one value, so later fields keep
    /// their placeholder text.
    Unfilled(String),
}

/// What a label pattern formats.
#[derive(Debug, Clone, Copy)]
pub enum PatternValue<'a> {
    /// A number (continuous scales).
    Number(f64),
    /// A category (discrete scales).
    Text(&'a str),
}

impl LabelPattern {
    /// Parse a pattern. Errors (as messages) name what is not supported.
    pub fn parse(pattern: &str) -> Result<LabelPattern, String> {
        if pattern.contains('{') && pattern.contains('}') {
            return Self::template(pattern);
        }
        if let Some(format) = NumberFormat::parse(pattern) {
            return Ok(LabelPattern {
                parts: vec![Part::Number(axis_style(format))],
            });
        }
        if is_datetime(pattern) {
            return Err(format!(
                "datetime formats like '{pattern}' are not supported yet"
            ));
        }
        Ok(LabelPattern {
            parts: vec![Part::Text(pattern.to_owned())],
        })
    }

    /// `{...}` fields among literal text (lets-plot `StringFormat.PLACEHOLDER_REGEX`):
    /// a field is `{` + no braces + `}`, not part of `{{` or followed by a lone `}`.
    fn template(pattern: &str) -> Result<LabelPattern, String> {
        let characters: Vec<char> = pattern.chars().collect();
        let mut parts = Vec::new();
        let mut text = String::new();
        let mut index = 0;
        let mut literal_start = 0;
        while index < characters.len() {
            let field_end = (characters[index] == '{' && characters.get(index + 1) != Some(&'{'))
                .then(|| {
                    let close = (index + 1..characters.len())
                        .find(|&at| matches!(characters[at], '{' | '}'))?;
                    (characters[close] == '}').then_some(close)
                })
                .flatten()
                .filter(|&close| match characters.get(close + 1) {
                    None => true,
                    Some('}') => characters.get(close + 2) == Some(&'}'),
                    Some(_) => true,
                });
            let Some(close) = field_end else {
                index += 1;
                continue;
            };
            text.push_str(&unescape(&characters[literal_start..index]));
            if !text.is_empty() {
                parts.push(Part::Text(std::mem::take(&mut text)));
            }
            let inner: String = characters[index + 1..close].iter().collect();
            let first = parts.iter().all(|part| matches!(part, Part::Text(_)));
            parts.push(if !first {
                Part::Unfilled(format!("{{{inner}}}"))
            } else if inner.is_empty() {
                Part::Value
            } else if let Some(format) = NumberFormat::parse(&inner) {
                Part::Number(axis_style(format))
            } else if is_datetime(&inner) {
                return Err(format!(
                    "datetime formats like '{{{inner}}}' are not supported yet"
                ));
            } else {
                Part::Text(format!("{{{inner}}}"))
            });
            index = close + 1;
            literal_start = index;
        }
        text.push_str(&unescape(&characters[literal_start..]));
        if !text.is_empty() {
            parts.push(Part::Text(text));
        }
        Ok(LabelPattern { parts })
    }

    /// The label of one value; a power written by a number format becomes the
    /// label's superscript when it ends the label.
    pub fn format(&self, value: PatternValue) -> Label {
        let mut text = String::new();
        for part in &self.parts {
            match (part, value) {
                (Part::Text(literal) | Part::Unfilled(literal), _) => text.push_str(literal),
                (Part::Value, PatternValue::Number(number)) => {
                    text.push_str(&kotlin_double(number))
                }
                (Part::Value, PatternValue::Text(category)) => text.push_str(category),
                (Part::Number(format), PatternValue::Number(number)) => {
                    text.push_str(&format.apply(number));
                }
                // lets-plot formats strings that parse as numbers, others as they are.
                (Part::Number(format), PatternValue::Text(category)) => {
                    match kotlin_to_double(category) {
                        Some(number) => text.push_str(&format.apply(number)),
                        None => text.push_str(category),
                    }
                }
            }
        }
        power_label(text)
    }
}

/// Axis labels write exponents as powers (lets-plot's default `exponent_format`).
fn axis_style(mut format: NumberFormat) -> NumberFormat {
    format.exponent_style = ExponentStyle::Power;
    format
}

fn unescape(characters: &[char]) -> String {
    let text: String = characters.iter().collect();
    text.replace("{{", "{").replace("}}", "}")
}

/// lets-plot's loose datetime check: one of its `%` directives (`%Y`, `%d`, ...).
fn is_datetime(pattern: &str) -> bool {
    const DIRECTIVES: &str = "aAbBdejmwyYHIlMpPSf";
    let mut characters = pattern.chars().peekable();
    while let Some(character) = characters.next() {
        if character == '%'
            && characters
                .peek()
                .is_some_and(|next| DIRECTIVES.contains(*next))
        {
            return true;
        }
    }
    false
}

/// Kotlin `String.toDoubleOrNull`: decimal numbers with an optional `f`/`d` suffix,
/// `NaN` and `Infinity` (signed); not Rust's `inf` or `nan` spellings.
fn kotlin_to_double(text: &str) -> Option<f64> {
    let text = text.trim();
    match text {
        "NaN" | "+NaN" | "-NaN" => return Some(f64::NAN),
        "Infinity" | "+Infinity" => return Some(f64::INFINITY),
        "-Infinity" => return Some(f64::NEG_INFINITY),
        _ => {}
    }
    let number = text.strip_suffix(['f', 'F', 'd', 'D']).unwrap_or(text);
    let valid = number.chars().any(|c| c.is_ascii_digit())
        && number
            .chars()
            .all(|c| c.is_ascii_digit() || matches!(c, '.' | 'e' | 'E' | '+' | '-'));
    if valid { number.parse().ok() } else { None }
}

/// `x·\(10^{n}\) rest` -> `x·10` with superscript `n` (minus as U+2212, as
/// lets-plot draws it) and `rest` after it; a second power stays as `10^n` text.
pub(crate) fn power_label(text: String) -> Label {
    const OPEN: &str = "\\(10^{";
    const CLOSE: &str = "}\\)";
    let Some(start) = text.find(OPEN) else {
        return Label::plain(text);
    };
    let after = &text[start + OPEN.len()..];
    let Some(end) = after.find(CLOSE) else {
        return Label::plain(text);
    };
    let degree = after[..end].replace('-', "\u{2212}");
    let rest = flatten_powers(&after[end + CLOSE.len()..]);
    Label {
        text: format!("{}10", &text[..start]),
        superscript: Some(degree),
        suffix: (!rest.is_empty()).then_some(rest),
    }
}

/// Further powers as plain `10^n` text (a label has one superscript).
fn flatten_powers(text: &str) -> String {
    text.replace("\\(10^{", "10^").replace("}\\)", "")
}

/// A double as Kotlin's `toString()` writes it (`2.0`, `1.5`, `1.0E7`).
fn kotlin_double(value: f64) -> String {
    if value.is_nan() {
        return "NaN".to_owned();
    }
    if value.is_infinite() {
        return if value > 0.0 { "Infinity" } else { "-Infinity" }.to_owned();
    }
    let magnitude = value.abs();
    if value == 0.0 || (1e-3..1e7).contains(&magnitude) {
        let text = format!("{value}");
        if text.contains('.') {
            text
        } else {
            format!("{text}.0")
        }
    } else {
        let text = format!("{value:e}");
        let (mantissa, exponent) = text.split_once('e').unwrap_or((&text, "0"));
        let mantissa = if mantissa.contains('.') {
            mantissa.to_owned()
        } else {
            format!("{mantissa}.0")
        };
        format!("{mantissa}E{exponent}")
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn label(pattern: &str, value: f64) -> String {
        LabelPattern::parse(pattern)
            .unwrap()
            .format(PatternValue::Number(value))
            .to_string()
    }

    #[test]
    fn patterns_match_lets_plot_axis_labels() {
        // Measured with lets-plot 4.11 `to_svg()`.
        assert_eq!(label(".2f", 1.5), "1.50");
        assert_eq!(label("{d} cells", 1.5), "2 cells");
        assert_eq!(label("e", 1.5), "1.500000");
        assert_eq!(label("%", 2.0), "200.000000%");
        assert_eq!(label("{} u", 2.0), "2.0 u");
        assert_eq!(label("abc", 2.0), "abc");
        assert_eq!(label("{.1f}%", 2.5), "2.5%");
        assert_eq!(label(".2e", 1.5e6), "1.50·10^6");
        assert_eq!(label("{.1e} m", 5e8), "5.0·10^8 m");
        let pattern = LabelPattern::parse("[{.1e}]").unwrap();
        let power = pattern.format(PatternValue::Number(5e8));
        assert_eq!(power.text, "[5.0·10");
        assert_eq!(power.superscript.as_deref(), Some("8"));
        assert_eq!(power.suffix.as_deref(), Some("]"));
        assert_eq!(label("g", 3e-9), "3.00000·10^\u{2212}9");
    }

    #[test]
    fn braces_escape_and_unknown_fields_stay() {
        assert_eq!(label("{{{.1f}}}", 1.25), "{1.3}");
        assert_eq!(label("{{x}} {.0f}", 2.0), "{x} 2");
        assert_eq!(label("{abc} {d}", 2.0), "{abc} 2");
    }

    #[test]
    fn categories_format_as_text_or_numbers() {
        let pattern = LabelPattern::parse("<{}>").unwrap();
        assert_eq!(pattern.format(PatternValue::Text("a")).to_string(), "<a>");
        let number = LabelPattern::parse(".1f").unwrap();
        assert_eq!(number.format(PatternValue::Text("2")).to_string(), "2.0");
        assert_eq!(number.format(PatternValue::Text("b")).to_string(), "b");
    }

    #[test]
    fn datetime_patterns_are_reported() {
        assert!(LabelPattern::parse("%Y-%m-%d").is_err());
        assert!(LabelPattern::parse("{%H:%M}").is_err());
    }

    #[test]
    fn only_the_first_field_takes_the_value() {
        assert_eq!(label("{.1f}{.2f}", -2.0), "-2.0{.2f}");
        assert_eq!(label("{}{}", -2.0), "-2.0{}");
        assert_eq!(label("{d} {%x}", -2.0), "-2 {%x}");
    }

    #[test]
    fn only_lets_plot_directives_are_datetime() {
        assert_eq!(label("50%off", 1.0), "50%off");
        assert!(LabelPattern::parse("%d.%m").is_err());
    }

    #[test]
    fn numbers_in_categories_parse_like_kotlin() {
        assert_eq!(kotlin_to_double("1f"), Some(1.0));
        assert_eq!(kotlin_to_double(" 2D "), Some(2.0));
        assert_eq!(kotlin_to_double("inf"), None);
        assert_eq!(kotlin_to_double("nan"), None);
        assert_eq!(kotlin_to_double("Infinity"), Some(f64::INFINITY));
    }

    #[test]
    fn doubles_print_like_kotlin() {
        assert_eq!(kotlin_double(2.0), "2.0");
        assert_eq!(kotlin_double(1.5), "1.5");
        assert_eq!(kotlin_double(1e7), "1.0E7");
        assert_eq!(kotlin_double(1.5e-4), "1.5E-4");
    }
}

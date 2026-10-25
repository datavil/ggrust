//! RGBA colors and the lets-plot color string syntax.

use std::fmt;

use super::color_names::NAMED_COLORS;

/// An 8-bit RGBA color.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct Color {
    /// Red.
    pub red: u8,
    /// Green.
    pub green: u8,
    /// Blue.
    pub blue: u8,
    /// Alpha: 0 transparent, 255 opaque.
    pub alpha: u8,
}

impl Color {
    /// Opaque black.
    pub const BLACK: Color = Color::rgb(0, 0, 0);
    /// Opaque white.
    pub const WHITE: Color = Color::rgb(255, 255, 255);
    /// Fully transparent.
    pub const TRANSPARENT: Color = Color::rgba(0, 0, 0, 0);

    /// An opaque color.
    pub const fn rgb(red: u8, green: u8, blue: u8) -> Self {
        Color::rgba(red, green, blue, 255)
    }

    /// A color with alpha.
    pub const fn rgba(red: u8, green: u8, blue: u8, alpha: u8) -> Self {
        Color {
            red,
            green,
            blue,
            alpha,
        }
    }

    /// Parse a color string exactly as lets-plot does (`Colors.parseColor`):
    ///
    /// - names: `red`, `steelblue`, `dark-grey`, `Light_Gray` (case, `-`, `_` and
    ///   grey/gray are normalised), `gray0` .. `gray100`, `transparent` / `blank` / `""`
    /// - hex: `#rgb`, `#rgba`, `#rrggbb`, `#rrggbbaa`
    /// - functional: `rgb(r, g, b)`, `rgba(r, g, b, opacity)`, `color(r, g, b[, opacity])`
    ///   with integer channels and opacity in 0..1
    /// - with opacity: `steelblue / 0.35`
    ///
    /// The error message says what was wrong and which forms are accepted.
    pub fn parse(text: &str) -> Result<Self, String> {
        let trimmed = text.trim();
        let parsed = if trimmed.contains('(') {
            parse_functional(trimmed)
        } else if trimmed.starts_with('#') {
            Color::from_hex(trimmed)
        } else if let Some(color) = named(trimmed) {
            Some(color)
        } else if let Some((name, opacity)) = trimmed.split_once('/') {
            let opacity = opacity
                .trim()
                .parse::<f64>()
                .ok()
                .filter(|value| value.is_finite());
            match (Color::parse(name), opacity) {
                (Ok(color), Some(opacity)) if !name.contains('/') => {
                    Some(color.with_opacity(opacity))
                }
                _ => None,
            }
        } else {
            None
        };
        parsed.ok_or_else(|| {
            format!(
                "invalid color '{text}'; expected a color name (e.g. 'steelblue', 'grey50'), \
                 '#rrggbb', 'rgb(r, g, b)', 'rgba(r, g, b, opacity)' or 'name / opacity'"
            )
        })
    }

    /// Parse `#rgb`, `#rgba`, `#rrggbb` or `#rrggbbaa` (case-insensitive).
    pub fn from_hex(text: &str) -> Option<Self> {
        let digits = text.strip_prefix('#')?;
        if !digits.is_ascii() {
            return None;
        }
        let pair = |index: usize| u8::from_str_radix(digits.get(index..index + 2)?, 16).ok();
        let single = |index: usize| {
            u8::from_str_radix(digits.get(index..index + 1)?, 16)
                .ok()
                .map(|value| value * 17)
        };
        match digits.len() {
            3 => Some(Color::rgb(single(0)?, single(1)?, single(2)?)),
            4 => Some(Color::rgba(single(0)?, single(1)?, single(2)?, single(3)?)),
            6 => Some(Color::rgb(pair(0)?, pair(2)?, pair(4)?)),
            8 => Some(Color::rgba(pair(0)?, pair(2)?, pair(4)?, pair(6)?)),
            _ => None,
        }
    }

    /// The same color with its alpha multiplied by `opacity` (clamped to 0..1).
    pub fn with_opacity(self, opacity: f64) -> Self {
        let opacity = if opacity.is_nan() {
            1.0
        } else {
            opacity.clamp(0.0, 1.0)
        };
        Color {
            alpha: (f64::from(self.alpha) * opacity).round() as u8,
            ..self
        }
    }

    /// Alpha as a fraction 0..1.
    pub fn opacity(self) -> f64 {
        f64::from(self.alpha) / 255.0
    }

    /// Whether the color is fully opaque.
    pub fn is_opaque(self) -> bool {
        self.alpha == 255
    }

    /// Whether the color is fully transparent.
    pub fn is_transparent(self) -> bool {
        self.alpha == 0
    }
}

/// Look up a color name after lets-plot's normalisation.
fn named(name: &str) -> Option<Color> {
    let normalised = name
        .chars()
        .filter(|character| !matches!(character, '-' | '_'))
        .collect::<String>()
        .to_lowercase()
        .replace("grey", "gray");
    if matches!(normalised.as_str(), "transparent" | "blank" | "") {
        return Some(Color::TRANSPARENT);
    }
    if let Some(level) = normalised.strip_prefix("gray")
        && !level.is_empty()
        && level.bytes().all(|byte| byte.is_ascii_digit())
    {
        let level: u32 = level.parse().ok().filter(|level| *level <= 100)?;
        let value = (f64::from(level) * 255.0 / 100.0).round() as u8;
        return Some(Color::rgb(value, value, value));
    }
    NAMED_COLORS
        .binary_search_by(|(candidate, _)| candidate.cmp(&normalised.as_str()))
        .ok()
        .map(|index| NAMED_COLORS[index].1)
}

/// `rgb(r, g, b)`, `rgba(r, g, b, a)`, `color(r, g, b[, a])`.
fn parse_functional(text: &str) -> Option<Color> {
    let (prefix, rest) = text.split_once('(')?;
    let inner = rest.strip_suffix(')')?;
    let components: Vec<&str> = inner.split(',').map(str::trim).collect();
    let valid_count = match prefix.trim() {
        "rgb" => components.len() == 3,
        "rgba" => components.len() == 4,
        "color" => matches!(components.len(), 3 | 4),
        _ => false,
    };
    if !valid_count {
        return None;
    }
    let channel = |index: usize| components[index].parse::<u8>().ok();
    let opacity = match components.get(3) {
        Some(value) => value
            .parse::<f64>()
            .ok()
            .filter(|value| (0.0..=1.0).contains(value))?,
        None => 1.0,
    };
    Some(Color::rgba(
        channel(0)?,
        channel(1)?,
        channel(2)?,
        (opacity * 255.0).round() as u8,
    ))
}

impl fmt::Display for Color {
    /// `#rrggbb`, ignoring alpha (SVG carries opacity separately).
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "#{:02x}{:02x}{:02x}",
            self.red, self.green, self.blue
        )
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_hex_forms() {
        assert_eq!(Color::from_hex("#fff"), Some(Color::WHITE));
        assert_eq!(Color::from_hex("#1F77b4"), Some(Color::rgb(31, 119, 180)));
        assert_eq!(
            Color::from_hex("#ff000080"),
            Some(Color::rgba(255, 0, 0, 128))
        );
        assert_eq!(Color::from_hex("ff0000"), None);
        assert_eq!(Color::from_hex("#ff00"), Some(Color::rgba(255, 255, 0, 0)));
        assert_eq!(Color::from_hex("#ff0"), Some(Color::rgb(255, 255, 0)));
        assert_eq!(Color::from_hex("#gg0000"), None);
        assert_eq!(Color::from_hex("#é00"), None);
    }

    #[test]
    fn parses_lets_plot_color_strings() {
        let parse = |text: &str| Color::parse(text).unwrap();
        assert_eq!(parse("red"), Color::rgb(255, 0, 0));
        assert_eq!(parse("SteelBlue"), Color::rgb(70, 130, 180));
        assert_eq!(parse("dark-grey"), parse("darkgray"));
        assert_eq!(parse("light_gray"), parse("lightgrey"));
        assert_eq!(parse("grey50"), Color::rgb(128, 128, 128));
        assert_eq!(parse("gray100"), Color::WHITE);
        assert_eq!(parse("gray0"), Color::BLACK);
        assert_eq!(parse("transparent"), Color::TRANSPARENT);
        assert_eq!(parse(""), Color::TRANSPARENT);
        assert_eq!(parse("#f008"), Color::rgba(255, 0, 0, 136));
        assert_eq!(parse("rgb(1, 2, 3)"), Color::rgb(1, 2, 3));
        assert_eq!(parse("rgba(1,2,3,0.5)"), Color::rgba(1, 2, 3, 128));
        assert_eq!(parse("color(1, 2, 3)"), Color::rgb(1, 2, 3));
        assert_eq!(parse("steelblue / 0.35"), Color::rgba(70, 130, 180, 89));
    }

    #[test]
    fn rejects_invalid_color_strings_with_guidance() {
        for text in [
            "reddish",
            "gray101",
            "rgb(1, 2)",
            "rgb(256, 0, 0)",
            "rgba(1, 2, 3, 2)",
            "red / x",
            "red / 0.5 / 0.5",
            "#12345",
        ] {
            let error = Color::parse(text).unwrap_err();
            assert!(
                error.starts_with(&format!("invalid color '{text}'")),
                "{error}"
            );
        }
    }

    #[test]
    fn name_table_is_sorted_and_complete() {
        assert_eq!(NAMED_COLORS.len(), 147);
        assert!(NAMED_COLORS.windows(2).all(|pair| pair[0].0 < pair[1].0));
    }

    #[test]
    fn opacity_multiplies_and_clamps() {
        assert_eq!(Color::BLACK.with_opacity(0.5).alpha, 128);
        assert_eq!(Color::BLACK.with_opacity(2.0).alpha, 255);
        assert_eq!(Color::BLACK.with_opacity(-1.0).alpha, 0);
        assert_eq!(Color::BLACK.with_opacity(f64::NAN).alpha, 255);
    }

    #[test]
    fn displays_as_hex_without_alpha() {
        assert_eq!(Color::rgba(31, 119, 180, 10).to_string(), "#1f77b4");
    }
}

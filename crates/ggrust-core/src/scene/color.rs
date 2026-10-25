//! RGBA colors.
//!
//! Parsing of named colors (`"red"`, `"grey50"`) belongs to the scale/theme
//! layer; this module only knows hex notation.

use std::fmt;

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

    /// Parse `#rgb`, `#rrggbb` or `#rrggbbaa` (case-insensitive).
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
        assert_eq!(Color::from_hex("#ff00"), None);
        assert_eq!(Color::from_hex("#gg0000"), None);
        assert_eq!(Color::from_hex("#é00"), None);
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

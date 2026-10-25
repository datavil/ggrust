//! Fonts and text measurement.
//!
//! What it does: embeds Liberation Sans (SIL OFL 1.1, metric-compatible with
//! Arial/Helvetica) and measures text with harfrust, the same shaper resvg uses,
//! so layout widths match rendered PNGs exactly and browsers closely.
//!
//! What it does not do: line wrapping or bidirectional text. Multi-line labels are
//! split on `\n` by [`measure_lines`].

use std::sync::LazyLock;

use harfrust::{FontRef, ShaperData, UnicodeBuffer};
use read_fonts::TableProvider;

/// Family name of the embedded font.
pub const FONT_FAMILY: &str = "Liberation Sans";

/// CSS `font-family` written into SVG: the embedded font first, then
/// metric-compatible fallbacks so browsers lay text out the same way.
pub const CSS_FONT_FAMILY: &str = "'Liberation Sans', Arial, Helvetica, sans-serif";

/// Line spacing as a multiple of the font size (lets-plot uses about 1.15).
pub const LINE_HEIGHT: f64 = 1.15;

/// Weight and slant of a font.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum FontFace {
    /// Normal weight, upright.
    #[default]
    Plain,
    /// Bold, upright.
    Bold,
    /// Normal weight, italic.
    Italic,
    /// Bold italic.
    BoldItalic,
}

impl FontFace {
    /// Every face, in the order the font files are embedded.
    pub const ALL: [FontFace; 4] = [
        FontFace::Plain,
        FontFace::Bold,
        FontFace::Italic,
        FontFace::BoldItalic,
    ];

    /// Parse a lets-plot `face` value: `plain`, `bold`, `italic`, `bold_italic`.
    pub fn from_name(name: &str) -> Option<Self> {
        match name {
            "plain" => Some(FontFace::Plain),
            "bold" => Some(FontFace::Bold),
            "italic" => Some(FontFace::Italic),
            "bold_italic" | "bolditalic" | "bold.italic" => Some(FontFace::BoldItalic),
            _ => None,
        }
    }

    /// Whether the face is bold.
    pub fn is_bold(self) -> bool {
        matches!(self, FontFace::Bold | FontFace::BoldItalic)
    }

    /// Whether the face is italic.
    pub fn is_italic(self) -> bool {
        matches!(self, FontFace::Italic | FontFace::BoldItalic)
    }

    /// Raw TrueType data of the embedded font file.
    pub fn font_data(self) -> &'static [u8] {
        match self {
            FontFace::Plain => include_bytes!("../../fonts/LiberationSans-Regular.ttf"),
            FontFace::Bold => include_bytes!("../../fonts/LiberationSans-Bold.ttf"),
            FontFace::Italic => include_bytes!("../../fonts/LiberationSans-Italic.ttf"),
            FontFace::BoldItalic => include_bytes!("../../fonts/LiberationSans-BoldItalic.ttf"),
        }
    }

    fn index(self) -> usize {
        self as usize
    }
}

/// Size of a single line of text, in pixels.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct TextExtent {
    /// Advance width.
    pub width: f64,
    /// Distance from the baseline up to the top of the line box.
    pub ascent: f64,
    /// Distance from the baseline down to the bottom of the line box (positive).
    pub descent: f64,
}

impl TextExtent {
    /// `ascent + descent`.
    pub fn height(&self) -> f64 {
        self.ascent + self.descent
    }
}

/// Size of a block of lines, in pixels.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct BlockExtent {
    /// Width of the widest line.
    pub width: f64,
    /// Total height: first line box plus `LINE_HEIGHT * size` per extra line.
    pub height: f64,
    /// Number of lines.
    pub line_count: usize,
}

/// Measure one line of text at `size` pixels.
pub fn measure(text: &str, size: f64, face: FontFace) -> TextExtent {
    let font = &FONTS[face.index()];
    let scale = size / f64::from(font.units_per_em);
    let width = if text.is_empty() {
        0.0
    } else {
        let shaper = font.shaper_data.shaper(&font.font_ref).build();
        let mut buffer = UnicodeBuffer::new();
        buffer.push_str(text);
        buffer.guess_segment_properties();
        let output = shaper.shape(buffer, harfrust::ShapeOptions::new());
        let advance: i64 = output
            .glyph_positions()
            .iter()
            .map(|position| i64::from(position.x_advance))
            .sum();
        advance as f64 * scale
    };
    TextExtent {
        width,
        ascent: f64::from(font.ascender) * scale,
        descent: f64::from(font.descender) * scale,
    }
}

/// Measure text that may contain `\n` line breaks.
pub fn measure_lines(text: &str, size: f64, face: FontFace) -> BlockExtent {
    let mut width: f64 = 0.0;
    let mut line_count = 0;
    for line in text.split('\n') {
        width = width.max(measure(line, size, face).width);
        line_count += 1;
    }
    let first = measure("", size, face);
    BlockExtent {
        width,
        height: first.height() + (line_count - 1) as f64 * LINE_HEIGHT * size,
        line_count,
    }
}

struct LoadedFont {
    font_ref: FontRef<'static>,
    shaper_data: ShaperData,
    units_per_em: u16,
    ascender: i16,
    /// Positive distance below the baseline.
    descender: i16,
}

static FONTS: LazyLock<Vec<LoadedFont>> = LazyLock::new(|| {
    FontFace::ALL
        .iter()
        .map(|face| {
            // The embedded fonts are known-good; a parse failure is a build defect.
            let font_ref = FontRef::new(face.font_data()).expect("embedded font parses");
            let head = font_ref.head().expect("embedded font has a head table");
            let hhea = font_ref.hhea().expect("embedded font has an hhea table");
            LoadedFont {
                shaper_data: ShaperData::new(&font_ref),
                units_per_em: head.units_per_em(),
                ascender: hhea.ascender().to_i16(),
                descender: hhea.descender().to_i16().saturating_abs(),
                font_ref,
            }
        })
        .collect()
});

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn empty_text_has_zero_width_but_a_line_box() {
        let extent = measure("", 10.0, FontFace::Plain);
        assert_eq!(extent.width, 0.0);
        assert!(extent.ascent > 8.0 && extent.ascent < 10.0, "{extent:?}");
        assert!(extent.descent > 1.0 && extent.descent < 3.0, "{extent:?}");
    }

    #[test]
    fn width_scales_linearly_with_size() {
        let small = measure("Cluster 12", 10.0, FontFace::Plain).width;
        let large = measure("Cluster 12", 20.0, FontFace::Plain).width;
        assert!((large - 2.0 * small).abs() < 1e-9);
    }

    #[test]
    fn arial_compatible_metrics() {
        // Arial advance widths in 1/1000 em: "0" = 556, "M" = 833, space = 278.
        let digit = measure("0", 1000.0, FontFace::Plain).width;
        let letter = measure("M", 1000.0, FontFace::Plain).width;
        let space = measure(" ", 1000.0, FontFace::Plain).width;
        assert!((digit - 556.0).abs() < 1.0, "{digit}");
        assert!((letter - 833.0).abs() < 1.0, "{letter}");
        assert!((space - 278.0).abs() < 1.0, "{space}");
    }

    #[test]
    fn bold_is_wider_than_plain() {
        let plain = measure("UMAP1", 12.0, FontFace::Plain).width;
        let bold = measure("UMAP1", 12.0, FontFace::Bold).width;
        assert!(bold > plain);
    }

    #[test]
    fn kerning_is_applied() {
        // "AV" kerns tighter than "A" + "V" measured separately.
        let pair = measure("AV", 100.0, FontFace::Plain).width;
        let separate =
            measure("A", 100.0, FontFace::Plain).width + measure("V", 100.0, FontFace::Plain).width;
        assert!(pair < separate, "{pair} vs {separate}");
    }

    #[test]
    fn multi_line_blocks() {
        let block = measure_lines("short\na much longer line", 10.0, FontFace::Plain);
        assert_eq!(block.line_count, 2);
        let longest = measure("a much longer line", 10.0, FontFace::Plain).width;
        assert!((block.width - longest).abs() < 1e-9);
        let single = measure("x", 10.0, FontFace::Plain).height();
        assert!((block.height - (single + LINE_HEIGHT * 10.0)).abs() < 1e-9);
    }

    #[test]
    fn face_names() {
        assert_eq!(
            FontFace::from_name("bold_italic"),
            Some(FontFace::BoldItalic)
        );
        assert_eq!(FontFace::from_name("heavy"), None);
    }
}

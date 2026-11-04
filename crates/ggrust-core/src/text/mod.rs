//! Fonts and text measurement.
//!
//! What it does: embeds Liberation Sans and Liberation Mono (SIL OFL 1.1,
//! metric-compatible with Arial/Helvetica and Courier New) and measures text with
//! harfrust, the same shaper resvg uses, so layout widths match rendered PNGs
//! exactly and browsers closely. Fonts the user registers ([`register_font`]) are
//! measured and drawn the same way; other family names map to the embedded two
//! ([`FontFamily::from_name`]).
//!
//! What it does not do: line wrapping or bidirectional text. Multi-line labels are
//! split on `\n` by [`measure_lines`].

mod estimate;
mod registry;

pub use estimate::estimated_width;
pub(crate) use registry::LoadedFont;
pub use registry::{
    DirectoryFonts, FONT_EXTENSIONS, register_font, register_font_data, register_font_dir,
    registered_fonts,
};

use std::borrow::Cow;
use std::sync::{Arc, LazyLock};

use harfrust::UnicodeBuffer;

/// Family name of the embedded sans font.
pub const FONT_FAMILY: &str = "Liberation Sans";

/// Family name of the embedded monospace font.
pub const MONO_FONT_FAMILY: &str = "Liberation Mono";

/// CSS `font-family` written into SVG: the embedded font first, then
/// metric-compatible fallbacks so browsers lay text out the same way.
pub const CSS_FONT_FAMILY: &str = "'Liberation Sans', Arial, Helvetica, sans-serif";

/// CSS `font-family` of monospace text.
pub const CSS_MONO_FONT_FAMILY: &str = "'Liberation Mono', 'Courier New', Courier, monospace";

/// A font family: one of the embedded two, or one the user registered.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum FontFamily {
    /// Liberation Sans (Arial metrics).
    #[default]
    Sans,
    /// Liberation Mono (Courier New metrics).
    Mono,
    /// A family registered with [`register_font`] (its registry id).
    Registered(u16),
}

/// Family names that map to an embedded family (lower case), for suggestions.
const EMBEDDED_ALIASES: &[&str] = &[
    "sans",
    "sans-serif",
    "arial",
    "helvetica",
    "liberation sans",
    "mono",
    "monospace",
    "courier",
    "courier new",
    "liberation mono",
];

impl FontFamily {
    /// The family for a family name, case-insensitive. A registered family
    /// ([`register_font`]) wins over the aliases below. Monospace names
    /// (`mono`, `monospace`, `Courier`, `Courier New`, `Liberation Mono`, `Menlo`,
    /// `Consolas`, ...) give Mono; sans names (`sans`, `sans-serif`, `Arial`,
    /// `Helvetica`, `Liberation Sans`, ...) give Sans. Any other name gives Sans and
    /// `false` (unknown; see [`family_warning`]).
    pub fn from_name(name: &str) -> (Self, bool) {
        if let Some(id) = registry::lookup(name.trim()) {
            return (FontFamily::Registered(id), true);
        }
        let name = name.trim().to_lowercase();
        const MONO: &[&str] = &[
            "mono",
            "monospace",
            "monospaced",
            "courier",
            "courier new",
            "liberation mono",
            "menlo",
            "monaco",
            "consolas",
            "dejavu sans mono",
            "lucida console",
        ];
        const SANS: &[&str] = &[
            "sans",
            "sans-serif",
            "sans serif",
            "arial",
            "helvetica",
            "liberation sans",
            "lucida grande",
            "verdana",
        ];
        if MONO.contains(&name.as_str()) {
            (FontFamily::Mono, true)
        } else {
            (FontFamily::Sans, SANS.contains(&name.as_str()))
        }
    }

    /// CSS `font-family` of an embedded family for SVG (registered families are
    /// written by the SVG renderer with their embedded web font).
    pub fn css(self) -> &'static str {
        match self {
            FontFamily::Sans | FontFamily::Registered(_) => CSS_FONT_FAMILY,
            FontFamily::Mono => CSS_MONO_FONT_FAMILY,
        }
    }

    /// The family's name: the registered name, or the embedded font's.
    pub fn name(self) -> String {
        match self {
            FontFamily::Sans => FONT_FAMILY.to_owned(),
            FontFamily::Mono => MONO_FONT_FAMILY.to_owned(),
            FontFamily::Registered(id) => registry::family_name(id),
        }
    }
}

/// The warning for a family name that names no registered or embedded font, with
/// a suggestion; `None` for known names. `path` locates it (e.g. `theme.title.family`).
pub fn family_warning(name: &str, path: &str) -> Option<String> {
    if FontFamily::from_name(name).1 {
        return None;
    }
    let registered = registry::family_names();
    let suggestion = crate::suggest::did_you_mean(
        name.trim(),
        registered
            .iter()
            .map(String::as_str)
            .chain(EMBEDDED_ALIASES.iter().copied()),
    );
    let known = if registered.is_empty() {
        "no fonts are registered".to_owned()
    } else {
        format!("registered: {}", registered.join(", "))
    };
    Some(format!(
        "at '{path}': font family '{name}' is not registered ({known}); Liberation Sans was \
         used. Register the font file with ggrust.register_font(path){suggestion}"
    ))
}

/// The warning when `font` is a registered family without the requested face
/// (the nearest registered face is drawn); `None` otherwise.
pub fn face_warning(font: Font, path: &str) -> Option<String> {
    let FontFamily::Registered(id) = font.family else {
        return None;
    };
    let (_, used) = registry::face(id, font.face)?;
    (used != font.face).then(|| {
        format!(
            "at '{path}': font family '{}' has no {} face registered; its {} face was used. \
             Register the {} font file with ggrust.register_font(path)",
            registry::family_name(id),
            font.face.label(),
            used.label(),
            font.face.label(),
        )
    })
}

/// An embedded font: family and face. A bare [`FontFace`] means the sans family.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub struct Font {
    /// Family.
    pub family: FontFamily,
    /// Weight and slant.
    pub face: FontFace,
}

impl From<FontFace> for Font {
    fn from(face: FontFace) -> Self {
        Font {
            family: FontFamily::Sans,
            face,
        }
    }
}

impl Font {
    /// The family and the face actually drawn: a registered family without the
    /// requested face draws its nearest registered face.
    pub fn resolved_face(self) -> FontFace {
        match self.family {
            FontFamily::Registered(id) => {
                registry::face(id, self.face).map_or(self.face, |(_, face)| face)
            }
            _ => self.face,
        }
    }

    /// Every embedded font, in loading order.
    pub fn all() -> impl Iterator<Item = Font> {
        [FontFamily::Sans, FontFamily::Mono]
            .into_iter()
            .flat_map(|family| FontFace::ALL.map(|face| Font { family, face }))
    }

    /// Raw TrueType data of an embedded font (a registered family gives its file).
    pub fn font_data(self) -> &'static [u8] {
        match self.family {
            FontFamily::Sans => self.face.font_data(),
            FontFamily::Registered(_) => loaded(self).data,
            FontFamily::Mono => match self.face {
                FontFace::Plain => include_bytes!("../../fonts/LiberationMono-Regular.ttf"),
                FontFace::Bold => include_bytes!("../../fonts/LiberationMono-Bold.ttf"),
                FontFace::Italic => include_bytes!("../../fonts/LiberationMono-Italic.ttf"),
                FontFace::BoldItalic => {
                    include_bytes!("../../fonts/LiberationMono-BoldItalic.ttf")
                }
            },
        }
    }

    /// Position among the embedded fonts (a registered family counts as Sans).
    fn embedded_index(self) -> usize {
        let family = match self.family {
            FontFamily::Mono => 1,
            FontFamily::Sans | FontFamily::Registered(_) => 0,
        };
        family * FontFace::ALL.len() + self.face as usize
    }
}

/// A parsed face: embedded ones are borrowed (no reference counting on the
/// measuring hot path), registered ones shared.
#[derive(Clone)]
pub(crate) enum FaceRef {
    Embedded(&'static LoadedFont),
    Registered(Arc<LoadedFont>),
}

impl std::ops::Deref for FaceRef {
    type Target = LoadedFont;

    fn deref(&self) -> &LoadedFont {
        match self {
            FaceRef::Embedded(face) => face,
            FaceRef::Registered(face) => face,
        }
    }
}

/// The parsed face drawn for `font`: an embedded face, or the registered one
/// (nearest face; Liberation Sans if the id is unknown).
pub(crate) fn loaded(font: Font) -> FaceRef {
    if let FontFamily::Registered(id) = font.family
        && let Some((loaded, _)) = registry::face(id, font.face)
    {
        return FaceRef::Registered(loaded);
    }
    FaceRef::Embedded(&FONTS[font.embedded_index()])
}

/// Every registered face, with the registry generation it belongs to.
pub(crate) fn registered_faces() -> (u64, Vec<Arc<LoadedFont>>) {
    registry::all_faces()
}

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

    /// Parse a lets-plot `face` value as lets-plot does: split on ` `, `_`, `.` and `-`,
    /// then look for the words `bold` and `italic` (case-sensitive, so `Bold` is plain).
    /// Other words are ignored: `bold italic`, `italic-bold` and `plain bold` all parse.
    pub fn from_name(name: &str) -> Self {
        let has = |word: &str| Self::words(name).any(|part| part == word);
        match (has("bold"), has("italic")) {
            (false, false) => FontFace::Plain,
            (true, false) => FontFace::Bold,
            (false, true) => FontFace::Italic,
            (true, true) => FontFace::BoldItalic,
        }
    }

    /// Words of a `face` value that lets-plot ignores (anything but `plain`, `normal`,
    /// `bold` and `italic`), for warnings.
    pub fn unknown_words(name: &str) -> Vec<&str> {
        Self::words(name)
            .filter(|word| !["plain", "normal", "bold", "italic"].contains(word))
            .collect()
    }

    fn words(name: &str) -> impl Iterator<Item = &str> {
        name.split([' ', '_', '.', '-'])
            .filter(|word| !word.is_empty())
    }

    /// The face as written in lets-plot (`plain`, `bold`, `italic`, `bold_italic`).
    pub fn label(self) -> &'static str {
        match self {
            FontFace::Plain => "plain",
            FontFace::Bold => "bold",
            FontFace::Italic => "italic",
            FontFace::BoldItalic => "bold_italic",
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

/// Text as one line: tabs, carriage returns and newlines become spaces.
///
/// Renderers write exactly this (with whitespace preserved), and [`measure`]
/// measures exactly this, so widths always match the drawn text. Split multi-line
/// labels on `\n` before (see [`measure_lines`]).
pub fn single_line(text: &str) -> Cow<'_, str> {
    if text.contains(['\t', '\r', '\n']) {
        Cow::Owned(text.replace(['\t', '\r', '\n'], " "))
    } else {
        Cow::Borrowed(text)
    }
}

/// Measure one line of text at `size` pixels (see [`single_line`]).
pub fn measure(text: &str, size: f64, font: impl Into<Font>) -> TextExtent {
    let font = loaded(font.into());
    let scale = size / f64::from(font.units_per_em);
    let text = single_line(text);
    let width = if text.is_empty() {
        0.0
    } else {
        let shaper = font.shaper_data.shaper(&font.font_ref).build();
        let mut buffer = UnicodeBuffer::new();
        buffer.push_str(&text);
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

/// One glyph of a shaped line, in font units (see [`shape`]).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct ShapedGlyph {
    /// Glyph id in the embedded font.
    pub glyph_id: u16,
    /// Horizontal advance after shaping (kerning included).
    pub x_advance: i32,
    /// Byte offset, in the [`single_line`] text, of the cluster the glyph belongs to.
    pub cluster: usize,
}

/// Shape one line (see [`single_line`]) with the shaper [`measure`] uses, so a
/// renderer drawing these glyphs gets exactly the measured width. Returns the
/// glyphs in visual order and the font's units per em.
pub fn shape(text: &str, font: Font) -> (Vec<ShapedGlyph>, u16) {
    let font = loaded(font);
    let text = single_line(text);
    if text.is_empty() {
        return (Vec::new(), font.units_per_em);
    }
    let shaper = font.shaper_data.shaper(&font.font_ref).build();
    let mut buffer = UnicodeBuffer::new();
    buffer.push_str(&text);
    buffer.guess_segment_properties();
    let output = shaper.shape(buffer, harfrust::ShapeOptions::new());
    let glyphs = output
        .glyph_infos()
        .iter()
        .zip(output.glyph_positions())
        .map(|(info, position)| ShapedGlyph {
            // Fonts have at most 65536 glyphs.
            glyph_id: u16::try_from(info.glyph_id).unwrap_or(0),
            x_advance: position.x_advance,
            cluster: info.cluster as usize,
        })
        .collect();
    (glyphs, font.units_per_em)
}

/// Width of a label with an optional superscript (see [`crate::scene::Text`]).
pub fn measure_label(
    text: &str,
    superscript: Option<&str>,
    suffix: Option<&str>,
    size: f64,
    face: impl Into<Font>,
) -> f64 {
    let face = face.into();
    let scale = crate::scene::SUPERSCRIPT_SCALE;
    measure(text, size, face).width
        + superscript.map_or(0.0, |superscript| {
            measure(superscript, size * scale, face).width
        })
        + suffix.map_or(0.0, |suffix| measure(suffix, size, face).width)
}

/// Measure text that may contain `\n` line breaks.
pub fn measure_lines(text: &str, size: f64, face: impl Into<Font>) -> BlockExtent {
    let face = face.into();
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

static FONTS: LazyLock<Vec<LoadedFont>> = LazyLock::new(|| {
    Font::all()
        .enumerate()
        .map(|(position, font)| {
            // The embedded fonts are known-good; a parse failure is a build defect.
            // Their CSS names are never written, so the position stands in for a hash.
            LoadedFont::parse(font.font_data(), 0, position as u64)
                .unwrap_or_else(|reason| panic!("embedded font does not parse: {reason}"))
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
    fn mono_has_courier_metrics() {
        let mono = |text: &str| {
            measure(
                text,
                1000.0,
                Font {
                    family: FontFamily::Mono,
                    face: FontFace::Plain,
                },
            )
            .width
        };
        // Every glyph is 600/1000 em wide.
        assert!((mono("i") - 600.0).abs() < 1.0);
        assert!((mono("MW") - 1200.0).abs() < 1.0);
        assert_eq!(
            FontFamily::from_name("Courier New"),
            (FontFamily::Mono, true)
        );
        assert_eq!(FontFamily::from_name("Arial"), (FontFamily::Sans, true));
        assert_eq!(FontFamily::from_name("Georgia"), (FontFamily::Sans, false));
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
    fn control_whitespace_measures_as_spaces() {
        let spaced = measure("a b", 20.0, FontFace::Plain).width;
        assert_eq!(measure("a\tb", 20.0, FontFace::Plain).width, spaced);
        assert_eq!(measure("a\nb", 20.0, FontFace::Plain).width, spaced);
        assert_eq!(single_line("plain"), "plain");
    }

    #[test]
    fn glyph_coverage_used_by_labels() {
        // Missing glyphs measure as .notdef (750 units). U+2212 is used for minus
        // signs in exponents; U+207B (superscript minus) is NOT in Liberation Sans.
        let notdef = measure("\u{FFFF}", 1000.0, FontFace::Plain).width;
        for character in ["\u{2212}", "\u{00B7}", "0", "9"] {
            assert_ne!(
                measure(character, 1000.0, FontFace::Plain).width,
                notdef,
                "{character}"
            );
        }
        assert_eq!(measure("\u{207B}", 1000.0, FontFace::Plain).width, notdef);
    }

    #[test]
    fn face_names() {
        // Expected faces measured on lets-plot 4.x SVG output (theme and geom_text agree).
        let cases = [
            ("plain", FontFace::Plain),
            ("bold", FontFace::Bold),
            ("italic", FontFace::Italic),
            ("bold_italic", FontFace::BoldItalic),
            ("bold.italic", FontFace::BoldItalic),
            ("bold-italic", FontFace::BoldItalic),
            ("bold italic", FontFace::BoldItalic),
            ("italic bold", FontFace::BoldItalic),
            ("bold  italic", FontFace::BoldItalic),
            ("bold._italic", FontFace::BoldItalic),
            (" bold", FontFace::Bold),
            ("plain bold", FontFace::Bold),
            ("bold Italic", FontFace::Bold),
            ("Bold", FontFace::Plain),
            ("BOLD", FontFace::Plain),
            ("Italic", FontFace::Plain),
            ("Bold Italic", FontFace::Plain),
            ("bolditalic", FontFace::Plain),
            ("bold,italic", FontFace::Plain),
            ("bold\titalic", FontFace::Plain),
            ("boldx", FontFace::Plain),
            ("heavy", FontFace::Plain),
            ("", FontFace::Plain),
        ];
        for (name, face) in cases {
            assert_eq!(FontFace::from_name(name), face, "{name:?}");
        }
        assert!(FontFace::unknown_words("plain bold-italic").is_empty());
        assert!(FontFace::unknown_words("normal").is_empty());
        assert_eq!(FontFace::unknown_words("Bold italic"), ["Bold"]);
    }
}

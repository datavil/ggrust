//! `scale_*_npg()`, `scale_*_d3()`, ...: the ggsci journal palettes (R package
//! ggsci, not in lets-plot). Discrete scales with fixed color lists: category `i`
//! takes color `i`. More categories than colors follow the brewer `overflow` rule
//! (generated colors by default) and warn; ggsci leaves them missing instead
//! (deviations.md). Continuous data is quantized like `scale_*_manual()`.
//!
//! Not here: the continuous ggsci scales (`gsea`, `material`, ...) and the other
//! discrete families.

use super::StyleMapper;
use super::brewer::{Overflow, extend_colors};
pub use super::ggsci_data::FAMILIES;
use super::manual::{cycle, quantize};
use crate::data::ContinuousColumn;
use crate::scene::{Color, PerItem};
use crate::style::{StyleValues, ValueKind};

/// One ggsci family (`npg`, `d3`, ...) and its palettes, the first the default.
#[derive(Debug)]
pub struct GgsciFamily {
    /// ggsci name, e.g. `npg`.
    pub name: &'static str,
    /// Its `scale_mapper_kind`, e.g. `color_npg`.
    pub kind: &'static str,
    /// Palettes, the default first.
    pub palettes: &'static [GgsciPalette],
}

/// One ggsci palette.
#[derive(Debug)]
pub struct GgsciPalette {
    /// ggsci name, e.g. `nrc`, `category20`.
    pub name: &'static str,
    /// Colors in ggsci's order.
    pub colors: &'static [[u8; 3]],
}

impl GgsciFamily {
    /// The palette called `name` (exact, as ggsci's `match.arg`), or the default.
    pub fn palette(&self, name: Option<&str>) -> Option<&'static GgsciPalette> {
        match name {
            None => self.palettes.first(),
            Some(name) => self.palettes.iter().find(|palette| palette.name == name),
        }
    }

    /// The palette names, quoted, for error messages.
    pub fn palette_names(&self) -> String {
        self.palettes
            .iter()
            .map(|palette| format!("'{}'", palette.name))
            .collect::<Vec<_>>()
            .join(", ")
    }
}

/// The family of a `scale_mapper_kind` (`color_npg` -> `npg`).
pub fn family_by_kind(kind: &str) -> Option<&'static GgsciFamily> {
    FAMILIES.iter().find(|family| family.kind == kind)
}

/// A ggsci palette with its options.
#[derive(Debug, Clone, PartialEq)]
pub struct GgsciMapper {
    /// `scale_mapper_kind`, e.g. `color_npg`.
    pub kind: &'static str,
    /// The palette's colors, opaque.
    pub colors: Vec<Color>,
    /// Alpha byte of every color (ggsci `alpha`, 0..1 rounded like R's `rgb()`).
    pub alpha: u8,
    /// `overflow`: how to get more colors than the palette has (`Auto`: generate).
    pub overflow: Overflow,
    /// Color of missing and unknown values.
    pub na: Color,
}

impl GgsciMapper {
    /// The palette's colors for `palette`, with `opacity` in (0, 1].
    pub fn new(
        kind: &'static str,
        palette: &GgsciPalette,
        opacity: f64,
        overflow: Overflow,
        na: Color,
    ) -> Self {
        GgsciMapper {
            kind,
            colors: palette
                .colors
                .iter()
                .map(|&[red, green, blue]| Color::rgb(red, green, blue))
                .collect(),
            alpha: (opacity * 255.0 + 0.5).floor().clamp(0.0, 255.0) as u8,
            overflow,
            na,
        }
    }

    /// `count` colors (at least the palette), with the alpha applied.
    fn colors(&self, count: usize) -> Vec<Color> {
        let overflow = match self.overflow {
            Overflow::Auto => Overflow::Generate,
            chosen => chosen,
        };
        extend_colors(self.colors.clone(), count, overflow)
            .into_iter()
            .map(|color| Color {
                alpha: self.alpha,
                ..color
            })
            .collect()
    }
}

impl StyleMapper for GgsciMapper {
    fn name(&self) -> &'static str {
        self.kind
    }

    fn value_kind(&self) -> ValueKind {
        ValueKind::Color
    }

    fn map_continuous(&self, column: &ContinuousColumn, domain: (f64, f64)) -> StyleValues {
        let colors = self.colors(self.colors.len());
        StyleValues::Colors(PerItem::Varying(quantize(&colors, self.na, column, domain)))
    }

    fn map_discrete(&self, indices: &[Option<usize>], level_count: usize) -> StyleValues {
        let colors = self.colors(level_count);
        StyleValues::Colors(PerItem::Varying(cycle(&colors, self.na, indices)))
    }

    fn palette_size(&self) -> Option<usize> {
        Some(self.colors.len())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn hex(color: Color) -> String {
        format!("#{:02X}{:02X}{:02X}", color.red, color.green, color.blue)
    }

    fn palette_hexes(kind: &str, palette: &str) -> Vec<String> {
        let family = family_by_kind(kind).unwrap();
        let palette = family.palette(Some(palette)).unwrap();
        GgsciMapper::new(family.kind, palette, 1.0, Overflow::Auto, Color::BLACK)
            .colors(0)
            .into_iter()
            .map(hex)
            .collect()
    }

    /// ggsci 5.2.0 `ggsci_db`, typed in again from its published hex values.
    #[test]
    fn palettes_match_ggsci() {
        let expected: &[(&str, &str, &str)] = &[
            (
                "color_npg",
                "nrc",
                "E64B35 4DBBD5 00A087 3C5488 F39B7F 8491B4 91D1C2 DC0000 7E6148 B09C85",
            ),
            (
                "color_aaas",
                "default",
                "3B4992 EE0000 008B45 631879 008280 BB0021 5F559B A20056 808180 1B1919",
            ),
            (
                "color_nejm",
                "default",
                "BC3C29 0072B5 E18727 20854E 7876B1 6F99AD FFDC91 EE4C97",
            ),
            (
                "color_lancet",
                "lanonc",
                "00468B ED0000 42B540 0099B4 925E9F FDAF91 AD002A ADB6B6 1B1919",
            ),
            (
                "color_jama",
                "default",
                "374E55 DF8F44 00A1D5 B24745 79AF97 6A6599 80796B",
            ),
            (
                "color_jco",
                "default",
                "0073C2 EFC000 868686 CD534C 7AA6DC 003C67 8F7700 3B3B3B A73030 4A6990",
            ),
            (
                "color_d3",
                "category10",
                "1F77B4 FF7F0E 2CA02C D62728 9467BD 8C564B E377C2 7F7F7F BCBD22 17BECF",
            ),
            (
                "color_d3",
                "category20",
                "1F77B4 FF7F0E 2CA02C D62728 9467BD 8C564B E377C2 7F7F7F BCBD22 17BECF \
                 AEC7E8 FFBB78 98DF8A FF9896 C5B0D5 C49C94 F7B6D2 C7C7C7 DBDB8D 9EDAE5",
            ),
            (
                "color_d3",
                "category20b",
                "393B79 637939 8C6D31 843C39 7B4173 5254A3 8CA252 BD9E39 AD494A A55194 \
                 6B6ECF B5CF6B E7BA52 D6616B CE6DBD 9C9EDE CEDB9C E7CB94 E7969C DE9ED6",
            ),
            (
                "color_d3",
                "category20c",
                "3182BD E6550D 31A354 756BB1 636363 6BAED6 FD8D3C 74C476 9E9AC8 969696 \
                 9ECAE1 FDAE6B A1D99B BCBDDC BDBDBD C6DBEF FDD0A2 C7E9C0 DADAEB D9D9D9",
            ),
            (
                "color_igv",
                "default",
                "5050FF CE3D32 749B58 F0E685 466983 BA6338 5DB1DD 802268 6BD76B D595A7 \
                 924822 837B8D C75127 D58F5C 7A65A5 E4AF69 3B1B53 CDDEB7 612A79 AE1F63 \
                 E7C76F 5A655E CC9900 99CC00 A9A9A9 CC9900 99CC00 33CC00 00CC33 00CC99 \
                 0099CC 0A47FF 4775FF FFC20A FFD147 990033 991A00 996600 809900 339900 \
                 00991A 009966 008099 003399 1A0099 660099 990080 D60047 FF1463 00D68F \
                 14FFB1",
            ),
            ("color_igv", "alternating", "5773CC FFB900"),
        ];
        for (kind, palette, hexes) in expected {
            let expected: Vec<String> = hexes
                .split_whitespace()
                .map(|hex| format!("#{hex}"))
                .collect();
            assert_eq!(palette_hexes(kind, palette), expected, "{kind} {palette}");
        }
        let palette_count: usize = FAMILIES.iter().map(|family| family.palettes.len()).sum();
        assert_eq!(palette_count, expected.len());
    }

    #[test]
    fn defaults_alpha_and_overflow() {
        let d3 = family_by_kind("color_d3").unwrap();
        assert_eq!(d3.palette(None).unwrap().name, "category10");
        assert!(d3.palette(Some("category30")).is_none());
        assert_eq!(
            family_by_kind("color_brewer").map(|family| family.name),
            None
        );
        let npg = family_by_kind("color_npg").unwrap().palette(None).unwrap();
        // R `rgb(alpha = 0.5 * 255)`: 127.5 rounds to 128 (#E64B3580).
        let half = GgsciMapper::new("color_npg", npg, 0.5, Overflow::Auto, Color::BLACK);
        assert!(half.colors(3).iter().all(|color| color.alpha == 128));
        let colors = half.colors(12);
        assert_eq!(colors.len(), 12);
        assert!(colors.iter().all(|color| color.alpha == 128));
        assert_ne!(colors[10], colors[0]);
        let cycled = GgsciMapper::new("color_npg", npg, 1.0, Overflow::Cycle, Color::BLACK);
        assert_eq!(cycled.colors(12)[10], cycled.colors(12)[0]);
        assert_eq!(cycled.palette_size(), Some(10));
    }
}

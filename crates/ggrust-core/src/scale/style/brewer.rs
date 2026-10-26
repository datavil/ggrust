//! `scale_*_brewer()`: ColorBrewer palettes (lets-plot `ColorBrewerMapperProvider`,
//! `ColorPalette`, `PaletteUtil`). Categories take a palette's set for that many
//! colors (more than it has: the `overflow` rule); continuous data is quantized
//! into the palette's largest set.

use super::StyleMapper;
use super::brewer_data::{DIVERGING, QUALITATIVE, SEQUENTIAL};
use super::color::generate_colors;
use super::gradient::gradient_color;
use crate::data::ContinuousColumn;
use crate::scene::{Color, PerItem};
use crate::style::{StyleValues, ValueKind};

/// lets-plot `ColorPalette.MIN_COLOR_SET_SIZE`.
const MIN_SET_SIZE: usize = 3;

/// A palette family.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum PaletteType {
    /// `seq`: light to dark.
    Sequential,
    /// `div`: two hues around a light middle.
    Diverging,
    /// `qual`: distinct hues.
    Qualitative,
}

impl PaletteType {
    /// `seq`, `div` or `qual`.
    pub fn parse(name: &str) -> Option<Self> {
        match name {
            "seq" => Some(PaletteType::Sequential),
            "div" => Some(PaletteType::Diverging),
            "qual" => Some(PaletteType::Qualitative),
            _ => None,
        }
    }

    fn schemes(self) -> &'static [BrewerScheme] {
        match self {
            PaletteType::Sequential => SEQUENTIAL,
            PaletteType::Diverging => DIVERGING,
            PaletteType::Qualitative => QUALITATIVE,
        }
    }
}

/// One ColorBrewer scheme: its color sets for 3, 4, ... colors.
#[derive(Debug)]
pub struct BrewerScheme {
    /// ColorBrewer name, e.g. `Blues`.
    pub name: &'static str,
    /// Family.
    pub kind: PaletteType,
    /// Color sets, smallest (3 colors) first.
    pub sets: &'static [&'static [[u8; 3]]],
}

impl PartialEq for BrewerScheme {
    /// Names are unique.
    fn eq(&self, other: &Self) -> bool {
        self.name == other.name
    }
}

impl BrewerScheme {
    /// Size of the largest set.
    pub fn max_colors(&self) -> usize {
        self.sets.last().map_or(0, |set| set.len())
    }

    /// lets-plot `ColorPalette.colors`: the set for `count` colors, clamped to the
    /// sizes the scheme has.
    fn set(&self, count: usize) -> Vec<Color> {
        let size = count.clamp(MIN_SET_SIZE, self.max_colors());
        self.sets[size - MIN_SET_SIZE]
            .iter()
            .map(|&[red, green, blue]| Color::rgb(red, green, blue))
            .collect()
    }
}

/// Find a scheme by its exact name (`Blues`, `RdBu`, `Set2`, ...).
pub fn scheme_by_name(name: &str) -> Option<&'static BrewerScheme> {
    [SEQUENTIAL, DIVERGING, QUALITATIVE]
        .into_iter()
        .flatten()
        .find(|scheme| scheme.name == name)
}

/// Every scheme name of a family, for error messages.
pub fn scheme_names(kind: PaletteType) -> String {
    kind.schemes()
        .iter()
        .map(|scheme| format!("'{}'", scheme.name))
        .collect::<Vec<_>>()
        .join(", ")
}

/// What lets-plot does when more colors are needed than a set has.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Overflow {
    /// Qualitative: generate; sequential and diverging: interpolate.
    #[default]
    Auto,
    /// LAB gradient through the set.
    Interpolate,
    /// Repeat the set.
    Cycle,
    /// New colors from the set's hues, lightness shifted.
    Generate,
}

impl Overflow {
    /// `interpolate` (`i`), `cycle` (`c`), `generate` (`g`), any case.
    pub fn parse(name: &str) -> Option<Self> {
        match name.to_lowercase().as_str() {
            "interpolate" | "i" => Some(Overflow::Interpolate),
            "cycle" | "c" => Some(Overflow::Cycle),
            "generate" | "g" => Some(Overflow::Generate),
            _ => None,
        }
    }
}

/// lets-plot `PaletteUtil.schemeColors`.
fn scheme_colors(scheme: &BrewerScheme, count: usize, overflow: Overflow) -> Vec<Color> {
    let colors = scheme.set(count);
    if colors.len() >= count {
        return colors;
    }
    let overflow = match (overflow, scheme.kind) {
        (Overflow::Auto, PaletteType::Qualitative) => Overflow::Generate,
        (Overflow::Auto, _) => Overflow::Interpolate,
        (chosen, _) => chosen,
    };
    match overflow {
        Overflow::Cycle => (0..count)
            .map(|index| colors[index % colors.len()])
            .collect(),
        Overflow::Generate => generate_colors(&colors, count),
        // lets-plot `GradientUtil.interpolateColors`.
        _ => {
            let domain = (0.0, (count - 1) as f64);
            (0..count)
                .map(|index| {
                    gradient_color(
                        &colors,
                        domain,
                        index as f64,
                        Color::rgb(128, 128, 128),
                        1.0,
                    )
                })
                .collect()
        }
    }
}

/// The `palette` option: a name, or an index within the `type` family.
#[derive(Debug, Clone, PartialEq)]
pub enum PaletteChoice {
    /// No palette given.
    Default,
    /// `palette="Blues"`.
    Named(&'static BrewerScheme),
    /// `palette=2` (with `type`).
    Index(i64),
}

/// A ColorBrewer mapper.
#[derive(Debug, Clone, PartialEq)]
pub struct BrewerMapper {
    /// `type`: `seq`, `div` or `qual` (ignored when the palette is named).
    pub kind: Option<PaletteType>,
    /// `palette`.
    pub palette: PaletteChoice,
    /// `direction < 0`: colors reversed.
    pub reversed: bool,
    /// `overflow`.
    pub overflow: Overflow,
    /// Color of missing and unknown values.
    pub na: Color,
}

impl BrewerMapper {
    /// lets-plot `ColorBrewerMapperProvider.colorScheme`: discrete data defaults to
    /// qualitative `Set1`, continuous data to the first sequential scheme (`Blues`).
    fn scheme(&self, discrete: bool) -> &'static BrewerScheme {
        let kind = self.kind.unwrap_or(if discrete {
            PaletteType::Qualitative
        } else {
            PaletteType::Sequential
        });
        match &self.palette {
            PaletteChoice::Named(scheme) => scheme,
            PaletteChoice::Index(index) => {
                let schemes = kind.schemes();
                &schemes[index.rem_euclid(schemes.len() as i64) as usize]
            }
            PaletteChoice::Default if kind == PaletteType::Qualitative => {
                scheme_by_name("Set1").unwrap_or(&QUALITATIVE[0])
            }
            PaletteChoice::Default => &kind.schemes()[0],
        }
    }

    fn colors(&self, scheme: &BrewerScheme, count: usize) -> Vec<Color> {
        let mut colors = scheme_colors(scheme, count, self.overflow);
        if self.reversed {
            colors.reverse();
        }
        colors
    }
}

impl StyleMapper for BrewerMapper {
    fn name(&self) -> &'static str {
        "color_brewer"
    }

    fn value_kind(&self) -> ValueKind {
        ValueKind::Color
    }

    fn map_continuous(&self, column: &ContinuousColumn, domain: (f64, f64)) -> StyleValues {
        let scheme = self.scheme(false);
        let palette = self.colors(scheme, scheme.max_colors());
        // lets-plot `QuantizeScale`: equal buckets over the domain, clamped at both ends.
        let bucket = (domain.1 - domain.0) / palette.len() as f64;
        let colors = (0..column.len())
            .map(
                |row| match column.get(row).filter(|value| value.is_finite()) {
                    Some(value) => {
                        let index = ((value - domain.0) / bucket) as i64;
                        palette[index.clamp(0, palette.len() as i64 - 1) as usize]
                    }
                    None => self.na,
                },
            )
            .collect();
        StyleValues::Colors(PerItem::Varying(colors))
    }

    fn map_discrete(&self, indices: &[Option<usize>], level_count: usize) -> StyleValues {
        let palette = self.colors(self.scheme(true), level_count);
        let colors = indices
            .iter()
            .map(|index| match index {
                Some(index) if !palette.is_empty() => palette[index % palette.len()],
                _ => self.na,
            })
            .collect();
        StyleValues::Colors(PerItem::Varying(colors))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn sets_are_clamped_and_overflow_follows_the_family() {
        let blues = scheme_by_name("Blues").unwrap();
        assert_eq!(blues.max_colors(), 9);
        assert_eq!(scheme_colors(blues, 1, Overflow::Auto).len(), 3);
        assert_eq!(scheme_colors(blues, 12, Overflow::Auto).len(), 12);
        let set2 = scheme_by_name("Set2").unwrap();
        let cycled = scheme_colors(set2, 10, Overflow::Cycle);
        assert_eq!(cycled[8], cycled[0]);
        assert_eq!(scheme_by_name("set2"), None);
    }
}

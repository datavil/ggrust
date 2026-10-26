//! `scale_*_viridis()`: matplotlib color maps (lets-plot `ColormapMapperProvider`,
//! `ColorMaps`). Continuous data: a gradient through the whole (cut) map;
//! categories: `n` colors sampled evenly.

use super::StyleMapper;
use super::colormap_data::{CIVIDIS, INFERNO, MAGMA, PLASMA, TURBO, TWILIGHT, VIRIDIS};
use super::gradient::gradient_color;
use crate::data::ContinuousColumn;
use crate::scene::{Color, PerItem};
use crate::style::{StyleValues, ValueKind};

/// Names lets-plot accepts, for error messages.
pub const COLORMAP_NAMES: &str = "\"magma\" (or \"A\"), \"inferno\" (or \"B\"), \"plasma\" (or \"C\"), \
     \"viridis\" (or \"D\"), \"cividis\" (or \"E\"), \"turbo\" or \"twilight\"";

/// The table of a color map name (case and surrounding spaces ignored).
pub fn colormap(name: &str) -> Option<&'static [[u8; 3]]> {
    Some(match name.trim().to_lowercase().as_str() {
        "magma" | "a" => &MAGMA,
        "inferno" | "b" => &INFERNO,
        "plasma" | "c" => &PLASMA,
        "viridis" | "d" => &VIRIDIS,
        "cividis" | "e" => &CIVIDIS,
        "turbo" => &TURBO,
        "twilight" => &TWILIGHT,
        _ => return None,
    })
}

/// A color map with its options.
#[derive(Debug, Clone, PartialEq)]
pub struct ColormapMapper {
    /// The color table.
    pub table: &'static [[u8; 3]],
    /// Opacity 0..1 (`alpha`).
    pub opacity: f64,
    /// Part of the map used, 0..1 (`begin`, `end`).
    pub range: (f64, f64),
    /// `direction < 0`: colors reversed.
    pub reversed: bool,
    /// Color of missing and out-of-domain values.
    pub na: Color,
}

impl ColormapMapper {
    /// lets-plot `ColorMaps.getColors` (+ reversal): the whole cut range, or `n`.
    fn colors(&self, n: Option<usize>) -> Vec<Color> {
        let max_index = (self.table.len() - 1) as f64;
        let clamp = |index: f64| index.clamp(0.0, max_index) as usize;
        let from = clamp((self.range.0 * max_index).floor().round());
        let to = clamp((self.range.1 * max_index).ceil().round());
        let indices: Vec<usize> = match n {
            None | Some(0) => (from..=to.max(from)).collect(),
            Some(1) => {
                vec![(((to as f64 - from as f64) / 2.0 + from as f64) + 0.5).floor() as usize]
            }
            Some(n) => {
                let increment = (to as f64 - from as f64) / (n - 1) as f64;
                (0..n)
                    .map(|index| ((index as f64 * increment) + 0.5).floor() as usize + from)
                    .collect()
            }
        };
        let alpha = (self.opacity * 255.0 + 0.5).floor() as u8;
        let mut colors: Vec<Color> = indices
            .into_iter()
            .map(|index| {
                let [red, green, blue] = self.table[index.min(self.table.len() - 1)];
                Color::rgba(red, green, blue, alpha)
            })
            .collect();
        if self.reversed {
            colors.reverse();
        }
        colors
    }
}

impl StyleMapper for ColormapMapper {
    fn name(&self) -> &'static str {
        "color_cmap"
    }

    fn value_kind(&self) -> ValueKind {
        ValueKind::Color
    }

    fn map_continuous(&self, column: &ContinuousColumn, domain: (f64, f64)) -> StyleValues {
        let stops = self.colors(None);
        let colors = (0..column.len())
            .map(|row| {
                column.get(row).map_or(self.na, |value| {
                    gradient_color(&stops, domain, value, self.na, self.opacity)
                })
            })
            .collect();
        StyleValues::Colors(PerItem::Varying(colors))
    }

    fn map_discrete(&self, indices: &[Option<usize>], level_count: usize) -> StyleValues {
        let palette = self.colors(Some(level_count));
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
    fn three_viridis_categories() {
        let mapper = ColormapMapper {
            table: colormap("D").unwrap(),
            opacity: 1.0,
            range: (0.0, 1.0),
            reversed: false,
            na: Color::rgb(1, 2, 3),
        };
        let StyleValues::Colors(PerItem::Varying(colors)) =
            mapper.map_discrete(&[Some(0), Some(1), Some(2)], 3)
        else {
            panic!("expected colors");
        };
        assert_eq!(colors[0], Color::from_hex("#440154").unwrap());
        assert_eq!(colors[2], Color::from_hex("#fde725").unwrap());
    }
}

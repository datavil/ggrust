//! `scale_*_hue()`: colors evenly spaced around the HCL wheel (lets-plot
//! `ColorHueMapperProvider`, `GradientUtil.gradientHCL`).

use super::color_space::{Hcl, rgb_from_hcl};
use super::{StyleMapper, linear};
use crate::data::ContinuousColumn;
use crate::scene::{Color, PerItem};
use crate::style::{StyleValues, ValueKind};

/// Hue mapper options, lets-plot defaults in [`HueMapper::default_with`].
#[derive(Debug, Clone, PartialEq)]
pub struct HueMapper {
    /// Hue range in degrees (`h`), before `h_start` is added.
    pub hue_range: (f64, f64),
    /// Chroma (`c`).
    pub chroma: f64,
    /// Luminance (`l`).
    pub luminance: f64,
    /// Added to both ends of the hue range (`h_start`).
    pub start_hue: f64,
    /// `direction < 0`: colors run the other way.
    pub reversed: bool,
    /// Color of missing and out-of-domain values.
    pub na: Color,
}

impl HueMapper {
    /// lets-plot `ColorMapperDefaults.Hue`: hues 15..375, chroma 100, luminance 65.
    pub fn default_with(na: Color) -> Self {
        HueMapper {
            hue_range: (15.0, 375.0),
            chroma: 100.0,
            luminance: 65.0,
            start_hue: 0.0,
            reversed: false,
            na,
        }
    }

    fn shifted_range(&self) -> (f64, f64) {
        (
            self.hue_range.0 + self.start_hue,
            self.hue_range.1 + self.start_hue,
        )
    }

    /// `gradientHCL` from hue `low` to `high` over `domain`; outside is NA.
    fn gradient(&self, domain: (f64, f64), low: f64, high: f64) -> impl Fn(f64) -> Color + '_ {
        let (low, high) = if self.reversed {
            (high, low)
        } else {
            (low, high)
        };
        let hue = linear(domain, (low, high));
        move |value| {
            if !(value >= domain.0 && value <= domain.1) {
                return self.na;
            }
            let h = hue(value) % 360.0;
            rgb_from_hcl(Hcl {
                h: if h >= 0.0 { h } else { 360.0 + h },
                c: self.chroma,
                l: self.luminance,
            })
        }
    }
}

impl StyleMapper for HueMapper {
    fn name(&self) -> &'static str {
        "color_hue"
    }

    fn value_kind(&self) -> ValueKind {
        ValueKind::Color
    }

    fn map_continuous(&self, column: &ContinuousColumn, domain: (f64, f64)) -> StyleValues {
        let (low, high) = self.shifted_range();
        let color = self.gradient(domain, low, high);
        let colors = (0..column.len())
            .map(|row| column.get(row).map_or(self.na, &color))
            .collect();
        StyleValues::Colors(PerItem::Varying(colors))
    }

    fn map_discrete(&self, indices: &[Option<usize>], level_count: usize) -> StyleValues {
        // At least 2 steps; on a full circle the last color must not repeat the first.
        let n = level_count.max(2) as f64;
        let (low, mut high) = self.shifted_range();
        if (high - low) % 360.0 < 1.0 {
            high -= 360.0 / n;
        }
        let step = (high - low) / (n - 1.0);
        let last = level_count.saturating_sub(1) as f64;
        // lets-plot `ensureApplicableRange`: one category spans -0.5..0.5.
        let domain = if last > 0.0 { (0.0, last) } else { (-0.5, 0.5) };
        let color = self.gradient(domain, low, low + step * (n - 1.0));
        let colors = indices
            .iter()
            .map(|index| index.map_or(self.na, |index| color(index as f64)))
            .collect();
        StyleValues::Colors(PerItem::Varying(colors))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scale::style::NA_COLOR;

    fn colors(values: StyleValues) -> Vec<Color> {
        let StyleValues::Colors(PerItem::Varying(colors)) = values else {
            panic!("expected colors");
        };
        colors
    }

    #[test]
    fn three_categories_are_the_lets_plot_hues() {
        let mapper = HueMapper::default_with(NA_COLOR);
        let found = colors(mapper.map_discrete(&[Some(0), Some(1), Some(2), None], 3));
        let expected = ["#f8766d", "#00ba38", "#619cff", "#808080"];
        for (color, hex) in found.iter().zip(expected) {
            assert_eq!(*color, Color::from_hex(hex).unwrap(), "{hex}");
        }
    }

    #[test]
    fn one_category_takes_the_middle_of_two() {
        let mapper = HueMapper::default_with(NA_COLOR);
        let found = colors(mapper.map_discrete(&[Some(0)], 1));
        assert_eq!(found[0], Color::from_hex("#7cae00").unwrap());
    }
}

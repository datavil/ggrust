//! Gradients in LAB space: `scale_*_gradient()`, `scale_*_gradientn()` and
//! `scale_*_gradient2()` (lets-plot `GradientUtil.createGradient`,
//! `ColorGradient*MapperProvider`). Values outside the domain are NA.

use super::color_space::{Lab, lab_from_rgb, rgb_from_lab};
use super::{StyleMapper, linear};
use crate::data::ContinuousColumn;
use crate::scene::{Color, PerItem};
use crate::style::{StyleValues, ValueKind};

/// lets-plot `ColorMapperDefaults.Gradient2`.
pub const GRADIENT2_LOW: Color = Color::rgb(0x96, 0x45, 0x40);
/// lets-plot `ColorMapperDefaults.Gradient2.DEF_MID`.
pub const GRADIENT2_MID: Color = Color::WHITE;
/// lets-plot `ColorMapperDefaults.Gradient2.DEF_HIGH`.
pub const GRADIENT2_HIGH: Color = Color::rgb(0x3b, 0x3d, 0x96);

/// Two or more colors spread evenly over the domain.
#[derive(Debug, Clone, PartialEq)]
pub struct GradientMapper {
    /// Name for messages (`color_gradient`, `color_gradientn`, `color_cmap`).
    pub name: &'static str,
    /// The colors, at least two.
    pub colors: Vec<Color>,
    /// Color of missing and out-of-domain values.
    pub na: Color,
    /// Opacity of every color (1: opaque).
    pub opacity: f64,
}

impl GradientMapper {
    /// lets-plot `createGradient` over `domain`.
    pub fn color(&self, domain: (f64, f64), value: f64) -> Color {
        gradient_color(&self.colors, domain, value, self.na, self.opacity)
    }
}

/// `createGradient`: `colors` at evenly spaced stops over `domain`, LAB between stops.
pub(super) fn gradient_color(
    colors: &[Color],
    domain: (f64, f64),
    value: f64,
    na: Color,
    opacity: f64,
) -> Color {
    if !value.is_finite() || value < domain.0 || value > domain.1 || colors.len() < 2 {
        return na;
    }
    let segments = colors.len() - 1;
    let length = (domain.1 - domain.0) / segments as f64;
    // The last stop is exact; inner stops are low + length * i.
    let stop = |index: usize| {
        if index == segments {
            domain.1
        } else {
            domain.0 + length * index as f64
        }
    };
    // lets-plot `binarySearch`: an exact hit on a stop starts that segment.
    let mut segment = 0;
    while segment + 1 < segments && value >= stop(segment + 1) {
        segment += 1;
    }
    lab_gradient(
        (stop(segment), stop(segment + 1)),
        colors[segment],
        colors[segment + 1],
        value,
        na,
        opacity,
    )
}

/// lets-plot `gradientLAB`: `low` to `high` over `domain`; outside is NA.
fn lab_gradient(
    domain: (f64, f64),
    low: Color,
    high: Color,
    value: f64,
    na: Color,
    opacity: f64,
) -> Color {
    if !(value >= domain.0 && value <= domain.1) {
        return na;
    }
    let (low, high) = (lab_from_rgb(low), lab_from_rgb(high));
    let color = rgb_from_lab(Lab {
        l: linear(domain, (low.l, high.l))(value),
        a: linear(domain, (low.a, high.a))(value),
        b: linear(domain, (low.b, high.b))(value),
    });
    if opacity < 1.0 {
        color.with_opacity(opacity)
    } else {
        color
    }
}

/// lets-plot `ensureApplicableRange` over category indices 0..n-1.
pub(super) fn index_domain(level_count: usize) -> (f64, f64) {
    match level_count.saturating_sub(1) {
        0 => (-0.5, 0.5),
        last => (0.0, last as f64),
    }
}

impl StyleMapper for GradientMapper {
    fn name(&self) -> &'static str {
        self.name
    }

    fn value_kind(&self) -> ValueKind {
        ValueKind::Color
    }

    fn map_continuous(&self, column: &ContinuousColumn, domain: (f64, f64)) -> StyleValues {
        let colors = (0..column.len())
            .map(|row| {
                column
                    .get(row)
                    .map_or(self.na, |value| self.color(domain, value))
            })
            .collect();
        StyleValues::Colors(PerItem::Varying(colors))
    }

    fn map_discrete(&self, indices: &[Option<usize>], level_count: usize) -> StyleValues {
        let domain = index_domain(level_count);
        let colors = indices
            .iter()
            .map(|index| index.map_or(self.na, |index| self.color(domain, index as f64)))
            .collect();
        StyleValues::Colors(PerItem::Varying(colors))
    }
}

/// `scale_grey()` (lets-plot `GreyscaleLightnessMapperProvider`): greys from
/// lightness L* = `start` x 100 to `end` x 100, interpolated before rounding.
#[derive(Debug, Clone, PartialEq)]
pub struct GreyMapper {
    /// Lightness at the low end, 0..1.
    pub start: f64,
    /// Lightness at the high end, 0..1.
    pub end: f64,
    /// Color of missing and out-of-domain values.
    pub na: Color,
}

impl GreyMapper {
    /// The grey of `value` over `domain`; outside is NA.
    pub fn color(&self, domain: (f64, f64), value: f64) -> Color {
        if !(value.is_finite() && value >= domain.0 && value <= domain.1) {
            return self.na;
        }
        rgb_from_lab(Lab {
            l: linear(domain, (self.start * 100.0, self.end * 100.0))(value),
            a: 0.0,
            b: 0.0,
        })
    }
}

impl StyleMapper for GreyMapper {
    fn name(&self) -> &'static str {
        "color_grey"
    }

    fn value_kind(&self) -> ValueKind {
        ValueKind::Color
    }

    fn map_continuous(&self, column: &ContinuousColumn, domain: (f64, f64)) -> StyleValues {
        let colors = (0..column.len())
            .map(|row| {
                column
                    .get(row)
                    .map_or(self.na, |value| self.color(domain, value))
            })
            .collect();
        StyleValues::Colors(PerItem::Varying(colors))
    }

    fn map_discrete(&self, indices: &[Option<usize>], level_count: usize) -> StyleValues {
        let domain = index_domain(level_count);
        let colors = indices
            .iter()
            .map(|index| index.map_or(self.na, |index| self.color(domain, index as f64)))
            .collect();
        StyleValues::Colors(PerItem::Varying(colors))
    }
}

/// `scale_*_gradient2()`: `low` -> `mid` below the midpoint, `mid` -> `high` above.
/// Continuous data only.
#[derive(Debug, Clone, PartialEq)]
pub struct Gradient2Mapper {
    /// Color at the low end.
    pub low: Color,
    /// Color at the midpoint.
    pub mid: Color,
    /// Color at the high end.
    pub high: Color,
    /// Data value of `mid`.
    pub midpoint: f64,
    /// Color of missing and out-of-domain values.
    pub na: Color,
}

impl Gradient2Mapper {
    /// lets-plot `ColorGradient2MapperProvider`: of the two half domains containing
    /// the value, the shorter non-empty one wins.
    pub fn color(&self, domain: (f64, f64), value: f64) -> Color {
        if !value.is_finite() {
            return self.na;
        }
        let low = (domain.0, self.midpoint.max(domain.0));
        let high = (self.midpoint.min(domain.1), domain.1);
        let mut chosen: Option<((f64, f64), Color, Color)> = None;
        for (range, from, to) in [(low, self.low, self.mid), (high, self.mid, self.high)] {
            if !(value >= range.0 && value <= range.1) {
                continue;
            }
            let span = range.1 - range.0;
            chosen = match chosen {
                None => Some((range, from, to)),
                Some((current, ..)) if current.1 - current.0 == 0.0 => Some((range, from, to)),
                Some((current, ..)) if span < current.1 - current.0 && span > 0.0 => {
                    Some((range, from, to))
                }
                keep => keep,
            };
        }
        match chosen {
            Some((range, from, to)) => lab_gradient(range, from, to, value, self.na, 1.0),
            None => self.na,
        }
    }
}

impl StyleMapper for Gradient2Mapper {
    fn name(&self) -> &'static str {
        "color_gradient2"
    }

    fn value_kind(&self) -> ValueKind {
        ValueKind::Color
    }

    fn continuous_only(&self) -> bool {
        true
    }

    fn map_continuous(&self, column: &ContinuousColumn, domain: (f64, f64)) -> StyleValues {
        let colors = (0..column.len())
            .map(|row| {
                column
                    .get(row)
                    .map_or(self.na, |value| self.color(domain, value))
            })
            .collect();
        StyleValues::Colors(PerItem::Varying(colors))
    }

    fn map_discrete(&self, indices: &[Option<usize>], _level_count: usize) -> StyleValues {
        // Never called: discrete data is refused (`continuous_only`).
        StyleValues::Colors(PerItem::Varying(vec![self.na; indices.len()]))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Measured from lets-plot `scale_grey('color', start, end)` over 0..10.
    #[test]
    fn grey_matches_lets_plot() {
        for (start, end, expected) in [
            (
                0.0,
                1.0,
                [
                    0x00, 0x1b, 0x30, 0x47, 0x5e, 0x77, 0x91, 0xab, 0xc6, 0xe2, 0xff,
                ],
            ),
            (
                0.2,
                0.8,
                [
                    0x30, 0x3e, 0x4b, 0x59, 0x68, 0x77, 0x86, 0x96, 0xa6, 0xb6, 0xc6,
                ],
            ),
            (
                0.0,
                0.0431,
                [
                    0x00, 0x02, 0x03, 0x05, 0x06, 0x08, 0x09, 0x0b, 0x0c, 0x0e, 0x0f,
                ],
            ),
        ] {
            let mapper = GreyMapper {
                start,
                end,
                na: Color::TRANSPARENT,
            };
            let greys: Vec<u8> = (0..=10)
                .map(|value| mapper.color((0.0, 10.0), f64::from(value)))
                .inspect(|color| assert_eq!((color.red, color.red), (color.green, color.blue)))
                .map(|color| color.red)
                .collect();
            assert_eq!(greys, expected, "start {start}, end {end}");
        }
    }

    #[test]
    fn ends_and_stops_are_exact_and_outside_is_na() {
        let mapper = GradientMapper {
            name: "color_gradientn",
            colors: vec![
                Color::rgb(255, 0, 0),
                Color::rgb(0, 255, 0),
                Color::rgb(0, 0, 255),
            ],
            na: Color::rgb(1, 2, 3),
            opacity: 1.0,
        };
        assert_eq!(mapper.color((0.0, 10.0), 0.0), Color::rgb(255, 0, 0));
        assert_eq!(mapper.color((0.0, 10.0), 5.0), Color::rgb(0, 255, 0));
        assert_eq!(mapper.color((0.0, 10.0), 10.0), Color::rgb(0, 0, 255));
        assert_eq!(mapper.color((0.0, 10.0), 10.5), Color::rgb(1, 2, 3));
    }

    #[test]
    fn gradient2_is_white_at_the_midpoint() {
        let mapper = Gradient2Mapper {
            low: GRADIENT2_LOW,
            mid: GRADIENT2_MID,
            high: GRADIENT2_HIGH,
            midpoint: 0.0,
            na: Color::rgb(1, 2, 3),
        };
        assert_eq!(mapper.color((-1.0, 1.0), 0.0), Color::WHITE);
        assert_eq!(mapper.color((-1.0, 1.0), -1.0), GRADIENT2_LOW);
        assert_eq!(mapper.color((-1.0, 1.0), 1.0), GRADIENT2_HIGH);
    }
}

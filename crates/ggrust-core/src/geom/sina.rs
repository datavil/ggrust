//! `geom_sina`: points spread across the violin outline, as lets-plot's
//! `SinaGeom`. Drawing is [`PointGeom`]'s.
//!
//! Each point moves sideways by `sign x u x violinwidth x width x resolution(x) / 2`
//! (`u` uniform in [0, 1), `sign` random, or fixed by `show_half`). Random draws
//! follow lets-plot: Kotlin's `Random(seed)`, points grouped by x in order of
//! first appearance and sorted by y, a sign then `u` per point. Without a `seed`
//! the jitter's default seed keeps renders reproducible.
//!
//! Setup runs before the default dodge: it spans each x by `xmin`/`xmax` (the
//! violin's width, which trains the x axis as lets-plot's does) and keeps the
//! spread point in `xend`, so dodging moves and narrows it as lets-plot's
//! position does; drawing reads it.
//!
//! What it does not do: draw the `quantiles` bands apart (one point group) or
//! `orientation="y"`.

use std::collections::HashMap;

use super::{DrawLayer, Geom, PointGeom};
use crate::Result;
use crate::coord::Panel;
use crate::data::{Column, ContinuousColumn, LayerData};
use crate::parameters::Parameters;
use crate::position::DefaultPosition;
use crate::position::random::{DEFAULT_SEED, KotlinRandom};
use crate::scene::{Node, Rect};
use crate::spec::Aesthetic;
use crate::style::{StyleDefault, Styles};

/// Points spread within the violin outline.
pub struct SinaGeom;

impl Geom for SinaGeom {
    fn tooltip_behavior(&self) -> crate::tooltip::TooltipBehavior {
        crate::tooltip::TooltipBehavior::NEAREST_XY
    }

    fn hover_targets(
        &self,
        layer: &DrawLayer,
        panel: &Panel,
    ) -> Result<Option<crate::tooltip::HoverTargets>> {
        let data = spread(layer.data)?;
        PointGeom.hover_targets(
            &DrawLayer {
                data: &data,
                ..*layer
            },
            panel,
        )
    }

    fn name(&self) -> &'static str {
        "sina"
    }

    fn default_stat(&self) -> &'static str {
        "sina"
    }

    fn default_position(&self) -> DefaultPosition {
        DefaultPosition::dodge(0.95)
    }

    fn required_aesthetics(&self) -> &'static [Aesthetic] {
        &[Aesthetic::X, Aesthetic::Y, Aesthetic::Violinwidth]
    }

    fn style_defaults(&self) -> &'static [(Aesthetic, StyleDefault)] {
        PointGeom.style_defaults()
    }

    fn parameters(&self) -> &'static [&'static str] {
        &["width", "seed", "show_half", "quantiles", "size_unit"]
    }

    fn setup_aesthetics(&self) -> &'static [Aesthetic] {
        &[Aesthetic::Xmin, Aesthetic::Xmax, Aesthetic::Xend]
    }

    fn span_aesthetics(&self) -> &'static [Aesthetic] {
        &[Aesthetic::Xmin, Aesthetic::Xmax]
    }

    fn setup(&self, mut data: LayerData, parameters: &Parameters) -> Result<LayerData> {
        let (Some(x), Some(y), Some(violin_width)) = (
            data.continuous(Aesthetic::X).cloned(),
            data.continuous(Aesthetic::Y).cloned(),
            data.continuous(Aesthetic::Violinwidth).cloned(),
        ) else {
            return Ok(data);
        };
        let width = parameters.number("width")?.unwrap_or(1.0);
        let show_half = parameters.number("show_half")?.unwrap_or(0.0);
        let seed = parameters
            .number("seed")?
            .map_or(DEFAULT_SEED, |seed| seed as i64);
        let half = data.resolution(Aesthetic::X) / 2.0 * width;
        let finite = |column: &ContinuousColumn, row: usize| {
            column.get(row).filter(|value| value.is_finite())
        };

        // Rows per x in order of first appearance, then by y (stable).
        let mut slot_of_x: HashMap<u64, usize> = HashMap::new();
        let mut by_x: Vec<Vec<(usize, f64)>> = Vec::new();
        for row in 0..data.row_count() {
            let (Some(location), Some(value), Some(_)) =
                (finite(&x, row), finite(&y, row), finite(&violin_width, row))
            else {
                continue;
            };
            let slot = *slot_of_x.entry(location.to_bits()).or_insert_with(|| {
                by_x.push(Vec::new());
                by_x.len() - 1
            });
            by_x[slot].push((row, value));
        }
        let mut random = KotlinRandom::new(seed);
        let mut spread_x = vec![f64::NAN; data.row_count()];
        for mut rows in by_x {
            rows.sort_by(|left, right| left.1.total_cmp(&right.1));
            for (row, _) in rows {
                let sign = if show_half > 0.0 {
                    1.0
                } else if show_half < 0.0 {
                    -1.0
                } else if random.next_boolean() {
                    1.0
                } else {
                    -1.0
                };
                let shift = random.next_double();
                let limit = half * violin_width.get(row).unwrap_or(0.0);
                spread_x[row] = x.get(row).unwrap_or(f64::NAN) + sign * shift * limit;
            }
        }
        let continuous = |values| Column::Continuous(ContinuousColumn::from_values(values));
        let location = |row: usize| x.get(row).unwrap_or(f64::NAN);
        let row_count = data.row_count();
        let span = |sign: f64| {
            (0..row_count)
                .map(|row| location(row) + sign * half)
                .collect()
        };
        data.insert(Aesthetic::Xmin, continuous(span(-1.0)))?;
        data.insert(Aesthetic::Xmax, continuous(span(1.0)))?;
        data.insert(Aesthetic::Xend, continuous(spread_x))?;
        Ok(data)
    }

    fn draw(&self, layer: &DrawLayer, panel: &Panel) -> Result<Vec<Node>> {
        let data = spread(layer.data)?;
        PointGeom.draw(
            &DrawLayer {
                data: &data,
                ..*layer
            },
            panel,
        )
    }

    fn legend_defaults(&self) -> &'static [(Aesthetic, StyleDefault)] {
        PointGeom.legend_defaults()
    }

    fn minimum_key_size(&self, styles: &Styles) -> Result<(f64, f64)> {
        PointGeom.minimum_key_size(styles)
    }

    fn draw_key(&self, styles: &Styles, cell: Rect) -> Result<Vec<Node>> {
        PointGeom.draw_key(styles, cell)
    }
}

/// The layer with each point at its spread x (kept in `xend` by setup).
fn spread(data: &LayerData) -> Result<LayerData> {
    let mut data = data.clone();
    if let Some(spread) = data.get(Aesthetic::Xend).cloned() {
        data.insert(Aesthetic::X, spread)?;
    }
    Ok(data)
}

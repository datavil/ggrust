//! `geom_slab` (R package ggdist): a density drawn as a filled shape on one side
//! of its x position (or both), the slab of `stat_halfeye` / `stat_slab`.
//!
//! Setup gives every row the region the slab may fill: `width` (default 1) x
//! the x resolution, starting at x on the slab's `side` (`"topright"`, default,
//! `"bottomleft"`, or centred for `"both"`), so dodging moves and narrows
//! slabs. Thickness is normalized by `normalize`: `"all"` (default; the layer's
//! largest thickness is 1), `"groups"` (each slab's largest is 1) or `"none"`.
//! A thickness of 1 reaches `scale` (default 0.9) of the region.
//!
//! Each slab (group and x) is one closed polygon from its baseline at x to the
//! scaled thickness, filled (`fill`, default gray65, faded by `alpha`) and
//! outlined by `color` (none by default).
//!
//! What it does not do: `"all"` across facet panels (each panel normalizes on
//! its own), `fill_ramp`/gradients, `justification`.

use std::collections::HashMap;

use super::bar::{draw_key, key_size};
use super::lines;
use super::{DrawLayer, Geom};
use crate::coord::Panel;
use crate::data::{Column, ContinuousColumn, LayerData};
use crate::parameters::Parameters;
use crate::scene::{Color, Node, Path, Rect};
use crate::spec::Aesthetic;
use crate::style::{StyleDefault, Styles};
use crate::{Error, Result};

/// ggdist slab style: gray65 fill, no outline.
const STYLE_DEFAULTS: &[(Aesthetic, StyleDefault)] = &[
    (Aesthetic::Color, StyleDefault::Color(Color::TRANSPARENT)),
    (
        Aesthetic::Fill,
        StyleDefault::Color(Color::rgb(166, 166, 166)),
    ),
    (Aesthetic::Alpha, StyleDefault::Number(f64::NAN)),
    (Aesthetic::Size, StyleDefault::Number(0.5)),
    (Aesthetic::Linetype, StyleDefault::Linetype(&[])),
];

/// ggdist `scale`.
const DEFAULT_SCALE: f64 = 0.9;

/// Where a slab grows from its baseline.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub(crate) enum SlabSide {
    /// Towards larger x (up when oriented along y).
    TopRight,
    /// Towards smaller x.
    BottomLeft,
    /// Mirrored around x.
    Both,
}

impl SlabSide {
    const NAMES: [&'static str; 7] = [
        "topright",
        "top",
        "right",
        "bottomleft",
        "bottom",
        "left",
        "both",
    ];

    pub(crate) fn from_parameters(parameters: &Parameters) -> Result<Self> {
        match parameters.string("side")? {
            None | Some("topright" | "top" | "right") => Ok(SlabSide::TopRight),
            Some("bottomleft" | "bottom" | "left") => Ok(SlabSide::BottomLeft),
            Some("both") => Ok(SlabSide::Both),
            Some(other) => Err(Error::spec(
                parameters.path_of("side"),
                format!(
                    "unknown side '{other}'{}; use one of: {}",
                    crate::suggest::did_you_mean(other, SlabSide::NAMES),
                    SlabSide::NAMES.join(", ")
                ),
            )),
        }
    }
}

/// How thickness is scaled before drawing.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Normalize {
    All,
    Groups,
    None,
}

impl Normalize {
    fn from_parameters(parameters: &Parameters) -> Result<Self> {
        match parameters.string("normalize")? {
            None | Some("all") => Ok(Normalize::All),
            Some("groups") => Ok(Normalize::Groups),
            Some("none") => Ok(Normalize::None),
            Some(other) => Err(Error::spec(
                parameters.path_of("normalize"),
                format!(
                    "unsupported normalize '{other}'{}; use one of: all, groups, none",
                    crate::suggest::did_you_mean(other, ["all", "groups", "none"])
                ),
            )),
        }
    }
}

/// Density slabs.
pub struct SlabGeom;

impl Geom for SlabGeom {
    fn name(&self) -> &'static str {
        "slab"
    }

    fn default_stat(&self) -> &'static str {
        "slab"
    }

    fn required_aesthetics(&self) -> &'static [Aesthetic] {
        &[Aesthetic::X, Aesthetic::Y, Aesthetic::Thickness]
    }

    fn style_defaults(&self) -> &'static [(Aesthetic, StyleDefault)] {
        STYLE_DEFAULTS
    }

    fn parameters(&self) -> &'static [&'static str] {
        &["width", "scale", "side", "normalize"]
    }

    fn setup_aesthetics(&self) -> &'static [Aesthetic] {
        &[Aesthetic::Xmin, Aesthetic::Xmax]
    }

    fn span_aesthetics(&self) -> &'static [Aesthetic] {
        &[Aesthetic::Xmin, Aesthetic::Xmax]
    }

    fn setup(&self, mut data: LayerData, parameters: &Parameters) -> Result<LayerData> {
        let side = SlabSide::from_parameters(parameters)?;
        let normalize = Normalize::from_parameters(parameters)?;
        let scale = parameters.number("scale")?.unwrap_or(DEFAULT_SCALE);
        if scale < 0.0 {
            return Err(Error::spec(
                parameters.path_of("scale"),
                format!("scale must be zero or positive, got {scale}"),
            ));
        }
        let (Some(x), Some(thickness)) = (
            data.continuous(Aesthetic::X).cloned(),
            data.continuous(Aesthetic::Thickness).cloned(),
        ) else {
            return Ok(data);
        };
        let width = parameters.number("width")?.unwrap_or(1.0);
        let region = width * data.resolution(Aesthetic::X);
        let (below, above) = match side {
            SlabSide::TopRight => (0.0, region),
            SlabSide::BottomLeft => (region, 0.0),
            SlabSide::Both => (region / 2.0, region / 2.0),
        };
        let row_count = data.row_count();
        let location = |row: usize| x.get(row).unwrap_or(f64::NAN);
        let continuous = |values| Column::Continuous(ContinuousColumn::from_values(values));
        data.insert(
            Aesthetic::Xmin,
            continuous((0..row_count).map(|row| location(row) - below).collect()),
        )?;
        data.insert(
            Aesthetic::Xmax,
            continuous((0..row_count).map(|row| location(row) + above).collect()),
        )?;
        data.insert(Aesthetic::Width, continuous(vec![width; row_count]))?;
        let value = |row: usize| thickness.get(row).filter(|value| value.is_finite());
        let largest_of =
            |rows: &mut dyn Iterator<Item = usize>| rows.filter_map(value).fold(0.0_f64, f64::max);
        let divisors: Vec<f64> = match normalize {
            Normalize::None => vec![1.0; row_count],
            Normalize::All => vec![largest_of(&mut (0..row_count)); row_count],
            Normalize::Groups => {
                let mut largest: HashMap<(u32, u64), f64> = HashMap::new();
                for row in 0..row_count {
                    let key = (data.group_of(row), location(row).to_bits());
                    let entry = largest.entry(key).or_insert(0.0);
                    *entry = entry.max(value(row).unwrap_or(0.0));
                }
                (0..row_count)
                    .map(|row| largest[&(data.group_of(row), location(row).to_bits())])
                    .collect()
            }
        };
        let scaled = (0..row_count)
            .map(|row| match value(row) {
                Some(value) if divisors[row] > 0.0 => value / divisors[row] * scale,
                Some(_) => 0.0,
                None => f64::NAN,
            })
            .collect();
        data.insert(Aesthetic::Thickness, continuous(scaled))?;
        Ok(data)
    }

    fn draw(&self, layer: &DrawLayer, panel: &Panel) -> Result<Vec<Node>> {
        let side = SlabSide::from_parameters(&layer.parameters)?;
        let column = |aesthetic| {
            layer
                .data
                .continuous(aesthetic)
                .ok_or_else(|| Error::data(format!("slab is missing its '{aesthetic}' column")))
        };
        let (x, y, xmin, xmax, thickness) = (
            column(Aesthetic::X)?,
            column(Aesthetic::Y)?,
            column(Aesthetic::Xmin)?,
            column(Aesthetic::Xmax)?,
            column(Aesthetic::Thickness)?,
        );
        let finite = |column: &ContinuousColumn, row: usize| {
            column.get(row).filter(|value| value.is_finite())
        };
        // Rows of each slab (group and x), in order of first appearance.
        let mut slabs: Vec<Vec<usize>> = Vec::new();
        let mut slot_of: HashMap<(u32, u64), usize> = HashMap::new();
        for row in 0..layer.data.row_count() {
            let defined = [x, y, xmin, xmax, thickness]
                .iter()
                .all(|column| finite(column, row).is_some());
            if !defined {
                continue;
            }
            let key = (
                layer.data.group_of(row),
                x.get(row).unwrap_or(0.0).to_bits(),
            );
            let slot = *slot_of.entry(key).or_insert_with(|| {
                slabs.push(Vec::new());
                slabs.len() - 1
            });
            slabs[slot].push(row);
        }
        let fills = layer.styles.colors(Aesthetic::Fill)?;
        let alphas = layer.styles.numbers(Aesthetic::Alpha)?;
        let mut nodes = Vec::new();
        for mut rows in slabs {
            rows.sort_by(|left, right| {
                y.get(*left)
                    .unwrap_or(0.0)
                    .total_cmp(&y.get(*right).unwrap_or(0.0))
            });
            // Offset of the outer edge (and of the inner one, mirrored for "both").
            let reach = |row: usize| {
                let region = xmax.get(row).unwrap_or(0.0) - xmin.get(row).unwrap_or(0.0);
                let extent = thickness.get(row).unwrap_or(0.0) * region;
                match side {
                    SlabSide::TopRight => (extent, 0.0),
                    SlabSide::BottomLeft => (-extent, 0.0),
                    SlabSide::Both => (extent / 2.0, -extent / 2.0),
                }
            };
            let at = |row: usize, offset: f64| {
                panel.map(
                    x.get(row).unwrap_or(0.0) + offset,
                    y.get(row).unwrap_or(0.0),
                )
            };
            let mut outline: Vec<(f64, f64)> =
                rows.iter().map(|&row| at(row, reach(row).0)).collect();
            outline.extend(rows.iter().rev().map(|&row| at(row, reach(row).1)));
            let first = rows[0];
            let fill = fills.get(first).with_opacity(alphas.get(first));
            nodes.push(Node::Path(Path {
                points: outline,
                closed: true,
                fill: (fill.alpha > 0).then_some(fill),
                stroke: lines::stroke_with_alpha(layer.styles, first, false)?,
            }));
        }
        Ok(nodes)
    }

    fn minimum_key_size(&self, styles: &Styles) -> Result<(f64, f64)> {
        key_size(styles)
    }

    fn draw_key(&self, styles: &Styles, cell: Rect) -> Result<Vec<Node>> {
        draw_key(styles, cell)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::spec::Options;

    fn setup(thickness: &[f64], groups: &[u32], options: &str) -> LayerData {
        let mut data = LayerData::new(thickness.len());
        let column = |values: Vec<f64>| Column::Continuous(ContinuousColumn::from_values(values));
        data.insert(Aesthetic::X, column(vec![1.0; thickness.len()]))
            .unwrap();
        data.insert(
            Aesthetic::Y,
            column((0..thickness.len()).map(|v| v as f64).collect()),
        )
        .unwrap();
        data.insert(Aesthetic::Thickness, column(thickness.to_vec()))
            .unwrap();
        data.set_group_index(Some(groups.to_vec())).unwrap();
        let options: Options = serde_json::from_str(options).unwrap();
        SlabGeom
            .setup(data, &Parameters::new(&options, "layers[0]"))
            .unwrap()
    }

    fn values(data: &LayerData, aesthetic: Aesthetic) -> Vec<f64> {
        let column = data.continuous(aesthetic).unwrap();
        (0..column.len())
            .map(|row| column.get(row).unwrap())
            .collect()
    }

    #[test]
    fn layer_wide_normalization_and_region_on_the_side() {
        let data = setup(&[1.0, 2.0, 4.0, 1.0], &[0, 0, 1, 1], "{}");
        assert_eq!(
            values(&data, Aesthetic::Thickness),
            [0.225, 0.45, 0.9, 0.225]
        );
        assert_eq!(values(&data, Aesthetic::Xmin), [1.0; 4]);
        assert_eq!(values(&data, Aesthetic::Xmax), [2.0; 4]);
        let data = setup(
            &[1.0, 2.0, 4.0, 1.0],
            &[0, 0, 1, 1],
            r#"{"normalize": "groups", "scale": 1, "side": "both"}"#,
        );
        assert_eq!(values(&data, Aesthetic::Thickness), [0.5, 1.0, 1.0, 0.25]);
        assert_eq!(values(&data, Aesthetic::Xmin), [0.5; 4]);
    }

    #[test]
    fn options_are_checked() {
        let mut data = LayerData::new(1);
        data.insert(
            Aesthetic::X,
            Column::Continuous(ContinuousColumn::from_values(vec![1.0])),
        )
        .unwrap();
        let options: Options = serde_json::from_str(r#"{"side": "rigth"}"#).unwrap();
        let error = SlabGeom
            .setup(data, &Parameters::new(&options, "layers[0]"))
            .unwrap_err()
            .to_string();
        assert!(
            error.contains("layers[0].side") && error.contains("did you mean 'right'"),
            "{error}"
        );
    }
}

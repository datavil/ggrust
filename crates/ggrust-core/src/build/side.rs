//! Side panels in the plot builder: the band each layer draws in, the bands' own
//! scales, how they train, what the layout is asked for, and side layers dropped
//! from faceted plots.
//!
//! A side layer stays in its panel's layer list (legends, tooltips and toggles
//! count every layer), tagged with its band. Along the shared axis it maps
//! through the main scale; across it through its band's own scale.
//!
//! What it does not do: parse the spec keys (`spec::side`) or lay out and decorate
//! a band (`layout::side_panel`).

use super::{MappedLayer, PlotData, PreparedLayer, overall_range, train_positions};
use crate::layout::side::Side;
use crate::layout::side_panel::{BandRequest, BandSize};
use crate::scale::{AxisScaleSpec, PositionScale, PositionScales};
use crate::spec::side::{BandKey, BandStyle, DEFAULT_MARGIN_SIZE, LayerSide};
use crate::spec::{Aesthetic, PlotSpec, Value};
use crate::stat::StatContext;
use crate::theme::Theme;
use crate::{Error, Result};

/// A band and what its layers ask of it (before its scale is built).
#[derive(Debug, Clone, PartialEq)]
pub(super) struct BandSetup {
    pub key: BandKey,
    /// ggside if any of its layers is a ggside layer, else marginal.
    pub style: BandStyle,
    /// The largest size its layers of that style give: `side_size` (share of the
    /// panel) for a ggside band, `margin_size` for a marginal one; `None`: the
    /// default (`ggside_panel_scale_*`, lets-plot's 0.1).
    pub size: Option<f64>,
    /// The `side_band` name of its layers (`None`: the side's unnamed band).
    pub name: Option<String>,
}

/// A prepared band: its setup and its own trained scale (data terms: the y scale
/// of an x band).
#[derive(Debug, Clone)]
pub(super) struct SideBand {
    pub setup: BandSetup,
    pub cross: PositionScale,
}

/// The bands of a plot (sorted by key) and each layer's band index.
///
/// Layers on the same side with the same `side_band` name (or none) share a band.
/// The bands on one side get their slot, from the panel outward, in the order of
/// their first layer.
pub(super) fn band_setups(sides: &[Option<LayerSide>]) -> (Vec<BandSetup>, Vec<Option<usize>>) {
    let mut setups: Vec<BandSetup> = Vec::new();
    let same = |setup: &BandSetup, side: &LayerSide| {
        setup.key.shares_x == side.band.shares_x
            && setup.key.far == side.band.far
            && setup.name == side.name
    };
    for side in sides.iter().flatten() {
        match setups.iter_mut().find(|setup| same(setup, side)) {
            Some(setup) => {
                if side.style == BandStyle::Ggside {
                    setup.style = BandStyle::Ggside;
                }
            }
            None => {
                let slot = setups
                    .iter()
                    .filter(|setup| {
                        setup.key.shares_x == side.band.shares_x && setup.key.far == side.band.far
                    })
                    .count();
                setups.push(BandSetup {
                    key: BandKey { slot, ..side.band },
                    style: side.style,
                    size: None,
                    name: side.name.clone(),
                });
            }
        }
    }
    // Sizes count only from layers of the band's final style (a margin's
    // `margin_size` is not a ggside panel share).
    for side in sides.iter().flatten() {
        if let (Some(size), Some(setup)) =
            (side.size, setups.iter_mut().find(|setup| same(setup, side)))
            && side.style == setup.style
        {
            setup.size = Some(setup.size.map_or(size, |current| current.max(size)));
        }
    }
    let bands: Vec<Option<usize>> = sides
        .iter()
        .map(|side| {
            side.as_ref()
                .and_then(|side| setups.iter().position(|setup| same(setup, side)))
        })
        .collect();
    // Sort by key, keeping each layer's index pointing at its band.
    let mut order: Vec<usize> = (0..setups.len()).collect();
    order.sort_by_key(|index| setups[*index].key);
    let mut new_index = vec![0; setups.len()];
    for (new, old) in order.iter().enumerate() {
        new_index[*old] = new;
    }
    let sorted = order.iter().map(|index| setups[*index].clone()).collect();
    let bands = bands
        .into_iter()
        .map(|band| band.map(|band| new_index[band]))
        .collect();
    (sorted, bands)
}

impl BandSetup {
    /// The data aesthetic across the band (y for a band sharing x).
    pub fn cross_aesthetic(&self) -> Aesthetic {
        if self.key.shares_x {
            Aesthetic::Y
        } else {
            Aesthetic::X
        }
    }

    /// The screen side of the band (`flipped`: `coord_flip`).
    pub fn screen_side(&self, flipped: bool) -> Side {
        let shares_horizontal = self.key.shares_x != flipped;
        match (shares_horizontal, self.key.far) {
            (true, true) => Side::Top,
            (true, false) => Side::Bottom,
            (false, true) => Side::Right,
            (false, false) => Side::Left,
        }
    }

    /// The options of the band's own scale: `scale_xsidey_*()` / `scale_ysidex_*()`
    /// for a ggside band; a margin's grows away from the panel below and left.
    /// A ggside band takes the most specific `scale_*side*_*(side_pos=, side_band=)`
    /// that targets it (see [`PositionScales::side_spec`]).
    pub fn cross_spec(&self, scales: &PositionScales) -> AxisScaleSpec {
        if self.style == BandStyle::Ggside
            && let Some(spec) =
                scales.side_spec(self.key.shares_x, self.key.far, self.name.as_deref())
        {
            return spec.clone();
        }
        match (self.style, self.key.shares_x) {
            (BandStyle::Ggside, true) => scales.xside_y.clone(),
            (BandStyle::Ggside, false) => scales.yside_x.clone(),
            (BandStyle::Marginal, _) if self.key.far => AxisScaleSpec::default(),
            (BandStyle::Marginal, _) => AxisScaleSpec::away_from_panel(),
        }
    }
}

/// Whether `layer` maps through its band's own scale on data axis `axis_x` (x:
/// true): the band's cross axis. `None` for main-panel layers and shared axes.
pub(super) fn band_on_cross(
    band: Option<usize>,
    axis_x: bool,
    setups: &[BandSetup],
) -> Option<usize> {
    band.filter(|band| setups[*band].key.shares_x != axis_x)
}

/// Whether a layer in `band` (or the main panel) uses the main scale of data axis
/// `axis_x`: main layers do, side layers on the shared axis do.
pub(super) fn uses_main_axis(band: Option<usize>, axis_x: bool, setups: &[BandSetup]) -> bool {
    band.is_none_or(|band| setups[band].key.shares_x == axis_x)
}

/// The stat context of each band: the main panel's range along the shared axis,
/// the band's layers' range across it.
pub(super) fn band_contexts<'a>(
    setups: &[BandSetup],
    scales: &[PositionScale],
    mapped: &[MappedLayer],
    main: StatContext<'a>,
) -> Vec<StatContext<'a>> {
    setups
        .iter()
        .zip(scales)
        .enumerate()
        .map(|(band, (setup, cross))| {
            let layers: Vec<&MappedLayer> = mapped
                .iter()
                .filter(|layer| layer.band == Some(band))
                .collect();
            let range = overall_range(&layers, setup.cross_aesthetic(), cross);
            if setup.key.shares_x {
                StatContext {
                    y_range: range,
                    ..main
                }
            } else {
                StatContext {
                    x_range: range,
                    ..main
                }
            }
        })
        .collect()
}

/// Train the scales on the side layers of a panel: the band's own scale across
/// it, the main scale along it (ggside; lets-plot margins leave it alone).
pub(super) fn train_bands(
    layers: &[PreparedLayer],
    bands: &[BandSetup],
    band_scales: &mut [PositionScale],
    x_scale: &mut PositionScale,
    y_scale: &mut PositionScale,
) {
    for (index, setup) in bands.iter().enumerate() {
        let members = layers.iter().filter(|layer| layer.band == Some(index));
        let cross = &mut band_scales[index];
        let trains_shared = setup.style == BandStyle::Ggside;
        let mut untouched = if setup.key.shares_x {
            x_scale.clone()
        } else {
            y_scale.clone()
        };
        let shared: &mut PositionScale = match (trains_shared, setup.key.shares_x) {
            (true, true) => x_scale,
            (true, false) => y_scale,
            (false, _) => &mut untouched,
        };
        if setup.key.shares_x {
            train_positions(members, shared, cross);
        } else {
            train_positions(members, cross, shared);
        }
    }
}

/// What the layout is asked for: one request per band.
pub(super) fn band_requests<'a>(
    bands: &'a [SideBand],
    theme: &Theme,
    flipped: bool,
) -> Vec<BandRequest<'a>> {
    bands
        .iter()
        .map(|band| {
            let side = band.setup.screen_side(flipped);
            let (size, spacing) = match band.setup.style {
                BandStyle::Ggside => {
                    let (x_share, y_share) = theme.ggside_panel_scale;
                    let share = band.setup.size.unwrap_or(if band.setup.key.shares_x {
                        x_share
                    } else {
                        y_share
                    });
                    let (horizontal_gap, vertical_gap) = theme.ggside_panel_spacing;
                    let spacing = if side.is_horizontal() {
                        vertical_gap
                    } else {
                        horizontal_gap
                    };
                    (BandSize::OfPanel(share), spacing)
                }
                BandStyle::Marginal => (
                    BandSize::OfTotal(band.setup.size.unwrap_or(DEFAULT_MARGIN_SIZE)),
                    0.0,
                ),
            };
            BandRequest {
                side,
                size,
                spacing,
                cross: &band.cross,
                decorated: band.setup.style == BandStyle::Ggside,
            }
        })
        .collect()
}

/// Check the ggside theme options when the plot has a ggside band.
pub(super) fn check_theme(theme: &Theme, bands: &[BandSetup]) -> Result<()> {
    if !bands.iter().any(|band| band.style == BandStyle::Ggside) {
        return Ok(());
    }
    let (x_share, y_share) = theme.ggside_panel_scale;
    for (key, share) in [
        ("ggside_panel_scale_x", x_share),
        ("ggside_panel_scale_y", y_share),
    ] {
        if share.is_nan() || share <= 0.0 {
            return Err(Error::spec(
                format!("theme.{key}"),
                format!(
                    "expected the side panel's size as a positive share of the main panel \
                     (ggside's default is 0.1), got {share}"
                ),
            ));
        }
    }
    let (horizontal, vertical) = theme.ggside_panel_spacing;
    for (key, spacing) in [
        ("ggside_panel_spacing_x", horizontal),
        ("ggside_panel_spacing_y", vertical),
    ] {
        if spacing < 0.0 {
            return Err(Error::spec(
                format!("theme.{key}"),
                format!("expected a gap of 0 px or more, got {spacing}"),
            ));
        }
    }
    Ok(())
}

/// The label of a side `tile`/`raster` track whose layer maps nothing across the
/// band: the name of the column mapped to fill (else color), else the geom.
pub(super) fn track_label(mapping: &crate::spec::Mapping, geom: &str) -> String {
    [Aesthetic::Fill, Aesthetic::Color]
        .iter()
        .find_map(|aesthetic| mapping.get(aesthetic).and_then(|value| value.as_column()))
        .map_or_else(|| geom.to_owned(), str::to_owned)
}

/// Geoms whose side layers default to one track per layer (see [`track_label`]).
pub(super) const TRACK_GEOMS: &[&str] = &["tile", "raster"];

/// Whether a layer spec is a side layer (ggside or marginal).
fn is_side_layer(layer: &crate::spec::LayerSpec) -> bool {
    layer.ggside.as_ref().is_some_and(|value| !value.is_null())
        || matches!(layer.marginal, Some(Value::Bool(true)))
}

/// A faceted plot with its side layers removed (facets draw no side panels yet),
/// with the warning; `None` when it has none or is not faceted.
pub(super) fn without_side_layers(
    spec: &PlotSpec,
    data: &PlotData,
) -> Option<(PlotSpec, PlotData, String)> {
    if spec.facet.is_none() || !spec.layers.iter().any(is_side_layer) {
        return None;
    }
    let mut spec = spec.clone();
    let mut data = data.clone();
    let mut dropped = Vec::new();
    let mut kept_layers = Vec::new();
    let mut kept_frames = Vec::new();
    for (index, layer) in spec.layers.drain(..).enumerate() {
        if is_side_layer(&layer) {
            dropped.push(format!("layers[{index}]"));
            continue;
        }
        kept_layers.push(layer);
        kept_frames.push(data.layers.get(index).cloned().flatten());
    }
    spec.layers = kept_layers;
    data.layers = kept_frames;
    let warning = format!(
        "at '{}': faceted plots draw no side panels yet (ggside layers, ggmarginal()); \
         these layers were dropped",
        dropped.join("', '")
    );
    Some((spec, data, warning))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn side(shares_x: bool, far: bool, style: BandStyle, size: Option<f64>) -> Option<LayerSide> {
        Some(LayerSide {
            band: BandKey {
                shares_x,
                far,
                slot: 0,
            },
            style,
            size,
            name: None,
        })
    }

    fn named(shares_x: bool, far: bool, name: &str, size: Option<f64>) -> Option<LayerSide> {
        side(shares_x, far, BandStyle::Ggside, size).map(|side| LayerSide {
            name: Some(name.to_owned()),
            ..side
        })
    }

    #[test]
    fn layers_on_one_side_share_a_band() {
        let sides = [
            None,
            side(true, true, BandStyle::Ggside, None),
            side(false, true, BandStyle::Marginal, Some(0.2)),
            side(true, true, BandStyle::Ggside, None),
            side(false, true, BandStyle::Marginal, None),
        ];
        let (setups, bands) = band_setups(&sides);
        assert_eq!(setups.len(), 2);
        assert_eq!(bands, vec![None, Some(1), Some(0), Some(1), Some(0)]);
        assert_eq!(setups[0].style, BandStyle::Marginal);
        assert_eq!(setups[0].size, Some(0.2));
        assert_eq!(setups[1].size, None);
    }

    #[test]
    fn named_bands_stack_outward_in_plot_order() {
        let sides = [
            None,
            named(false, false, "score", None),
            side(false, true, BandStyle::Ggside, None),
            side(false, false, BandStyle::Ggside, Some(0.05)),
            named(false, false, "score", Some(0.2)),
            named(false, false, "other", Some(0.1)),
            named(false, false, "score", Some(0.3)),
        ];
        let (setups, bands) = band_setups(&sides);
        // Left bands first (far = false), by slot; then the right band.
        let summary: Vec<(bool, usize, Option<&str>, Option<f64>)> = setups
            .iter()
            .map(|setup| {
                (
                    setup.key.far,
                    setup.key.slot,
                    setup.name.as_deref(),
                    setup.size,
                )
            })
            .collect();
        assert_eq!(
            summary,
            vec![
                (false, 0, Some("score"), Some(0.3)),
                (false, 1, None, Some(0.05)),
                (false, 2, Some("other"), Some(0.1)),
                (true, 0, None, None),
            ]
        );
        assert_eq!(
            bands,
            vec![None, Some(0), Some(3), Some(1), Some(0), Some(2), Some(0)]
        );
    }

    #[test]
    fn a_margin_size_does_not_size_a_ggside_band() {
        let sides = [
            side(true, true, BandStyle::Marginal, Some(0.4)),
            side(true, true, BandStyle::Ggside, None),
        ];
        let (setups, _) = band_setups(&sides);
        assert_eq!(setups[0].style, BandStyle::Ggside);
        assert_eq!(setups[0].size, None);
    }

    #[test]
    fn coord_flip_moves_bands_to_the_other_screen_sides() {
        let (setups, _) = band_setups(&[side(true, true, BandStyle::Ggside, None)]);
        assert_eq!(setups[0].screen_side(false), Side::Top);
        assert_eq!(setups[0].screen_side(true), Side::Right);
        let (setups, _) = band_setups(&[side(false, false, BandStyle::Ggside, None)]);
        assert_eq!(setups[0].screen_side(false), Side::Left);
        assert_eq!(setups[0].screen_side(true), Side::Bottom);
    }
}

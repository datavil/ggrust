//! Shared axes in a grid: `gggrid(sharex=, sharey=)` (lets-plot `sharex`/`sharey`).
//!
//! What it does: plots of a grid in one sharing group (`all`, the same `row` or the
//! same `col`) take one range on that screen axis. Continuous axes take the union
//! of the plots' trained ranges (each plot's stats keep their own range, then the
//! axis is widened, expanded and broken as usual). Discrete axes take the union of
//! the plots' categories in order of appearance: those plots are prepared again
//! with it as their limits, so every category sits at the same place. lets-plot
//! does not share discrete axes (lets-plot#1015); ggrust does.
//!
//! A shared axis applies to every panel of a faceted plot, free facet scales
//! included. A plot's `reverse` and `breaks` order only its own axis; the union
//! is in order of appearance over the plots' trained categories, so explicit
//! level orders (`as_discrete(levels=)`) are not merged across plots.
//!
//! What it does not do: nested grids do not take part; plots with their own scale
//! limits on the axis keep them; continuous axes share only with plots of the
//! same transform (the first continuous plot's in the group); labels are not
//! hidden on inner plots (as lets-plot).

use std::collections::{BTreeMap, HashSet};

use crate::build::{PlotData, PreparedPlot, prepare_plot};
use crate::data::DataFrame;
use crate::layout::grid::{Span, figure_spans};
use crate::scale::PositionScale;
use crate::spec::{Aesthetic, GridLayoutSpec, OneOrMany, Options, PlotSpec, ScaleSpec, Value};
use crate::{Error, Result};

/// Which plots share an axis.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum ShareMode {
    All,
    Row,
    Column,
}

impl ShareMode {
    /// `None` for `none` or unset; an error names the accepted values.
    fn parse(value: Option<&str>, key: &str) -> Result<Option<ShareMode>> {
        match value {
            None | Some("none") => Ok(None),
            Some("all") => Ok(Some(ShareMode::All)),
            Some("row") => Ok(Some(ShareMode::Row)),
            Some("col") => Ok(Some(ShareMode::Column)),
            Some(other) => Err(Error::spec(
                format!("layout.{key}"),
                format!(
                    "unsupported value '{other}'; use one of: all, none, row, col \
                     (or True / False)"
                ),
            )),
        }
    }

    /// The sharing group of a figure: by the row or column of its first cell.
    fn group(self, span: Span) -> usize {
        match self {
            ShareMode::All => 0,
            ShareMode::Row => span.top,
            ShareMode::Column => span.left,
        }
    }
}

/// Error unless `sharex`/`sharey` hold accepted values (checked before preparing).
pub(crate) fn validate(layout: &GridLayoutSpec) -> Result<()> {
    ShareMode::parse(layout.sharex.as_deref(), "sharex")?;
    ShareMode::parse(layout.sharey.as_deref(), "sharey")?;
    Ok(())
}

/// Share the axes of the grid's plots. `specs` are the plot specs as prepared
/// (with the grid's theme), `None` for empty cells and nested grids; `prepared`
/// the matching results, replaced when a plot is prepared again.
pub(crate) fn share_axes(
    layout: &GridLayoutSpec,
    specs: &[Option<PlotSpec>],
    prepared: &mut [Option<Result<PreparedPlot>>],
    frames: &[DataFrame],
) -> Result<()> {
    let spans = figure_spans(
        (layout.ncol as usize, layout.nrow as usize),
        prepared.len(),
        layout.areas.as_deref(),
    )
    .map_err(|message| Error::spec("layout.areas", message))?;
    for (horizontal, value, key) in [
        (true, layout.sharex.as_deref(), "sharex"),
        (false, layout.sharey.as_deref(), "sharey"),
    ] {
        let Some(mode) = ShareMode::parse(value, key)? else {
            continue;
        };
        let mut groups: BTreeMap<usize, Vec<usize>> = BTreeMap::new();
        for (index, result) in prepared.iter().enumerate() {
            if matches!(result, Some(Ok(_))) {
                groups
                    .entry(mode.group(spans[index]))
                    .or_default()
                    .push(index);
            }
        }
        for members in groups.values() {
            share_discrete(members, horizontal, specs, prepared, frames);
            share_continuous(members, horizontal, prepared);
        }
    }
    Ok(())
}

fn plot(prepared: &[Option<Result<PreparedPlot>>], index: usize) -> Option<&PreparedPlot> {
    match &prepared[index] {
        Some(Ok(plot)) => Some(plot),
        _ => None,
    }
}

/// Union of the members' categories; members without limits whose categories
/// differ are prepared again with the union as limits.
fn share_discrete(
    members: &[usize],
    horizontal: bool,
    specs: &[Option<PlotSpec>],
    prepared: &mut [Option<Result<PreparedPlot>>],
    frames: &[DataFrame],
) {
    let discrete = |scales: &[&PositionScale]| {
        !scales.is_empty() && scales.iter().all(|scale| scale.is_discrete())
    };
    let sharing: Vec<usize> = members
        .iter()
        .copied()
        .filter(|&index| {
            plot(prepared, index).is_some_and(|plot| discrete(&plot.screen_axis_scales(horizontal)))
        })
        .collect();
    if sharing.len() < 2 {
        return;
    }
    let mut union: Vec<String> = Vec::new();
    let mut seen = HashSet::new();
    for &index in &sharing {
        let Some(plot) = plot(prepared, index) else {
            continue;
        };
        for scale in plot.screen_axis_scales(horizontal) {
            // Trained levels: `reverse` and breaks order only their own plot.
            if let PositionScale::Discrete(scale) = scale {
                for level in scale.trained_levels() {
                    if seen.insert(level.clone()) {
                        union.push(level.clone());
                    }
                }
            }
        }
    }
    for index in sharing {
        let Some(plot) = plot(prepared, index) else {
            continue;
        };
        let scales = plot.screen_axis_scales(horizontal);
        let fixed = scales
            .iter()
            .any(|scale| matches!(scale, PositionScale::Discrete(scale) if scale.has_limits()));
        let complete = scales.iter().all(|scale| {
            matches!(scale, PositionScale::Discrete(scale) if scale.trained_levels() == union)
        });
        if fixed || complete {
            continue;
        }
        let aesthetic = plot.screen_aesthetic(horizontal);
        let Some(spec) = &specs[index] else {
            continue;
        };
        let spec = with_limits(spec, aesthetic, &union);
        prepared[index] =
            Some(PlotData::from_frames(&spec, frames).and_then(|data| prepare_plot(&spec, &data)));
    }
}

/// `spec` with `limits` on the scale of `aesthetic` (added when it has none).
fn with_limits(spec: &PlotSpec, aesthetic: Aesthetic, limits: &[String]) -> PlotSpec {
    let mut spec = spec.clone();
    let values: Vec<Value> = limits
        .iter()
        .map(|level| Value::from(level.as_str()))
        .collect();
    match spec
        .scales
        .iter_mut()
        .find(|scale| scale.aesthetic.as_slice().contains(&aesthetic) && scale.ggside.is_none())
    {
        Some(scale) => scale.limits = Some(values),
        None => spec.scales.push(ScaleSpec {
            aesthetic: OneOrMany::One(aesthetic),
            name: None,
            breaks: None,
            labels: None,
            limits: Some(values),
            expand: None,
            trans: None,
            format: None,
            na_value: None,
            guide: None,
            discrete: None,
            reverse: None,
            position: None,
            scale_mapper_kind: None,
            scale_group: BTreeMap::new(),
            ggside: None,
            side_pos: None,
            side_band: None,
            parameters: Options::new(),
        }),
    }
    spec
}

/// Widen the members' continuous scales to the union of their trained ranges.
fn share_continuous(
    members: &[usize],
    horizontal: bool,
    prepared: &mut [Option<Result<PreparedPlot>>],
) {
    let mut transform = None;
    let mut sharing = Vec::new();
    let mut union: Option<(f64, f64)> = None;
    for &index in members {
        let Some(plot) = plot(prepared, index) else {
            continue;
        };
        let scales = plot.screen_axis_scales(horizontal);
        let continuous: Vec<_> = scales
            .iter()
            .filter_map(|scale| match scale {
                PositionScale::Continuous(scale) => Some(scale),
                PositionScale::Discrete(_) => None,
            })
            .collect();
        if continuous.is_empty() || continuous.len() != scales.len() {
            continue;
        }
        let first = continuous[0].transform;
        if continuous.iter().any(|scale| scale.transform != first)
            || *transform.get_or_insert(first) != first
        {
            continue;
        }
        sharing.push(index);
        for scale in continuous {
            if let Some((low, high)) = scale.trained_domain() {
                union = Some(union.map_or((low, high), |(union_low, union_high)| {
                    (union_low.min(low), union_high.max(high))
                }));
            }
        }
    }
    let Some((low, high)) = union.filter(|_| sharing.len() > 1) else {
        return;
    };
    for index in sharing {
        let Some(Ok(plot)) = &mut prepared[index] else {
            continue;
        };
        let mut failed = None;
        for scale in plot.screen_axis_scales_mut(horizontal) {
            // Both limits given: the user fixed the range.
            if !matches!(scale.limits(), [Some(_), Some(_)]) {
                scale.train_span(low, high);
                // `labels=` without `breaks=` name the breaks of the new range,
                // then a secondary axis samples its transform over it.
                if let Err(error) = scale
                    .attach_labels()
                    .and_then(|()| scale.resolve_secondary())
                {
                    failed.get_or_insert(error);
                }
            }
        }
        if let Some(error) = failed {
            prepared[index] = Some(Err(error));
        }
    }
}

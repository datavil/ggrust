//! A position scale: continuous (with a transform) or discrete. Layout, coord and
//! geoms use this type, so they work the same on both kinds of axis.

use super::number_format::ExponentFormat;
use super::{ContinuousPositionScale, DiscretePositionScale, ScaleBreaks};
use crate::data::{Column, ContinuousColumn};

/// Sides of the panel a position scale draws its axis on (`position=`).
///
/// `primary` is below the panel for x and left of it for y (the default),
/// `secondary` above it for x and right of it for y. Under `coord_flip` the sides
/// follow the scale to its screen axis (x `"top"` goes right, y `"right"` goes
/// top), as lets-plot and ggplot2 do.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct AxisSides {
    /// Bottom (x) or left (y).
    pub primary: bool,
    /// Top (x) or right (y).
    pub secondary: bool,
}

impl Default for AxisSides {
    fn default() -> Self {
        AxisSides {
            primary: true,
            secondary: false,
        }
    }
}

impl AxisSides {
    /// Parse a `position=` value of an x (`horizontal`) or y scale; `path`
    /// locates it in error messages.
    pub fn parse(name: &str, horizontal: bool, path: &str) -> crate::Result<Self> {
        let (primary, secondary) = if horizontal {
            ("bottom", "top")
        } else {
            ("left", "right")
        };
        let sides = |primary, secondary| AxisSides { primary, secondary };
        match name {
            _ if name == primary => Ok(sides(true, false)),
            _ if name == secondary => Ok(sides(false, true)),
            "both" => Ok(sides(true, true)),
            other => {
                let axis = if horizontal { "x" } else { "y" };
                let accepted = [primary, secondary, "both"];
                Err(crate::Error::spec(
                    path,
                    format!(
                        "unknown position '{other}' for a {axis} axis; use one of '{primary}', \
                         '{secondary}', 'both'{}",
                        crate::suggest::did_you_mean(other, accepted)
                    ),
                ))
            }
        }
    }
}

/// The scale of one position axis.
#[derive(Debug, Clone, PartialEq)]
// A handful per plot, and matched everywhere: boxing would only add noise.
#[allow(clippy::large_enum_variant)]
pub enum PositionScale {
    /// Numbers, optionally transformed (`scale_x_continuous`, `scale_x_log10`, ...).
    Continuous(ContinuousPositionScale),
    /// Categories at 0, 1, 2, ... (`scale_x_discrete` or a discrete column).
    Discrete(DiscretePositionScale),
}

impl PositionScale {
    /// No padding around the domain (`coord_cartesian(expand=False)`).
    pub fn remove_expansion(&mut self) {
        match self {
            PositionScale::Continuous(scale) => {
                scale.multiplicative_expand = 0.0;
                scale.additive_expand = 0.0;
            }
            PositionScale::Discrete(scale) => {
                scale.multiplicative_expand = 0.0;
                scale.additive_expand = 0.0;
            }
        }
    }

    /// The sides of the panel this axis is drawn on (`position=`); both with a
    /// secondary axis.
    pub fn sides(&self) -> AxisSides {
        if self.secondary_axis().is_some() {
            return AxisSides {
                primary: true,
                secondary: true,
            };
        }
        self.position_sides()
    }

    /// The sides `position=` asked for, regardless of a secondary axis.
    fn position_sides(&self) -> AxisSides {
        match self {
            PositionScale::Continuous(scale) => scale.sides,
            PositionScale::Discrete(scale) => scale.sides,
        }
    }

    /// The drawn secondary axis (`sec_axis=`, not `guide="none"`), if any.
    pub fn secondary_axis(&self) -> Option<&super::secondary::SecondaryAxis> {
        match self {
            PositionScale::Continuous(scale) => {
                scale.secondary.as_ref().filter(|axis| !axis.hidden)
            }
            PositionScale::Discrete(_) => None,
        }
    }

    /// Whether the secondary axis is drawn on the top / right side (`None`
    /// without one): it takes the side opposite `position=` (ggplot2).
    pub fn secondary_on_far_side(&self) -> Option<bool> {
        self.secondary_axis().map(|_| {
            let sides = self.position_sides();
            !(sides.secondary && !sides.primary)
        })
    }

    /// `labels=[...]` without `breaks=` on a continuous axis: fix the generated
    /// breaks of the trained scale and label them (see
    /// [`ContinuousPositionScale::attach_labels`]); a no-op otherwise.
    pub fn attach_labels(&mut self) -> crate::Result<()> {
        match self {
            PositionScale::Continuous(scale) => scale.attach_labels(),
            PositionScale::Discrete(_) => Ok(()),
        }
    }

    /// Sample the secondary axis transform over the final range (after training,
    /// coord limits and grid sharing); a no-op without a secondary axis.
    pub fn resolve_secondary(&mut self) -> crate::Result<()> {
        if let PositionScale::Continuous(scale) = self
            && let Some(axis) = &scale.secondary
        {
            let resolved = axis.resolve(scale)?;
            scale.secondary = Some(resolved);
        }
        Ok(())
    }

    /// The dendrogram drawn beside this axis (`scale_*_dendro()`), if any.
    pub fn dendrogram(&self) -> Option<&crate::layout::dendrogram::AxisDendrogram> {
        match self {
            PositionScale::Discrete(scale) => scale.dendrogram.as_deref(),
            PositionScale::Continuous(_) => None,
        }
    }

    /// Drop the dendrogram (facets do not draw one).
    pub fn drop_dendrogram(&mut self) -> bool {
        match self {
            PositionScale::Discrete(scale) => scale.dendrogram.take().is_some(),
            PositionScale::Continuous(_) => false,
        }
    }

    /// Whether the axis shows categories.
    pub fn is_discrete(&self) -> bool {
        matches!(self, PositionScale::Discrete(_))
    }

    /// Positions of a mapped column, in the units [`Self::map`] takes: transformed
    /// numbers, or category indices. Values the scale drops (outside limits or the
    /// transform domain, unknown categories) become missing.
    ///
    /// Errors when a discrete column meets a continuous scale.
    pub fn positions(&self, column: Column) -> Result<Column, String> {
        Ok(Column::Continuous(match (self, column) {
            (PositionScale::Continuous(scale), Column::Continuous(column)) => {
                match scale.na_value {
                    // ggplot2 `oob` then `na.value`: dropped values are drawn there.
                    Some(missing) => ContinuousColumn::from_options(
                        (0..column.len())
                            .map(|row| {
                                let value = column.get(row).filter(|value| !value.is_nan());
                                Some(
                                    value
                                        .and_then(|value| scale.transform_value(value))
                                        .unwrap_or(missing),
                                )
                            })
                            .collect(),
                    ),
                    None if scale.transform == super::Transform::Identity
                        && scale.limits == [None, None] =>
                    {
                        column
                    }
                    None => column.map_values(|value| scale.transform_value(value)),
                }
            }
            (PositionScale::Discrete(scale), Column::Discrete(column)) => scale.positions(&column),
            (PositionScale::Discrete(_), Column::Continuous(column)) => column,
            (PositionScale::Continuous(scale), Column::Discrete(_)) => {
                return Err(format!(
                    "the column is discrete but the scale is continuous (transform '{}'); \
                     use a numeric column or drop the transform",
                    scale.transform.as_str()
                ));
            }
        }))
    }

    /// Positions already in axis units (e.g. a stat's bin centres) with values
    /// outside the scale limits made missing, as [`Self::positions`] does.
    pub fn within_limits(&self, column: ContinuousColumn) -> ContinuousColumn {
        match self {
            PositionScale::Continuous(scale) if scale.limits != [None, None] => {
                column.map_values(|value| scale.limit(value))
            }
            _ => column,
        }
    }

    /// Ends of the scale limits in axis units, where given and finite.
    pub fn limits(&self) -> [Option<f64>; 2] {
        match self {
            PositionScale::Continuous(scale) => scale
                .limits
                .map(|limit| limit.filter(|value| value.is_finite())),
            PositionScale::Discrete(_) => [None, None],
        }
    }

    /// The scale as its axis draws it (see
    /// [`ContinuousPositionScale::axis_identity`],
    /// [`DiscretePositionScale::axis_identity`]).
    pub fn axis_identity(&self) -> PositionScale {
        match self {
            PositionScale::Continuous(scale) => PositionScale::Continuous(scale.axis_identity()),
            PositionScale::Discrete(scale) => PositionScale::Discrete(scale.axis_identity()),
        }
    }

    /// Widen the domain to include `[low, high]`, even past continuous limits
    /// (a geom's width span).
    pub fn train_span(&mut self, low: f64, high: f64) {
        match self {
            PositionScale::Continuous(scale) => scale.train_beyond_limits(low, high),
            PositionScale::Discrete(scale) => {
                scale.train_numbers(&ContinuousColumn::from_values(vec![low, high]))
            }
        }
    }

    /// Widen the domain to include the positions of `column`.
    pub fn train(&mut self, column: &ContinuousColumn) {
        match self {
            PositionScale::Continuous(scale) => scale.train(column),
            PositionScale::Discrete(scale) => scale.train_numbers(column),
        }
    }

    /// Domain after limits and expansion: the panel spans exactly this range.
    pub fn expanded_domain(&self) -> (f64, f64) {
        match self {
            PositionScale::Continuous(scale) => scale.expanded_domain(),
            PositionScale::Discrete(scale) => scale.expanded_domain(),
        }
    }

    /// The domain with no expansion (coord limits and a geom's span kept).
    pub fn unexpanded_domain(&self) -> (f64, f64) {
        let mut scale = self.clone();
        match &mut scale {
            PositionScale::Continuous(scale) => {
                scale.multiplicative_expand = 0.0;
                scale.additive_expand = 0.0;
            }
            PositionScale::Discrete(scale) => {
                scale.multiplicative_expand = 0.0;
                scale.additive_expand = 0.0;
            }
        }
        scale.expanded_domain()
    }

    /// Make the panel span exactly `[low, high]`, in transformed units.
    pub fn set_view_domain(&mut self, low: f64, high: f64) {
        match self {
            PositionScale::Continuous(scale) => scale.coord_limits = [Some(low), Some(high)],
            PositionScale::Discrete(scale) => scale.coord_limits = [Some(low), Some(high)],
        }
    }

    /// Domain after limits, before expansion (`scale_info()`).
    pub fn limited_domain(&self) -> (f64, f64) {
        match self {
            PositionScale::Continuous(scale) => scale.limited_domain(),
            PositionScale::Discrete(scale) => {
                (0.0, scale.effective_levels().len().saturating_sub(1) as f64)
            }
        }
    }

    /// The categories on the axis, in axis order; empty for a continuous scale.
    pub fn levels(&self) -> Vec<String> {
        match self {
            PositionScale::Continuous(_) => Vec::new(),
            PositionScale::Discrete(scale) => scale
                .effective_levels()
                .into_iter()
                .map(str::to_owned)
                .collect(),
        }
    }

    /// Map a position to 0..1 across the panel. Non-finite input gives NaN.
    pub fn map(&self, value: f64) -> f64 {
        match self {
            PositionScale::Continuous(scale) => scale.map(value),
            PositionScale::Discrete(scale) => scale.map(value),
        }
    }

    /// The domain to use when nothing trains the axis. Continuous scales only.
    pub fn set_null_domain(&mut self, domain: (f64, f64)) {
        if let PositionScale::Continuous(scale) = self {
            scale.null_domain = domain;
        }
    }

    /// Include 0 without padding past it (bars). Continuous scales only.
    pub fn set_include_zero(&mut self) {
        if let PositionScale::Continuous(scale) = self {
            scale.include_zero = true;
        }
    }

    /// The theme's `exponent_format` for the axis labels.
    pub fn set_exponent_format(&mut self, exponent: ExponentFormat) {
        match self {
            PositionScale::Continuous(scale) => scale.set_exponent_format(exponent),
            PositionScale::Discrete(scale) => {
                if let Some(format) = &scale.format {
                    scale.format = Some(format.with_exponent(exponent));
                }
            }
        }
    }

    /// `coord_*(xlim=...)`: zoom to these data limits (either side open).
    pub fn set_coord_limits(&mut self, limits: [Option<f64>; 2]) {
        let finite = limits.map(|limit| limit.filter(|value| value.is_finite()));
        match self {
            PositionScale::Continuous(scale) => scale.set_coord_limits(finite),
            PositionScale::Discrete(scale) => scale.coord_limits = finite,
        }
    }

    /// Whether one coord limit is given and lies beyond the other (open) end of the
    /// scale's own domain, which lets-plot rejects.
    pub fn coord_limit_beyond_data(&self) -> bool {
        let (limits, (low, high)) = match self {
            PositionScale::Continuous(scale) => (scale.coord_limits, scale.own_domain()),
            PositionScale::Discrete(scale) => (scale.coord_limits, scale.own_domain()),
        };
        match limits {
            [Some(limit), None] => limit > high,
            [None, Some(limit)] => limit < low,
            _ => false,
        }
    }

    /// `guide_axis_mark()` options, when the axis spreads its labels with links.
    pub fn mark(&self) -> Option<crate::layout::axis_mark::AxisMark> {
        match self {
            PositionScale::Continuous(scale) => scale.mark,
            PositionScale::Discrete(scale) => scale.mark,
        }
    }

    /// `guide_axis_nested()` options, when the axis draws group brackets.
    pub fn nested(&self) -> Option<&std::sync::Arc<crate::layout::axis_nested::AxisNested>> {
        match self {
            PositionScale::Continuous(scale) => scale.nested.as_ref(),
            PositionScale::Discrete(scale) => scale.nested.as_ref(),
        }
    }

    /// Drop a nested guide drawn across the panel (facets do not draw one); true
    /// when one was dropped.
    pub fn drop_opposite_nested(&mut self) -> bool {
        let nested = match self {
            PositionScale::Continuous(scale) => &mut scale.nested,
            PositionScale::Discrete(scale) => &mut scale.nested,
        };
        let opposite = nested
            .as_ref()
            .is_some_and(|nested| nested.side == crate::layout::axis_nested::NestedSide::Opposite);
        if opposite {
            *nested = None;
        }
        opposite
    }

    /// Whether the breaks are fixed (every category, or `breaks=[...]`): the axis
    /// shows all of them instead of choosing a count.
    pub fn has_fixed_breaks(&self) -> bool {
        match self {
            // A `break_width` fixes the breaks too (lets-plot
            // `DateTimeFixedBreaksGen`, numbers likewise), so their labels are laid
            // out as fixed.
            PositionScale::Continuous(scale) => {
                scale.fixed_breaks.is_some()
                    || scale.break_width.is_some()
                    || scale
                        .temporal
                        .as_ref()
                        .is_some_and(|temporal| temporal.break_width.is_some())
            }
            PositionScale::Discrete(_) => true,
        }
    }

    /// Whether the axis shows date-times or durations.
    pub fn is_temporal(&self) -> bool {
        matches!(self, PositionScale::Continuous(scale) if scale.temporal.is_some())
    }

    /// Breaks: about `target_count` for continuous scales, every category for
    /// discrete ones.
    pub fn breaks(&self, target_count: usize) -> ScaleBreaks {
        match self {
            PositionScale::Continuous(scale) => scale.breaks(target_count),
            PositionScale::Discrete(scale) => scale.breaks(),
        }
    }
}

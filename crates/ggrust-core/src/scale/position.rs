//! A position scale: continuous (with a transform) or discrete. Layout, coord and
//! geoms use this type, so they work the same on both kinds of axis.

use super::{ContinuousPositionScale, DiscretePositionScale, ScaleBreaks};
use crate::data::{Column, ContinuousColumn};

/// The scale of one position axis.
#[derive(Debug, Clone, PartialEq)]
pub enum PositionScale {
    /// Numbers, optionally transformed (`scale_x_continuous`, `scale_x_log10`, ...).
    Continuous(ContinuousPositionScale),
    /// Categories at 0, 1, 2, ... (`scale_x_discrete` or a discrete column).
    Discrete(DiscretePositionScale),
}

impl PositionScale {
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
                if scale.transform == super::Transform::Identity && scale.limits == [None, None] {
                    column
                } else {
                    column.map_values(|value| scale.transform_value(value))
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
                column.map_values(|value| scale.within_limits(value).then_some(value))
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

    /// Map a position to 0..1 across the panel. Non-finite input gives NaN.
    pub fn map(&self, value: f64) -> f64 {
        match self {
            PositionScale::Continuous(scale) => scale.map(value),
            PositionScale::Discrete(scale) => scale.map(value),
        }
    }

    /// Include 0 without padding past it (bars). Continuous scales only.
    pub fn set_include_zero(&mut self) {
        if let PositionScale::Continuous(scale) = self {
            scale.include_zero = true;
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

    /// Whether the breaks are fixed (every category, or `breaks=[...]`): the axis
    /// shows all of them instead of choosing a count.
    pub fn has_fixed_breaks(&self) -> bool {
        match self {
            PositionScale::Continuous(scale) => scale.fixed_breaks.is_some(),
            PositionScale::Discrete(_) => true,
        }
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

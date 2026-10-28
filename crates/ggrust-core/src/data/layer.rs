//! The columns of one layer, keyed by variable, as stats and positions see them.

use std::collections::BTreeMap;
use std::fmt;

use super::{Column, ContinuousColumn};
use crate::spec::Aesthetic;
use crate::{Error, Result};

/// A variable computed by a stat, mapped with `..name..` (e.g. `aes(y="..count..")`).
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Computed {
    /// `..count..`: rows per bin or category.
    Count,
    /// `..density..`: density estimate.
    Density,
    /// `..ncount..`: count scaled to a maximum of 1.
    NormalizedCount,
    /// `..ndensity..`: density scaled to a maximum of 1.
    NormalizedDensity,
    /// `..prop..`: proportion within the group.
    Proportion,
    /// `..proppct..`: proportion within the group, in percent.
    ProportionPercent,
    /// `..sum..`: sum of weights.
    Sum,
    /// `..sumprop..`: share of the layer total at the location.
    SumProportion,
    /// `..sumpct..`: share of the layer total at the location, in percent.
    SumPercent,
    /// `..scaled..`: density scaled to a maximum of 1 (violin).
    Scaled,
    /// `..violinwidth..`: violin width.
    ViolinWidth,
    /// `..quantile..`: quantile band of a density point (violins).
    Quantile,
    /// `..height..`: ridge height, density scaled to the layer's largest.
    Height,
}

impl Computed {
    const ALL: [(Computed, &'static str); 13] = [
        (Computed::Count, "count"),
        (Computed::Density, "density"),
        (Computed::NormalizedCount, "ncount"),
        (Computed::NormalizedDensity, "ndensity"),
        (Computed::Proportion, "prop"),
        (Computed::ProportionPercent, "proppct"),
        (Computed::Sum, "sum"),
        (Computed::SumProportion, "sumprop"),
        (Computed::SumPercent, "sumpct"),
        (Computed::Scaled, "scaled"),
        (Computed::ViolinWidth, "violinwidth"),
        (Computed::Quantile, "quantile"),
        (Computed::Height, "height"),
    ];

    /// lets-plot name without dots, e.g. `count`.
    pub fn as_str(self) -> &'static str {
        Computed::ALL
            .iter()
            .find(|(variable, _)| *variable == self)
            .map_or("", |(_, name)| name)
    }

    /// Parse a mapping value like `..count..`; `None` if it is not a known stat variable.
    pub fn from_mapping(name: &str) -> Option<Computed> {
        let inner = name.strip_prefix("..")?.strip_suffix("..")?;
        Computed::ALL
            .iter()
            .find(|(_, candidate)| *candidate == inner)
            .map(|(variable, _)| *variable)
    }
}

/// A layer column key: an aesthetic or a stat-computed variable.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub enum Variable {
    /// A mapped aesthetic (`x`, `color`, ...).
    Aesthetic(Aesthetic),
    /// A stat variable (`..count..`).
    Computed(Computed),
}

impl From<Aesthetic> for Variable {
    fn from(aesthetic: Aesthetic) -> Self {
        Variable::Aesthetic(aesthetic)
    }
}

impl From<Computed> for Variable {
    fn from(computed: Computed) -> Self {
        Variable::Computed(computed)
    }
}

impl fmt::Display for Variable {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Variable::Aesthetic(aesthetic) => formatter.write_str(aesthetic.as_str()),
            Variable::Computed(computed) => write!(formatter, "..{}..", computed.as_str()),
        }
    }
}

/// Which input rows a layer row came from (for tooltips and row-wise options).
#[derive(Debug, Clone, PartialEq, Default)]
pub enum SourceRows {
    /// Row `i` is input row `i`.
    #[default]
    All,
    /// Row `i` is input row `rows[i]` (after filtering or reordering).
    Selected(Vec<u32>),
    /// Rows summarise several input rows (a stat aggregated them).
    Aggregated,
}

/// Columns of one layer keyed by variable, with per-row group and panel indices.
/// Every column and index has `row_count` rows.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct LayerData {
    row_count: usize,
    columns: BTreeMap<Variable, Column>,
    /// `None`: one group.
    group_index: Option<Vec<u32>>,
    /// `None`: one panel.
    panel_index: Option<Vec<u32>>,
    source_rows: SourceRows,
    /// Whether the x and y position scales are discrete (see [`Self::resolution`]).
    discrete_axes: (bool, bool),
}

impl LayerData {
    /// An empty layer with a fixed number of rows, one group and one panel.
    pub fn new(row_count: usize) -> Self {
        LayerData {
            row_count,
            ..LayerData::default()
        }
    }

    /// Number of rows.
    pub fn row_count(&self) -> usize {
        self.row_count
    }

    /// Mark the x and y position scales as discrete or not.
    pub fn set_discrete_axes(&mut self, x: bool, y: bool) {
        self.discrete_axes = (x, y);
    }

    /// Resolution of a position aesthetic (bar widths, dodging, tile sizes): 1 on a
    /// discrete axis, as lets-plot (`PlotUtil.createLayerAesthetics`), else the
    /// smallest gap between the column's values (1 without a column).
    pub fn resolution(&self, aesthetic: Aesthetic) -> f64 {
        let discrete = (aesthetic.is_x_position() && self.discrete_axes.0)
            || (aesthetic.is_y_position() && self.discrete_axes.1);
        match self.continuous(aesthetic) {
            Some(column) if !discrete => super::resolution(column),
            _ => 1.0,
        }
    }

    /// Add or replace a column. Its length must equal [`Self::row_count`].
    pub fn insert(&mut self, variable: impl Into<Variable>, column: Column) -> Result<()> {
        let variable = variable.into();
        self.check_length(&variable.to_string(), column.len())?;
        self.columns.insert(variable, column);
        Ok(())
    }

    /// The column of a variable.
    pub fn get(&self, variable: impl Into<Variable>) -> Option<&Column> {
        self.columns.get(&variable.into())
    }

    /// The column of a variable, if it is continuous.
    pub fn continuous(&self, variable: impl Into<Variable>) -> Option<&ContinuousColumn> {
        match self.columns.get(&variable.into()) {
            Some(Column::Continuous(column)) => Some(column),
            _ => None,
        }
    }

    /// Mapped aesthetics, in order (stat variables excluded).
    pub fn aesthetics(&self) -> impl Iterator<Item = Aesthetic> + '_ {
        self.columns.keys().filter_map(|variable| match variable {
            Variable::Aesthetic(aesthetic) => Some(*aesthetic),
            Variable::Computed(_) => None,
        })
    }

    /// Whether a variable has a column.
    pub fn contains(&self, variable: impl Into<Variable>) -> bool {
        self.columns.contains_key(&variable.into())
    }

    /// Group of every row; `None` when all rows form one group.
    pub fn group_index(&self) -> Option<&[u32]> {
        self.group_index.as_deref()
    }

    /// Group of a row (0 when there is one group).
    pub fn group_of(&self, row: usize) -> u32 {
        self.group_index.as_ref().map_or(0, |groups| groups[row])
    }

    /// Number of groups (1 for an ungrouped layer, even an empty one).
    pub fn group_count(&self) -> usize {
        self.group_index.as_ref().map_or(1, |groups| {
            groups.iter().max().map_or(1, |max| *max as usize + 1)
        })
    }

    /// Set the group of every row (`None`: one group).
    pub fn set_group_index(&mut self, groups: Option<Vec<u32>>) -> Result<()> {
        if let Some(groups) = &groups {
            self.check_length("group index", groups.len())?;
        }
        self.group_index = groups;
        Ok(())
    }

    /// Panel of every row; `None` when there is one panel.
    pub fn panel_index(&self) -> Option<&[u32]> {
        self.panel_index.as_deref()
    }

    /// Set the panel of every row (`None`: one panel).
    pub fn set_panel_index(&mut self, panels: Option<Vec<u32>>) -> Result<()> {
        if let Some(panels) = &panels {
            self.check_length("panel index", panels.len())?;
        }
        self.panel_index = panels;
        Ok(())
    }

    /// Which input rows the layer rows came from.
    pub fn source_rows(&self) -> &SourceRows {
        &self.source_rows
    }

    /// The input row of a layer row; `None` for aggregated rows.
    pub fn source_row(&self, row: usize) -> Option<usize> {
        match &self.source_rows {
            SourceRows::All => Some(row),
            SourceRows::Selected(rows) => rows.get(row).map(|source| *source as usize),
            SourceRows::Aggregated => None,
        }
    }

    /// Set where the rows came from.
    pub fn set_source_rows(&mut self, source_rows: SourceRows) -> Result<()> {
        if let SourceRows::Selected(rows) = &source_rows {
            self.check_length("source rows", rows.len())?;
        }
        self.source_rows = source_rows;
        Ok(())
    }

    fn check_length(&self, what: &str, length: usize) -> Result<()> {
        if length == self.row_count {
            return Ok(());
        }
        Err(Error::InvalidData(format!(
            "{what} has {length} rows, expected {}",
            self.row_count
        )))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn computed_names_round_trip() {
        for (variable, name) in Computed::ALL {
            assert_eq!(
                Computed::from_mapping(&format!("..{name}..")),
                Some(variable)
            );
            assert_eq!(Variable::from(variable).to_string(), format!("..{name}.."));
        }
        assert_eq!(Computed::from_mapping("count"), None);
        assert_eq!(Computed::from_mapping("..nope.."), None);
    }

    #[test]
    fn indices_must_match_the_row_count() {
        let mut data = LayerData::new(3);
        assert_eq!(data.group_count(), 1);
        assert_eq!(data.group_of(2), 0);
        assert!(data.set_group_index(Some(vec![0, 1])).is_err());
        data.set_group_index(Some(vec![0, 2, 1])).unwrap();
        assert_eq!(data.group_count(), 3);
        assert!(data.set_panel_index(Some(vec![0])).is_err());
        assert!(data.set_source_rows(SourceRows::Selected(vec![1])).is_err());
        data.set_source_rows(SourceRows::Selected(vec![4, 5, 6]))
            .unwrap();
        assert_eq!(data.source_row(1), Some(5));
        let error = data
            .insert(
                Computed::Count,
                Column::Continuous(ContinuousColumn::from_values(vec![1.0])),
            )
            .unwrap_err();
        assert!(
            error
                .to_string()
                .contains("..count.. has 1 rows, expected 3"),
            "{error}"
        );
    }

    #[test]
    fn aesthetics_skip_computed_variables() {
        let mut data = LayerData::new(1);
        let column = || Column::Continuous(ContinuousColumn::from_values(vec![1.0]));
        data.insert(Aesthetic::X, column()).unwrap();
        data.insert(Computed::Count, column()).unwrap();
        assert_eq!(data.aesthetics().collect::<Vec<_>>(), [Aesthetic::X]);
        assert!(data.continuous(Computed::Count).is_some());
    }
}

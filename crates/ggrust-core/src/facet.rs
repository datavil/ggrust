//! Facets (`facet_wrap`, `facet_grid`): which panels a plot has, which data rows
//! each panel shows, and where panels sit in the grid (lets-plot
//! `FacetConfig.createWrap`/`createGrid`, `FacetWrap`, `FacetGrid`, `PlotFacets`).
//!
//! What it does: levels of every faceting column joined over the layers' data
//! (first appearance, then `order`), strip labels (`format`, else lets-plot's
//! `{,~g}` for numbers; then the `labeller`, see [`labeller`]; split at newlines and
//! wrapped at `labwidth` characters)
//! and which panels draw axes. `facet_wrap`: one panel per
//! level combination that has data (`drop`), the grid shape from `ncol`, `nrow`,
//! `dir`. `facet_grid`: one panel per (`x` level, `y` level), empty ones too, column
//! strips on the top row and row strips on the right column; free scales are shared
//! per grid column (x) and row (y). A layer without a faceting column shows all its
//! rows in every panel; rows with a missing value in a faceting column are in no
//! panel.
//!
//! What it does not do: datetime labels. Building and drawing the panels is
//! `build` and `layout::facet`.

pub mod labeller;

use std::collections::HashMap;

use crate::data::Column;
use crate::layout::{LINE_BREAK, wrap_words};
use crate::scale::format::Label;
use crate::scale::label_pattern::{LabelPattern, PatternValue};
use crate::scale::secondary::AxisFunctions;
use crate::spec::FacetSpec;
use crate::{Error, Result};
use labeller::Labeller;

/// One faceting column's value.
#[derive(Debug, Clone, PartialEq)]
pub enum FacetValue {
    /// A category.
    Text(String),
    /// A finite number.
    Number(f64),
}

impl FacetValue {
    /// Hashable identity (numbers by bits, as Kotlin's boxed doubles compare).
    fn key(&self) -> LevelKey {
        match self {
            FacetValue::Text(text) => LevelKey::Text(text.clone()),
            FacetValue::Number(number) => LevelKey::Number(number.to_bits()),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq, Hash)]
enum LevelKey {
    Text(String),
    Number(u64),
}

/// One panel.
#[derive(Debug, Clone, PartialEq)]
pub struct Tile {
    /// Grid column, from 0 at the left.
    pub column: usize,
    /// Grid row, from 0 at the top.
    pub row: usize,
    /// Strip labels above the panel, top first: one per faceting column
    /// (`facet_wrap`), the `x` level on the top row (`facet_grid`), else none.
    pub labels: Vec<Label>,
    /// Strip label right of the panel: the `y` level on the right column of a
    /// `facet_grid`.
    pub row_label: Option<Label>,
    /// Panels with the same x scale group share their x scale (one group for
    /// fixed scales, one per panel for a free `facet_wrap`, one per grid column
    /// for a free `facet_grid`).
    pub x_scale_group: usize,
    /// As [`Tile::x_scale_group`], for y (per grid row in a `facet_grid`).
    pub y_scale_group: usize,
    /// Whether the panel draws its x axis (the last panel of its column, or free x).
    pub has_x_axis: bool,
    /// Whether the panel draws its y axis (the first column, or free y).
    pub has_y_axis: bool,
    /// The last panel of its grid column.
    pub is_bottom: bool,
    /// Whether the panel draws a top x axis when the scale asks for one (the first
    /// row, or free x).
    pub has_top_axis: bool,
    /// Whether the panel draws a right y axis when the scale asks for one (the
    /// last panel of its row, or free y).
    pub has_right_axis: bool,
    /// The last panel of its grid row.
    pub is_row_end: bool,
    /// Level index per faceting column.
    levels: Vec<usize>,
}

/// The panels of a faceted plot.
#[derive(Debug, Clone, PartialEq)]
pub struct Facets {
    /// Faceting column names.
    pub variables: Vec<String>,
    /// Grid columns.
    pub columns: usize,
    /// Grid rows.
    pub rows: usize,
    /// Panels in grid order (by row, then column).
    pub tiles: Vec<Tile>,
    /// `scales="free"` or `"free_x"`: x scales per [`Tile::x_scale_group`].
    pub free_x: bool,
    /// `scales="free"` or `"free_y"`.
    pub free_y: bool,
    /// Levels per faceting column, in panel order.
    levels: Vec<Vec<FacetValue>>,
}

/// Faceting columns of one layer's data, one per faceting variable; `None` when
/// the layer's data has no such column (all its rows are in every panel).
pub type LayerFacetColumns = Vec<Option<Column>>;

impl Facets {
    /// `facet_wrap` from its spec and each layer's faceting columns (`layers[i][v]`
    /// is layer `i`'s column of variable `v`) with each layer's row count.
    pub fn wrap(spec: &FacetSpec, layers: &[(&LayerFacetColumns, usize)]) -> Result<Facets> {
        Facets::wrap_with_functions(spec, layers, &AxisFunctions::default())
    }

    /// [`Facets::wrap`], with the functions a `labeller` may refer to.
    pub fn wrap_with_functions(
        spec: &FacetSpec,
        layers: &[(&LayerFacetColumns, usize)],
        functions: &AxisFunctions,
    ) -> Result<Facets> {
        let labeller = Labeller::parse(spec.labeller.as_ref(), functions, "facet.labeller")?;
        let invalid = |key: &str, message: String| Error::spec(format!("facet.{key}"), message);
        let variables: Vec<String> = spec
            .facets
            .as_ref()
            .map(|facets| facets.as_slice().to_vec())
            .unwrap_or_default();
        if variables.is_empty() {
            return Err(invalid(
                "facets",
                "facet_wrap needs one or more faceting columns".to_owned(),
            ));
        }
        for (index, name) in variables.iter().enumerate() {
            if variables[..index].contains(name) {
                return Err(invalid(
                    "facets",
                    format!("faceting columns must be distinct, '{name}' is given twice"),
                ));
            }
        }
        for (key, count) in [("ncol", spec.ncol), ("nrow", spec.nrow)] {
            if count == Some(0) {
                return Err(invalid(key, "expected a positive count, got 0".to_owned()));
            }
        }
        let free = free_scales(spec)?;
        let vertical = match spec.dir.as_deref().map(str::to_lowercase).as_deref() {
            None | Some("h") => false,
            Some("v") => true,
            Some(_) => {
                return Err(invalid(
                    "dir",
                    format!(
                        "unknown value '{}'; expected 'h' (by rows) or 'v' (by columns)",
                        spec.dir.as_deref().unwrap_or_default()
                    ),
                ));
            }
        };
        let orders = per_variable(spec.order.as_ref(), variables.len());
        let formats = per_variable(spec.format.as_ref(), variables.len());
        let widths = per_variable(spec.labwidth.as_ref(), variables.len());

        let mut levels = Vec::with_capacity(variables.len());
        for (index, name) in variables.iter().enumerate() {
            let order = orders[index].flatten().unwrap_or(0);
            levels.push(collect_levels(name, "facets", index, layers, order)?);
        }

        // Every combination (first variable slowest), then only those with data.
        let mut combinations: Vec<Vec<usize>> = vec![Vec::new()];
        for values in &levels {
            combinations = combinations
                .into_iter()
                .flat_map(|prefix| {
                    (0..values.len()).map(move |level| {
                        let mut combination = prefix.clone();
                        combination.push(level);
                        combination
                    })
                })
                .collect();
        }
        if spec.drop.unwrap_or(true) {
            let mut counts = vec![0_usize; combinations.len()];
            let lookup = Lookup::new(&levels);
            for (columns, row_count) in layers {
                for (tile, rows) in lookup
                    .rows_by_tile(&combinations, columns, *row_count)
                    .iter()
                    .enumerate()
                {
                    counts[tile] += rows.len();
                }
            }
            let mut kept = counts.iter().map(|count| *count > 0);
            combinations.retain(|_| kept.next().unwrap_or(false));
        }

        let patterns = formats
            .iter()
            .enumerate()
            .map(|(index, format)| {
                let key = match &spec.format {
                    Some(crate::spec::OneOrMany::Many(_)) => format!("format[{index}]"),
                    _ => "format".to_owned(),
                };
                parse_format(format.clone().flatten().as_deref(), &key)
            })
            .collect::<Result<Vec<_>>>()?;
        let level_labels = levels
            .iter()
            .zip(&patterns)
            .map(|(values, pattern)| {
                labeller.transform_levels(
                    values
                        .iter()
                        .map(|value| format_level(value, pattern.as_ref()))
                        .collect(),
                    &raw_texts(values),
                )
            })
            .collect::<Result<Vec<_>>>()?;
        let names: Vec<&str> = variables.iter().map(String::as_str).collect();
        let count = combinations.len();
        let (columns, rows) = shape(count, spec.ncol, spec.nrow, vertical);
        let mut tiles: Vec<Tile> = combinations
            .into_iter()
            .enumerate()
            .map(|(index, combination)| {
                let (column, row) = if vertical {
                    (index / rows, index % rows)
                } else {
                    (index % columns, index / columns)
                };
                let next = if vertical {
                    column * rows + row + 1
                } else {
                    (row + 1) * columns + column
                };
                let is_bottom = (row + 1) % rows == 0 || next >= count;
                let next_in_row = if vertical {
                    (column + 1) * rows + row
                } else {
                    index + 1
                };
                let is_row_end = column + 1 == columns || next_in_row >= count;
                let values = combination
                    .iter()
                    .enumerate()
                    .map(|(variable, level)| level_labels[variable][*level].clone())
                    .collect();
                let labels = labeller
                    .strips(&names, values)
                    .into_iter()
                    .enumerate()
                    .map(|(strip, label)| wrap_label(label, widths[strip].flatten()))
                    .collect();
                let tile_index = row * columns + column;
                Tile {
                    column,
                    row,
                    labels,
                    row_label: None,
                    x_scale_group: if free.0 { tile_index } else { 0 },
                    y_scale_group: if free.1 { tile_index } else { 0 },
                    has_x_axis: is_bottom || free.0,
                    has_y_axis: column == 0 || free.1,
                    is_bottom,
                    has_top_axis: row == 0 || free.0,
                    has_right_axis: is_row_end || free.1,
                    is_row_end,
                    levels: combination,
                }
            })
            .collect();
        tiles.sort_by_key(|tile| (tile.row, tile.column));
        Ok(Facets {
            variables,
            columns,
            rows,
            tiles,
            free_x: free.0,
            free_y: free.1,
            levels,
        })
    }

    /// `facet_grid` from its spec and each layer's faceting columns (variables in
    /// [`Facets::variables`] order: `x` first when given, then `y`). Levels sort
    /// by `x_order`/`y_order`, ascending by default (lets-plot's `1`).
    pub fn grid(spec: &FacetSpec, layers: &[(&LayerFacetColumns, usize)]) -> Result<Facets> {
        Facets::grid_with_functions(spec, layers, &AxisFunctions::default())
    }

    /// [`Facets::grid`], with the functions a `labeller` may refer to.
    pub fn grid_with_functions(
        spec: &FacetSpec,
        layers: &[(&LayerFacetColumns, usize)],
        functions: &AxisFunctions,
    ) -> Result<Facets> {
        let labeller = Labeller::parse(spec.labeller.as_ref(), functions, "facet.labeller")?;
        let variables = grid_variables(spec)?;
        let free = free_scales(spec)?;
        let x_defined = spec.x.is_some();
        let y_defined = spec.y.is_some();
        let mut levels = Vec::with_capacity(variables.len());
        // Label pattern and wrap width per variable.
        let mut strips = Vec::with_capacity(variables.len());
        for (index, name) in variables.iter().enumerate() {
            let is_x = index == 0 && x_defined;
            let key = if is_x { "x" } else { "y" };
            let (order, format, width) = if is_x {
                (spec.x_order, &spec.x_format, spec.x_labwidth)
            } else {
                (spec.y_order, &spec.y_format, spec.y_labwidth)
            };
            let values = collect_levels(name, key, index, layers, order.unwrap_or(1))?;
            let pattern = parse_format(format.as_deref(), &format!("{key}_format"))?;
            // Each margin is its own strip group of one variable (ggplot2).
            let level_labels = labeller.transform_levels(
                values
                    .iter()
                    .map(|value| format_level(value, pattern.as_ref()))
                    .collect(),
                &raw_texts(&values),
            )?;
            strips.push((level_labels, width));
            if values.is_empty() {
                return Err(Error::spec(
                    format!("facet.{key}"),
                    format!(
                        "no panel to draw: every row has a missing value in faceting column \
                         '{name}'"
                    ),
                ));
            }
            levels.push(values);
        }
        let columns = if x_defined { levels[0].len() } else { 1 };
        let rows = if y_defined {
            levels[usize::from(x_defined)].len()
        } else {
            1
        };
        let mut tiles = Vec::with_capacity(columns * rows);
        for row in 0..rows {
            for column in 0..columns {
                let mut combination = Vec::with_capacity(variables.len());
                if x_defined {
                    combination.push(column);
                }
                if y_defined {
                    combination.push(row);
                }
                let label = |variable: usize, level: usize| {
                    let (level_labels, width) = &strips[variable];
                    let strip = labeller
                        .strips(
                            &[variables[variable].as_str()],
                            vec![level_labels[level].clone()],
                        )
                        .into_iter()
                        .next()
                        .unwrap_or_else(|| Label::plain(String::new()));
                    wrap_label(strip, *width)
                };
                let labels = if x_defined && row == 0 {
                    vec![label(0, column)]
                } else {
                    Vec::new()
                };
                let row_label = (y_defined && column + 1 == columns)
                    .then(|| label(usize::from(x_defined), row));
                tiles.push(Tile {
                    column,
                    row,
                    labels,
                    row_label,
                    x_scale_group: if free.0 { column } else { 0 },
                    y_scale_group: if free.1 { row } else { 0 },
                    // lets-plot `FacetGrid.tileInfos`: axes on the outer panels only,
                    // free or not (free scales are shared along the grid).
                    has_x_axis: row + 1 == rows,
                    has_y_axis: column == 0,
                    is_bottom: row + 1 == rows,
                    has_top_axis: row == 0,
                    has_right_axis: column + 1 == columns,
                    is_row_end: column + 1 == columns,
                    levels: combination,
                });
            }
        }
        Ok(Facets {
            variables,
            columns,
            rows,
            tiles,
            free_x: free.0,
            free_y: free.1,
            levels,
        })
    }

    /// Rows of one layer per panel (in [`Facets::tiles`] order), from its faceting
    /// columns and row count.
    pub fn rows_by_tile(&self, columns: &LayerFacetColumns, row_count: usize) -> Vec<Vec<u32>> {
        let combinations: Vec<Vec<usize>> =
            self.tiles.iter().map(|tile| tile.levels.clone()).collect();
        Lookup::new(&self.levels).rows_by_tile(&combinations, columns, row_count)
    }
}

/// Faceting columns of a `facet_grid`: `x` then `y`, whichever are given.
pub fn grid_variables(spec: &FacetSpec) -> Result<Vec<String>> {
    match (&spec.x, &spec.y) {
        (None, None) => Err(Error::spec(
            "facet",
            "facet_grid needs a faceting column: give x (columns), y (rows) or both",
        )),
        (Some(x), Some(y)) if x == y => Err(Error::spec(
            "facet.y",
            format!("x and y must be different faceting columns, '{y}' is given twice"),
        )),
        (x, y) => Ok([x, y].into_iter().flatten().cloned().collect()),
    }
}

/// `scales`: whether x and y are free.
fn free_scales(spec: &FacetSpec) -> Result<(bool, bool)> {
    match spec.scales.as_deref().map(str::to_lowercase).as_deref() {
        None | Some("fixed") => Ok((false, false)),
        Some("free") => Ok((true, true)),
        Some("free_x") => Ok((true, false)),
        Some("free_y") => Ok((false, true)),
        Some(_) => Err(Error::spec(
            "facet.scales",
            format!(
                "unknown value '{}'; expected fixed, free, free_x or free_y",
                spec.scales.as_deref().unwrap_or_default()
            ),
        )),
    }
}

/// Levels of faceting variable `index` (named `name`, given by option `key`) over
/// every layer: first appearance, then sorted when `order` is not 0 (descending
/// below 0).
fn collect_levels(
    name: &str,
    key: &str,
    index: usize,
    layers: &[(&LayerFacetColumns, usize)],
    order: i64,
) -> Result<Vec<FacetValue>> {
    let mut found = false;
    let mut values: Vec<FacetValue> = Vec::new();
    let mut seen: HashMap<LevelKey, ()> = HashMap::new();
    for (columns, _) in layers {
        let Some(column) = &columns[index] else {
            continue;
        };
        found = true;
        for value in distinct_values(column) {
            if seen.insert(value.key(), ()).is_none() {
                values.push(value);
            }
        }
    }
    if !found {
        return Err(Error::spec(
            format!("facet.{key}"),
            format!("faceting column '{name}' is not in the data of any layer"),
        ));
    }
    if order != 0 {
        values.sort_by(compare_values);
        if order < 0 {
            values.reverse();
        }
    }
    Ok(values)
}

/// A strip label format (`format`, `x_format`, `y_format`), parsed; errors name
/// option `key`.
fn parse_format(format: Option<&str>, key: &str) -> Result<Option<LabelPattern>> {
    format
        .map(|pattern| {
            LabelPattern::parse(pattern)
                .map_err(|message| Error::spec(format!("facet.{key}"), message))
        })
        .transpose()
}

/// A strip label split at newlines and, for a positive `width`, wrapped at `width`
/// characters, the lines joined by [`LINE_BREAK`]. A label with a superscript stays
/// on one line.
fn wrap_label(mut label: Label, width: Option<i64>) -> Label {
    if label.superscript.is_none() {
        let lines: Vec<String> = match width.and_then(|width| usize::try_from(width).ok()) {
            Some(width) if width > 0 => wrap_words(&label.text, width),
            _ => label.text.split('\n').map(str::to_owned).collect(),
        };
        label.text = lines.join(&LINE_BREAK.to_string());
    }
    label
}

/// Plain decimal text of every numeric level (`4`, `4.5`), `None` for texts: the
/// keys an `as_labeller` lookup may use besides the label.
fn raw_texts(values: &[FacetValue]) -> Vec<Option<String>> {
    values
        .iter()
        .map(|value| match value {
            FacetValue::Text(_) => None,
            FacetValue::Number(number) => Some(number.to_string()),
        })
        .collect()
}

/// A level's label before line breaking: `pattern`, else the text itself or a
/// number in lets-plot's `{,~g}`.
fn format_level(value: &FacetValue, pattern: Option<&LabelPattern>) -> Label {
    match (pattern, value) {
        (Some(pattern), FacetValue::Text(text)) => pattern.format(PatternValue::Text(text)),
        (Some(pattern), FacetValue::Number(number)) => {
            pattern.format(PatternValue::Number(*number))
        }
        (None, FacetValue::Text(text)) => Label::plain(text.clone()),
        (None, FacetValue::Number(number)) => match LabelPattern::parse("{,~g}") {
            Ok(pattern) => pattern.format(PatternValue::Number(*number)),
            Err(_) => Label::plain(number.to_string()),
        },
    }
}

/// Level index of every value, per faceting variable.
struct Lookup {
    index_by_key: Vec<HashMap<LevelKey, usize>>,
}

impl Lookup {
    fn new(levels: &[Vec<FacetValue>]) -> Self {
        Lookup {
            index_by_key: levels
                .iter()
                .map(|values| {
                    values
                        .iter()
                        .enumerate()
                        .map(|(index, value)| (value.key(), index))
                        .collect()
                })
                .collect(),
        }
    }

    /// Rows of a layer per combination. A row belongs to every combination whose
    /// levels match its values; a variable the layer lacks matches any level.
    fn rows_by_tile(
        &self,
        combinations: &[Vec<usize>],
        columns: &LayerFacetColumns,
        row_count: usize,
    ) -> Vec<Vec<u32>> {
        // Level index per variable and row; `None` per variable: column absent.
        let row_levels: Vec<Option<Vec<Option<usize>>>> = columns
            .iter()
            .zip(&self.index_by_key)
            .map(|(column, index_by_key)| {
                column
                    .as_ref()
                    .map(|column| row_level_indices(column, index_by_key))
            })
            .collect();
        let mut rows_by_tile = vec![Vec::new(); combinations.len()];
        if row_levels.iter().all(Option::is_some) {
            // Every variable present: each row is in at most one panel.
            let tile_by_combination: HashMap<&[usize], usize> = combinations
                .iter()
                .enumerate()
                .map(|(tile, combination)| (combination.as_slice(), tile))
                .collect();
            let mut combination = vec![0; row_levels.len()];
            'rows: for row in 0..row_count {
                for (slot, levels) in combination.iter_mut().zip(row_levels.iter().flatten()) {
                    match levels[row] {
                        Some(level) => *slot = level,
                        None => continue 'rows,
                    }
                }
                if let Some(&tile) = tile_by_combination.get(combination.as_slice()) {
                    rows_by_tile[tile].push(row as u32);
                }
            }
            return rows_by_tile;
        }
        for (tile, combination) in combinations.iter().enumerate() {
            rows_by_tile[tile] = (0..row_count)
                .filter(|&row| {
                    row_levels
                        .iter()
                        .zip(combination)
                        .all(|(levels, wanted)| match levels {
                            None => true,
                            Some(levels) => levels[row] == Some(*wanted),
                        })
                })
                .map(|row| row as u32)
                .collect();
        }
        rows_by_tile
    }
}

/// Level index of every row (`None`: missing or unknown value).
fn row_level_indices(
    column: &Column,
    index_by_key: &HashMap<LevelKey, usize>,
) -> Vec<Option<usize>> {
    match column {
        Column::Discrete(column) => {
            let by_code: Vec<Option<usize>> = column
                .levels()
                .iter()
                .map(|level| index_by_key.get(&LevelKey::Text(level.clone())).copied())
                .collect();
            (0..column.len())
                .map(|row| column.code(row).and_then(|code| by_code[code as usize]))
                .collect()
        }
        Column::Continuous(column) => (0..column.len())
            .map(|row| {
                column
                    .get(row)
                    .filter(|value| value.is_finite())
                    .and_then(|value| index_by_key.get(&LevelKey::Number(value.to_bits())))
                    .copied()
            })
            .collect(),
    }
}

/// Distinct present values of a column: levels in their order (only those a row
/// uses), or finite numbers in order of first appearance. lets-plot's Python layer
/// turns NaN and infinities into missing values.
fn distinct_values(column: &Column) -> Vec<FacetValue> {
    match column {
        Column::Discrete(column) => {
            let mut used = vec![false; column.levels().len()];
            for row in 0..column.len() {
                if let Some(code) = column.code(row) {
                    used[code as usize] = true;
                }
            }
            column
                .levels()
                .iter()
                .zip(used)
                .filter(|(_, used)| *used)
                .map(|(level, _)| FacetValue::Text(level.clone()))
                .collect()
        }
        Column::Continuous(column) => {
            let mut seen = HashMap::new();
            (0..column.len())
                .filter_map(|row| column.get(row).filter(|value| value.is_finite()))
                .filter(|value| seen.insert(value.to_bits(), ()).is_none())
                .map(FacetValue::Number)
                .collect()
        }
    }
}

/// Natural order: numbers by value, texts by code point (Kotlin compares UTF-16
/// units, which agrees outside surrogate pairs); numbers before texts.
fn compare_values(left: &FacetValue, right: &FacetValue) -> std::cmp::Ordering {
    match (left, right) {
        (FacetValue::Number(left), FacetValue::Number(right)) => left.total_cmp(right),
        (FacetValue::Text(left), FacetValue::Text(right)) => left.cmp(right),
        (FacetValue::Number(_), FacetValue::Text(_)) => std::cmp::Ordering::Less,
        (FacetValue::Text(_), FacetValue::Number(_)) => std::cmp::Ordering::Greater,
    }
}

/// One option per faceting variable: a single value applies to the first variable
/// only, a list is padded with `None` (lets-plot pads with defaults).
fn per_variable<T: Clone>(
    option: Option<&crate::spec::OneOrMany<T>>,
    count: usize,
) -> Vec<Option<T>> {
    let given: Vec<Option<T>> = option
        .map(|option| option.as_slice().iter().cloned().map(Some).collect())
        .unwrap_or_default();
    (0..count)
        .map(|index| given.get(index).cloned().flatten())
        .collect()
}

/// lets-plot `FacetWrap.shape`: (columns, rows) of the grid for `count` panels.
fn shape(count: usize, ncol: Option<u32>, nrow: Option<u32>, vertical: bool) -> (usize, usize) {
    let count = count.max(1);
    let ceil_div = |numerator: usize, denominator: usize| numerator.div_ceil(denominator.max(1));
    let (width, height) = match (ncol, nrow) {
        (Some(ncol), _) => {
            let columns = (ncol as usize).min(count);
            (columns, ceil_div(count, columns).max(1))
        }
        (None, Some(nrow)) => {
            let rows = (nrow as usize).min(count);
            (ceil_div(count, rows), rows.max(1))
        }
        (None, None) => {
            let width = (count / 2).clamp(1, 4);
            (width, ceil_div(count, width).max(1))
        }
    };
    if vertical {
        (ceil_div(count, height), height)
    } else {
        (width, ceil_div(count, width))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::ContinuousColumn;

    fn spec(json: &str) -> FacetSpec {
        serde_json::from_str(json).unwrap()
    }

    fn texts(values: &[Option<&str>]) -> Column {
        let array: arrow_array::ArrayRef =
            std::sync::Arc::new(arrow_array::StringArray::from(values.to_vec()));
        let batch = arrow_array::RecordBatch::try_from_iter([("g", array)]).unwrap();
        crate::data::DataFrame::new(batch.schema(), vec![batch])
            .unwrap()
            .column("g")
            .unwrap()
    }

    fn labels(facets: &Facets) -> Vec<String> {
        facets
            .tiles
            .iter()
            .map(|tile| {
                tile.labels
                    .iter()
                    .map(ToString::to_string)
                    .collect::<Vec<_>>()
                    .join("/")
            })
            .collect()
    }

    #[test]
    fn shape_follows_lets_plot() {
        assert_eq!(shape(1, None, None, false), (1, 1));
        assert_eq!(shape(3, None, None, false), (1, 3));
        assert_eq!(shape(4, None, None, false), (2, 2));
        assert_eq!(shape(5, None, None, false), (2, 3));
        assert_eq!(shape(12, None, None, false), (4, 3));
        assert_eq!(shape(5, Some(3), None, false), (3, 2));
        assert_eq!(shape(5, Some(9), None, false), (5, 1));
        assert_eq!(shape(5, None, Some(2), false), (3, 2));
        assert_eq!(shape(5, None, Some(2), true), (3, 2));
        assert_eq!(shape(7, Some(3), None, true), (3, 3));
    }

    #[test]
    fn levels_sort_and_combinations_without_data_are_dropped() {
        let first = vec![Some(texts(&[Some("b"), Some("a"), None, Some("b")]))];
        let second = vec![Some(texts(&[Some("c")]))];
        let facets = Facets::wrap(
            &spec(r#"{"name": "wrap", "facets": "g", "order": 1}"#),
            &[(&first, 4), (&second, 1)],
        )
        .unwrap();
        assert_eq!(labels(&facets), ["a", "b", "c"]);
        assert_eq!(
            facets.rows_by_tile(&first, 4),
            [vec![1], vec![0, 3], vec![]]
        );
        // No order: first appearance across layers; descending.
        let facets =
            Facets::wrap(&spec(r#"{"name": "wrap", "facets": "g"}"#), &[(&first, 4)]).unwrap();
        assert_eq!(labels(&facets), ["b", "a"]);
        let facets = Facets::wrap(
            &spec(r#"{"name": "wrap", "facets": "g", "order": -1}"#),
            &[(&first, 4)],
        )
        .unwrap();
        assert_eq!(labels(&facets), ["b", "a"]);
    }

    #[test]
    fn two_variables_keep_present_combinations_and_layers_without_a_column_repeat() {
        let columns = vec![
            Some(texts(&[Some("p"), Some("q"), Some("p"), Some("q")])),
            Some(texts(&[None, Some("b"), Some("a"), Some("b")])),
        ];
        let facets = Facets::wrap(
            &spec(r#"{"name": "wrap", "facets": ["h", "g"], "order": [1, 1]}"#),
            &[(&columns, 4)],
        )
        .unwrap();
        assert_eq!(labels(&facets), ["p/a", "q/b"]);
        let partial = vec![Some(texts(&[Some("q"), Some("p")])), None];
        assert_eq!(facets.rows_by_tile(&partial, 2), [vec![1], vec![0]]);
        let absent = vec![None, None];
        assert_eq!(facets.rows_by_tile(&absent, 2), [vec![0, 1], vec![0, 1]]);
        // drop=False keeps every combination; one order value is the first column's.
        let facets = Facets::wrap(
            &spec(r#"{"name": "wrap", "facets": ["h", "g"], "order": 1, "drop": false}"#),
            &[(&columns, 4)],
        )
        .unwrap();
        assert_eq!(labels(&facets), ["p/b", "p/a", "q/b", "q/a"]);
    }

    #[test]
    fn numbers_sort_numerically_and_format_as_lets_plot() {
        let column = vec![Some(Column::Continuous(ContinuousColumn::from_options(
            vec![Some(10.0), Some(2.0), Some(f64::NAN), Some(2.5), Some(1e6)],
        )))];
        let facets = Facets::wrap(
            &spec(r#"{"name": "wrap", "facets": "g", "order": 1}"#),
            &[(&column, 5)],
        )
        .unwrap();
        assert_eq!(labels(&facets), ["2", "2.5", "10", "10^6"]);
        let facets = Facets::wrap(
            &spec(r#"{"name": "wrap", "facets": "g", "order": 1, "format": "{.1f} u"}"#),
            &[(&column, 5)],
        )
        .unwrap();
        assert_eq!(labels(&facets)[0], "2.0 u");
    }

    #[test]
    fn grid_positions_and_axes_follow_lets_plot() {
        let column = vec![Some(texts(&[
            Some("a"),
            Some("b"),
            Some("c"),
            Some("d"),
            Some("e"),
        ]))];
        let facets = Facets::wrap(
            &spec(r#"{"name": "wrap", "facets": "g", "ncol": 3}"#),
            &[(&column, 5)],
        )
        .unwrap();
        let cells: Vec<_> = facets
            .tiles
            .iter()
            .map(|tile| (tile.column, tile.row, tile.has_x_axis, tile.has_y_axis))
            .collect();
        // c has no panel below it: it draws the x axis.
        assert_eq!(
            cells,
            [
                (0, 0, false, true),
                (1, 0, false, false),
                (2, 0, true, false),
                (0, 1, true, true),
                (1, 1, true, false)
            ]
        );
        let facets = Facets::wrap(
            &spec(r#"{"name": "wrap", "facets": "g", "nrow": 2, "dir": "v", "scales": "free_y"}"#),
            &[(&column, 5)],
        )
        .unwrap();
        let order: Vec<_> = facets
            .tiles
            .iter()
            .map(|tile| (tile.labels[0].text.clone(), tile.has_y_axis))
            .collect();
        assert_eq!(
            order,
            [
                ("a".into(), true),
                ("c".into(), true),
                ("e".into(), true),
                ("b".into(), true),
                ("d".into(), true)
            ]
        );
    }

    #[test]
    fn grid_has_every_combination_with_outer_strips_and_axes() {
        let columns = vec![
            Some(texts(&[Some("b"), Some("a"), Some("b")])),
            Some(texts(&[Some("u"), Some("u"), Some("v")])),
        ];
        let facets = Facets::grid(
            &spec(r#"{"name": "grid", "x": "g", "y": "h", "scales": "free"}"#),
            &[(&columns, 3)],
        )
        .unwrap();
        assert_eq!((facets.columns, facets.rows), (2, 2));
        let cells: Vec<_> = facets
            .tiles
            .iter()
            .map(|tile| {
                (
                    tile.labels
                        .iter()
                        .map(ToString::to_string)
                        .collect::<Vec<_>>(),
                    tile.row_label.as_ref().map(ToString::to_string),
                    (tile.has_x_axis, tile.has_y_axis),
                    (tile.x_scale_group, tile.y_scale_group),
                )
            })
            .collect();
        let some = |text: &str| Some(text.to_owned());
        // Levels ascending; (a, v) has no rows and still is a panel.
        assert_eq!(
            cells,
            [
                (vec!["a".to_owned()], None, (false, true), (0, 0)),
                (vec!["b".to_owned()], some("u"), (false, false), (1, 0)),
                (vec![], None, (true, true), (0, 1)),
                (vec![], some("v"), (true, false), (1, 1)),
            ]
        );
        assert_eq!(
            facets.rows_by_tile(&columns, 3),
            [vec![1], vec![0], vec![], vec![2]]
        );
        // Only y: one column, row strips only; fixed scales share one group.
        let rows_only = vec![Some(texts(&[Some("u"), Some("v")]))];
        let facets =
            Facets::grid(&spec(r#"{"name": "grid", "y": "h"}"#), &[(&rows_only, 2)]).unwrap();
        assert_eq!((facets.columns, facets.rows), (1, 2));
        assert!(facets.tiles.iter().all(|tile| tile.labels.is_empty()
            && tile.row_label.is_some()
            && tile.y_scale_group == 0));
    }

    #[test]
    fn grid_orders_formats_and_wraps_each_axis() {
        let columns = vec![
            Some(texts(&[Some("b long name"), Some("a"), Some("c")])),
            Some(Column::Continuous(ContinuousColumn::from_options(vec![
                Some(1.5),
                Some(3.0),
                Some(2.25),
            ]))),
        ];
        let facets = Facets::grid(
            &spec(
                r#"{"name": "grid", "x": "g", "y": "n", "x_order": 0, "y_order": -1,
                    "y_format": ".2f", "x_labwidth": 4, "y_labwidth": 3}"#,
            ),
            &[(&columns, 3)],
        )
        .unwrap();
        let break_ = LINE_BREAK.to_string();
        // x: data order, wrapped at 4; y: descending, formatted, wrapped at 3.
        assert_eq!(
            labels(&facets)[..3],
            [format!("b{break_}long{break_}name"), "a".into(), "c".into()]
        );
        let rows: Vec<String> = facets
            .tiles
            .iter()
            .filter_map(|tile| tile.row_label.as_ref().map(ToString::to_string))
            .collect();
        assert_eq!(
            rows,
            [
                format!("3.0{break_}0"),
                format!("2.2{break_}5"),
                format!("1.5{break_}0")
            ]
        );
        // A format that is not supported names its option.
        let error = Facets::grid(
            &spec(r#"{"name": "grid", "x": "g", "x_format": "%Y"}"#),
            &[(&columns, 3)],
        )
        .unwrap_err()
        .to_string();
        assert!(error.contains("facet.x_format"), "{error}");
    }

    #[test]
    fn wrap_labwidth_matches_columns_and_newlines_always_split() {
        let columns = vec![
            Some(texts(&[Some("aa bb"), Some("c\nd")])),
            Some(texts(&[Some("pp qq"), Some("pp qq")])),
        ];
        let break_ = LINE_BREAK.to_string();
        // A single width applies to the first column only; 0 does not wrap.
        for (labwidth, first) in [("2", format!("aa{break_}bb")), ("0", "aa bb".to_owned())] {
            let facets = Facets::wrap(
                &spec(&format!(
                    r#"{{"name": "wrap", "facets": ["g", "h"], "order": 1, "labwidth": {labwidth}}}"#
                )),
                &[(&columns, 2)],
            )
            .unwrap();
            assert_eq!(
                labels(&facets),
                [format!("{first}/pp qq"), format!("c{break_}d/pp qq")]
            );
        }
        let facets = Facets::wrap(
            &spec(r#"{"name": "wrap", "facets": ["g", "h"], "order": 1, "labwidth": [null, 2]}"#),
            &[(&columns, 2)],
        )
        .unwrap();
        assert_eq!(labels(&facets)[0], format!("aa bb/pp{break_}qq"));
    }

    #[test]
    fn grid_errors_name_the_option() {
        let column = vec![Some(texts(&[Some("a")]))];
        for (json, key) in [
            (r#"{"name": "grid"}"#, "'facet'"),
            (r#"{"name": "grid", "x": "g", "y": "g"}"#, "facet.y"),
            (
                r#"{"name": "grid", "x": "g", "scales": "nope"}"#,
                "facet.scales",
            ),
        ] {
            let error = Facets::grid(&spec(json), &[(&column, 1)])
                .unwrap_err()
                .to_string();
            assert!(error.contains(key), "{json}: {error}");
        }
        let error = Facets::grid(
            &spec(r#"{"name": "grid", "x": "g", "y": "missing"}"#),
            &[(&vec![Some(texts(&[Some("a")])), None], 1)],
        )
        .unwrap_err()
        .to_string();
        assert!(
            error.contains("facet.y") && error.contains("'missing'"),
            "{error}"
        );
        let error = Facets::grid(
            &spec(r#"{"name": "grid", "x": "g"}"#),
            &[(&vec![Some(texts(&[None]))], 1)],
        )
        .unwrap_err()
        .to_string();
        assert!(
            error.contains("facet.x") && error.contains("no panel"),
            "{error}"
        );
    }

    #[test]
    fn invalid_options_are_errors_naming_the_option() {
        let column = vec![Some(texts(&[Some("a")]))];
        for (json, key) in [
            (
                r#"{"name": "wrap", "facets": "g", "scales": "loose"}"#,
                "facet.scales",
            ),
            (
                r#"{"name": "wrap", "facets": "g", "dir": "x"}"#,
                "facet.dir",
            ),
            (
                r#"{"name": "wrap", "facets": "g", "ncol": 0}"#,
                "facet.ncol",
            ),
            (r#"{"name": "wrap", "facets": ["g", "g"]}"#, "facet.facets"),
            (r#"{"name": "wrap", "facets": "missing"}"#, "facet.facets"),
        ] {
            let facets = if json.contains("missing") {
                Facets::wrap(&spec(json), &[(&vec![None], 1)])
            } else {
                Facets::wrap(&spec(json), &[(&column, 1)])
            };
            let error = facets.unwrap_err().to_string();
            assert!(error.contains(key), "{json}: {error}");
        }
    }
}

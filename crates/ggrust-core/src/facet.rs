//! Facets (`facet_wrap`): which panels a plot has, which data rows each panel
//! shows, and where panels sit in the grid (lets-plot `FacetConfig.createWrap`,
//! `FacetWrap`, `PlotFacets`).
//!
//! What it does: levels of every faceting column joined over the layers' data
//! (first appearance, then `order`), one panel per level combination that has data
//! (`drop`), strip labels (`format`, else lets-plot's `{,~g}` for numbers), the grid
//! shape (`ncol`, `nrow`, `dir`) and which panels draw axes. A layer without a
//! faceting column shows all its rows in every panel; rows with a missing value in
//! a faceting column are in no panel.
//!
//! What it does not do: `facet_grid`, `labwidth`, datetime labels. Building and
//! drawing the panels is `build` and `layout::facet`.

use std::collections::HashMap;

use crate::data::Column;
use crate::scale::format::Label;
use crate::scale::label_pattern::{LabelPattern, PatternValue};
use crate::spec::FacetSpec;
use crate::{Error, Result};

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
    /// Strip labels, one per faceting column, top first.
    pub labels: Vec<Label>,
    /// Whether the panel draws its x axis (the last panel of its column, or free x).
    pub has_x_axis: bool,
    /// Whether the panel draws its y axis (the first column, or free y).
    pub has_y_axis: bool,
    /// The last panel of its grid column.
    pub is_bottom: bool,
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
    /// `scales="free"` or `"free_x"`: every panel has its own x scale.
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
        let invalid =
            |key: &str, message: String| Error::InvalidSpec(format!("at 'facet.{key}': {message}"));
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
        let free = match spec.scales.as_deref().map(str::to_lowercase).as_deref() {
            None | Some("fixed") => (false, false),
            Some("free") => (true, true),
            Some("free_x") => (true, false),
            Some("free_y") => (false, true),
            Some(_) => {
                return Err(invalid(
                    "scales",
                    format!(
                        "unknown value '{}'; expected fixed, free, free_x or free_y",
                        spec.scales.as_deref().unwrap_or_default()
                    ),
                ));
            }
        };
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

        let mut levels = Vec::with_capacity(variables.len());
        for (index, name) in variables.iter().enumerate() {
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
                return Err(invalid(
                    "facets",
                    format!("faceting column '{name}' is not in the data of any layer"),
                ));
            }
            let order = orders[index].flatten().unwrap_or(0);
            if order != 0 {
                values.sort_by(compare_values);
                if order < 0 {
                    values.reverse();
                }
            }
            levels.push(values);
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
            .map(|(index, format)| match format.clone().flatten() {
                Some(pattern) => LabelPattern::parse(&pattern).map(Some).map_err(|message| {
                    invalid(
                        &match &spec.format {
                            Some(crate::spec::OneOrMany::Many(_)) => format!("format[{index}]"),
                            _ => "format".to_owned(),
                        },
                        message,
                    )
                }),
                None => Ok(None),
            })
            .collect::<Result<Vec<_>>>()?;
        let number_pattern = LabelPattern::parse("{,~g}").map_err(Error::InvalidSpec)?;

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
                let labels = combination
                    .iter()
                    .enumerate()
                    .map(|(variable, level)| {
                        let value = &levels[variable][*level];
                        match (&patterns[variable], value) {
                            (Some(pattern), FacetValue::Text(text)) => {
                                pattern.format(PatternValue::Text(text))
                            }
                            (Some(pattern), FacetValue::Number(number)) => {
                                pattern.format(PatternValue::Number(*number))
                            }
                            (None, FacetValue::Text(text)) => Label::plain(text.clone()),
                            (None, FacetValue::Number(number)) => {
                                number_pattern.format(PatternValue::Number(*number))
                            }
                        }
                    })
                    .collect();
                Tile {
                    column,
                    row,
                    labels,
                    has_x_axis: is_bottom || free.0,
                    has_y_axis: column == 0 || free.1,
                    is_bottom,
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

    /// Rows of one layer per panel (in [`Facets::tiles`] order), from its faceting
    /// columns and row count.
    pub fn rows_by_tile(&self, columns: &LayerFacetColumns, row_count: usize) -> Vec<Vec<u32>> {
        let combinations: Vec<Vec<usize>> =
            self.tiles.iter().map(|tile| tile.levels.clone()).collect();
        Lookup::new(&self.levels).rows_by_tile(&combinations, columns, row_count)
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

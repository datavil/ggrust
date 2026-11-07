//! Collected axes and axis titles in a grid: `gggrid(axes=, axis_titles=)`, as
//! patchwork's `plot_layout(axes=, axis_titles=)` (patchwork 1.2+, `collect_axes`
//! and `collect_axis_titles`). lets-plot has none.
//!
//! What it does, per screen side (bottom, top, left, right) on its own:
//!
//! - **Axes.** Plots whose axis on a side is equal (same screen scale, same axis
//!   theme elements, and for x the same column span, for y the same row span)
//!   form runs: down a column for x (then joined across neighbouring columns with
//!   the same rows), along a row for y. Each run keeps the axis of its outer plot
//!   (bottom-most for a bottom axis, top-most for a top axis, left-most for a left
//!   axis, right-most for a right axis); the others drop it with its room.
//! - **Titles.** Plots whose title on a side is equal (text and style) form runs
//!   the same way, without the span rule. Each run keeps one title, from the
//!   first plot on its outer edge (left-most for x, top-most for y). When any
//!   run spans more than one column (x) or row (y), every run's title is drawn
//!   by the grid, centred on the run's panels. A removed title keeps its room
//!   when another plot ending on the same grid line still has a title on that
//!   side (patchwork's shared gtable row), so the panels of a row stay level.
//!
//! What it does not do: empty cells, nested grids and faceted plots take no part
//! and break runs (patchwork collects facet axes of a plot too; ggrust leaves
//! faceted plots alone). Axes compare by their scales, not by drawn breaks, so
//! plots of different sizes with equal scales count as equal.

use crate::layout::grid::Span;
use crate::layout::side::{CollectedSides, Side, TitleRoom};
use crate::scale::PositionScale;
use crate::suggest::did_you_mean;
use crate::theme::{AxisLine, LineStyle, TextStyle};
use crate::{Error, Result};

/// One side's axis as a grid compares it.
#[derive(Debug, Clone)]
pub struct AxisKey {
    /// The screen scale of the axis (breaks, labels, sides, guides), as
    /// [`PositionScale::axis_identity`].
    pub scale: PositionScale,
    /// Tick labels.
    pub text: Option<TextStyle>,
    /// Major ticks.
    pub ticks: Option<LineStyle>,
    /// Minor ticks.
    pub minor_ticks: Option<LineStyle>,
    /// Axis line.
    pub line: Option<AxisLine>,
    /// Tick length, minor tick length, label spacing.
    pub lengths: [f64; 3],
}

/// One side's axis title as a grid compares it.
#[derive(Debug, Clone, Copy)]
pub struct TitleKey<'a> {
    /// The title text (never empty).
    pub text: &'a str,
    /// Its style.
    pub style: TextStyle,
}

/// What a plot shows on each side, in [`Side::ALL`] order (None: nothing there).
#[derive(Debug, Clone, Default)]
pub struct SideKeys<'a> {
    /// The axis on each side.
    pub axes: [Option<AxisKey>; 4],
    /// The axis title on each side.
    pub titles: [Option<TitleKey<'a>>; 4],
}

/// A value of `gggrid(axes=)` / `gggrid(axis_titles=)`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Collect {
    /// Every plot keeps its own (default).
    #[default]
    Keep,
    /// Collect x and y.
    Both,
    /// Collect the horizontal (x) sides only.
    X,
    /// Collect the vertical (y) sides only.
    Y,
}

/// Values [`Collect::parse`] accepts.
pub const COLLECT_VALUES: [&str; 4] = ["keep", "collect", "collect_x", "collect_y"];

impl Collect {
    /// Parse the layout option `key` (`axes` or `axis_titles`).
    pub fn parse(value: Option<&str>, key: &str) -> Result<Collect> {
        match value {
            None | Some("keep") => Ok(Collect::Keep),
            Some("collect") => Ok(Collect::Both),
            Some("collect_x") => Ok(Collect::X),
            Some("collect_y") => Ok(Collect::Y),
            Some(other) => Err(Error::spec(
                format!("layout.{key}"),
                format!(
                    "expected 'keep', 'collect', 'collect_x' or 'collect_y', got '{other}'{}",
                    did_you_mean(other, COLLECT_VALUES)
                ),
            )),
        }
    }

    /// Whether sides of this direction are collected.
    fn covers(self, side: Side) -> bool {
        match self {
            Collect::Keep => false,
            Collect::Both => true,
            Collect::X => side.is_horizontal(),
            Collect::Y => !side.is_horizontal(),
        }
    }
}

/// A title the grid draws itself, centred on a run of plots' panels.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct RunTitle {
    /// The plot whose title (text, style, distance from its panel) is drawn.
    pub figure: usize,
    /// Its side.
    pub side: Side,
    /// The plots of the run (`figure` among them): the title spans their panels.
    pub members: Vec<usize>,
}

/// What a grid collected: per figure its [`CollectedSides`], and the titles the
/// grid draws.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct GridAxes {
    /// Per figure (all default when nothing is collected).
    pub collected: Vec<CollectedSides>,
    /// Titles drawn by the grid across runs.
    pub run_titles: Vec<RunTitle>,
}

/// A run of equal cells: rows and columns inclusive.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Run {
    rows: (usize, usize),
    columns: (usize, usize),
}

/// Collect the axes and titles of a grid of `columns` x `rows` cells whose
/// figures span `spans` and show `keys` (None: an empty cell, nested grid or
/// faceted plot).
pub fn collect(
    axes: Collect,
    titles: Collect,
    (columns, rows): (usize, usize),
    spans: &[Span],
    keys: &[Option<SideKeys>],
) -> GridAxes {
    let mut result = GridAxes {
        collected: vec![CollectedSides::default(); spans.len()],
        run_titles: Vec::new(),
    };
    if axes == Collect::Keep && titles == Collect::Keep {
        return result;
    }
    // The figure in each cell.
    let mut cells: Vec<Vec<Option<usize>>> = vec![vec![None; columns]; rows];
    for (figure, span) in spans.iter().enumerate() {
        if keys.get(figure).is_none_or(Option::is_none) {
            continue;
        }
        for row in cells.iter_mut().take(span.bottom + 1).skip(span.top) {
            for cell in row.iter_mut().take(span.right + 1).skip(span.left) {
                *cell = Some(figure);
            }
        }
    }
    for side in Side::ALL {
        if axes.covers(side) {
            collect_axes(side, &cells, spans, keys, &mut result);
        }
        if titles.covers(side) {
            collect_titles(side, &cells, spans, keys, &mut result);
        }
    }
    result
}

/// patchwork `collect_axes` for one side.
fn collect_axes(
    side: Side,
    cells: &[Vec<Option<usize>>],
    spans: &[Span],
    keys: &[Option<SideKeys>],
    result: &mut GridAxes,
) {
    let index = side.index();
    // Equal axes spanning the same columns (x) or rows (y): patchwork hashes the
    // grob with its extent.
    let extent = |figure: usize| {
        let span = spans[figure];
        if side.is_horizontal() {
            (span.left, span.right)
        } else {
            (span.top, span.bottom)
        }
    };
    let classes = classes((0..keys.len()).map(|figure| {
        let key = keys[figure]
            .as_ref()
            .and_then(|keys| keys.axes[index].as_ref())?;
        Some(format!("{key:?} {:?}", extent(figure)))
    }));
    let has = |figure: usize| {
        keys[figure]
            .as_ref()
            .is_some_and(|keys| keys.axes[index].is_some())
    };
    let values = class_matrix(cells, &classes, has);
    let mut keepers = Vec::new();
    for run in runs(&values, !side.is_horizontal()) {
        let row = if side == Side::Bottom {
            run.rows.1
        } else {
            run.rows.0
        };
        let column = if side == Side::Right {
            run.columns.1
        } else {
            run.columns.0
        };
        keepers.extend(cells[row][column]);
    }
    for figure in 0..spans.len() {
        if has(figure) && !keepers.contains(&figure) {
            result.collected[figure].axes[index] = true;
        }
    }
}

/// patchwork `collect_axis_titles` (`merge = TRUE`) for one side.
fn collect_titles(
    side: Side,
    cells: &[Vec<Option<usize>>],
    spans: &[Span],
    keys: &[Option<SideKeys>],
    result: &mut GridAxes,
) {
    let index = side.index();
    let key = |figure: usize| keys[figure].as_ref().and_then(|keys| keys.titles[index]);
    let classes = classes((0..keys.len()).map(|figure| Some(format!("{:?}", key(figure)?))));
    let has = |figure: usize| key(figure).is_some();
    let values = class_matrix(cells, &classes, has);
    let found = runs(&values, !side.is_horizontal());
    // (first, members) per run.
    let mut kept: Vec<(usize, Vec<usize>)> = Vec::new();
    for run in &found {
        let edge: Vec<Option<usize>> = match side {
            Side::Bottom => (run.columns.0..=run.columns.1)
                .map(|column| cells[run.rows.1][column])
                .collect(),
            Side::Top => (run.columns.0..=run.columns.1)
                .map(|column| cells[run.rows.0][column])
                .collect(),
            Side::Left => (run.rows.0..=run.rows.1)
                .map(|row| cells[row][run.columns.0])
                .collect(),
            Side::Right => (run.rows.0..=run.rows.1)
                .map(|row| cells[row][run.columns.1])
                .collect(),
        };
        let Some(first) = edge.into_iter().flatten().next() else {
            continue;
        };
        let mut members = vec![first];
        for row in &cells[run.rows.0..=run.rows.1] {
            for figure in row[run.columns.0..=run.columns.1].iter().flatten() {
                if !members.contains(figure) {
                    members.push(*figure);
                }
            }
        }
        kept.push((first, members));
    }
    let firsts: Vec<usize> = kept.iter().map(|(first, _)| *first).collect();
    let removed: Vec<usize> = (0..spans.len())
        .filter(|figure| has(*figure) && !firsts.contains(figure))
        .collect();
    // The grid line a title on `side` sits along.
    let line = |figure: usize| {
        let span = spans[figure];
        match side {
            Side::Bottom => span.bottom,
            Side::Top => span.top,
            Side::Left => span.left,
            Side::Right => span.right,
        }
    };
    for &figure in &removed {
        let shared = (0..spans.len())
            .any(|other| has(other) && !removed.contains(&other) && line(other) == line(figure));
        result.collected[figure].titles[index] = if shared {
            TitleRoom::Hidden
        } else {
            TitleRoom::Dropped
        };
    }
    // patchwork extends the kept titles over their runs only when some run spans
    // more than one column (x) or row (y).
    let spanning = found.iter().any(|run| {
        if side.is_horizontal() {
            run.columns.0 != run.columns.1
        } else {
            run.rows.0 != run.rows.1
        }
    });
    if spanning {
        for (first, members) in kept {
            result.collected[first].titles[index] = TitleRoom::Hidden;
            result.run_titles.push(RunTitle {
                figure: first,
                side,
                members,
            });
        }
    }
}

/// Class ids of figures from their canonical keys (None: nothing on that
/// side). Keys are `Debug` texts: unset style values are NaN, so a derived
/// `PartialEq` would never match two default-styled axes (as `Styles::same_values`
/// works around for legends); `Debug` writes NaN the same each time.
fn classes(keys: impl Iterator<Item = Option<String>>) -> Vec<Option<usize>> {
    let mut seen: Vec<String> = Vec::new();
    keys.map(|key| {
        let key = key?;
        Some(
            seen.iter()
                .position(|other| *other == key)
                .unwrap_or_else(|| {
                    seen.push(key);
                    seen.len() - 1
                }),
        )
    })
    .collect()
}

/// Each cell's class (None: empty, or its figure shows nothing on the side).
fn class_matrix(
    cells: &[Vec<Option<usize>>],
    classes: &[Option<usize>],
    has: impl Fn(usize) -> bool,
) -> Vec<Vec<Option<usize>>> {
    cells
        .iter()
        .map(|row| {
            row.iter()
                .map(|cell| cell.filter(|figure| has(*figure)).and_then(|f| classes[f]))
                .collect()
        })
        .collect()
}

/// patchwork `rle_2d`: runs of equal values down each column, joined across
/// neighbouring columns with the same rows and value (`along_rows`: along each
/// row, joined across neighbouring rows). Runs of None are left out.
fn runs(values: &[Vec<Option<usize>>], along_rows: bool) -> Vec<Run> {
    if along_rows {
        let rows = values.len();
        let columns = values.first().map_or(0, Vec::len);
        let transposed: Vec<Vec<Option<usize>>> = (0..columns)
            .map(|column| (0..rows).map(|row| values[row][column]).collect())
            .collect();
        return runs(&transposed, false)
            .into_iter()
            .map(|run| Run {
                rows: run.columns,
                columns: run.rows,
            })
            .collect();
    }
    let rows = values.len();
    let columns = values.first().map_or(0, Vec::len);
    let mut found: Vec<(Run, Option<usize>)> = Vec::new();
    #[allow(clippy::needless_range_loop)] // `column` is also a run bound
    for column in 0..columns {
        let mut start = 0;
        while start < rows {
            let value = values[start][column];
            let mut end = start;
            while end + 1 < rows && values[end + 1][column] == value {
                end += 1;
            }
            let joined = found.iter_mut().rev().find(|(run, other)| {
                run.rows == (start, end) && *other == value && run.columns.1 + 1 == column
            });
            match joined {
                Some((run, _)) => run.columns.1 = column,
                None => found.push((
                    Run {
                        rows: (start, end),
                        columns: (column, column),
                    },
                    value,
                )),
            }
            start = end + 1;
        }
    }
    found
        .into_iter()
        .filter(|(_, value)| value.is_some())
        .map(|(run, _)| run)
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scale::ContinuousPositionScale;

    fn span(left: usize, top: usize, right: usize, bottom: usize) -> Span {
        Span {
            left,
            top,
            right,
            bottom,
        }
    }

    fn grid_spans(columns: usize, count: usize) -> Vec<Span> {
        (0..count)
            .map(|index| {
                let (column, row) = (index % columns, index / columns);
                span(column, row, column, row)
            })
            .collect()
    }

    fn style() -> TextStyle {
        crate::theme::Theme::default()
            .axis_title_x
            .expect("default theme has an x title")
    }

    /// Keys of a plot with bottom and left axes of `scale` and titles x / y.
    fn keys<'a>(scale: &'a PositionScale, x_title: &'a str) -> SideKeys<'a> {
        let axis = AxisKey {
            scale: scale.clone(),
            text: None,
            ticks: None,
            minor_ticks: None,
            line: None,
            lengths: [0.0; 3],
        };
        let title = |text| {
            Some(TitleKey {
                text,
                style: style(),
            })
        };
        SideKeys {
            axes: [Some(axis.clone()), None, Some(axis), None],
            titles: [title(x_title), None, title("y"), None],
        }
    }

    fn scale() -> PositionScale {
        PositionScale::Continuous(ContinuousPositionScale::default())
    }

    #[test]
    fn two_by_two_keeps_the_outer_axes() {
        let scale = scale();
        let all: Vec<Option<SideKeys>> = (0..4).map(|_| Some(keys(&scale, "x"))).collect();
        let result = collect(
            Collect::Both,
            Collect::Keep,
            (2, 2),
            &grid_spans(2, 4),
            &all,
        );
        let hidden = |figure: usize, side: Side| result.collected[figure].axis_hidden(side);
        assert!(hidden(0, Side::Bottom) && hidden(1, Side::Bottom));
        assert!(!hidden(2, Side::Bottom) && !hidden(3, Side::Bottom));
        assert!(!hidden(0, Side::Left) && !hidden(2, Side::Left));
        assert!(hidden(1, Side::Left) && hidden(3, Side::Left));
        assert!(result.run_titles.is_empty());
    }

    #[test]
    fn collect_x_leaves_y_axes() {
        let scale = scale();
        let all: Vec<Option<SideKeys>> = (0..4).map(|_| Some(keys(&scale, "x"))).collect();
        let result = collect(Collect::X, Collect::Keep, (2, 2), &grid_spans(2, 4), &all);
        assert!(result.collected[0].axis_hidden(Side::Bottom));
        assert!(!result.collected[1].axis_hidden(Side::Left));
    }

    #[test]
    fn different_axes_and_empty_cells_break_runs() {
        let scale = scale();
        let mut other_scale = ContinuousPositionScale::default();
        other_scale.limits = [Some(0.0), Some(1.0)];
        let other = PositionScale::Continuous(other_scale);
        // Column 0: equal axes; column 1: different ones; row 1 col 2 empty.
        let all = vec![
            Some(keys(&scale, "x")),
            Some(keys(&scale, "x")),
            Some(keys(&scale, "x")),
            Some(keys(&scale, "x")),
            Some(keys(&other, "x")),
            None,
        ];
        let result = collect(
            Collect::Both,
            Collect::Keep,
            (3, 2),
            &grid_spans(3, 6),
            &all,
        );
        assert!(result.collected[0].axis_hidden(Side::Bottom));
        assert!(!result.collected[1].axis_hidden(Side::Bottom));
        // Above an empty cell the axis stays (cellestial's unfinished last row).
        assert!(!result.collected[2].axis_hidden(Side::Bottom));
        // Row 1: plot 4 differs, so plot 3 keeps its left axis.
        assert!(!result.collected[3].axis_hidden(Side::Left));
        assert!(!result.collected[4].axis_hidden(Side::Left));
    }

    #[test]
    fn titles_collect_into_one_per_run() {
        let scale = scale();
        let all: Vec<Option<SideKeys>> = (0..4).map(|_| Some(keys(&scale, "x"))).collect();
        let result = collect(
            Collect::Keep,
            Collect::Both,
            (2, 2),
            &grid_spans(2, 4),
            &all,
        );
        let title = |figure: usize, side: Side| result.collected[figure].title(side);
        // x: plot 2 (bottom left) draws over both columns; 3 keeps the room.
        assert_eq!(title(0, Side::Bottom), TitleRoom::Dropped);
        assert_eq!(title(1, Side::Bottom), TitleRoom::Dropped);
        assert_eq!(title(2, Side::Bottom), TitleRoom::Hidden);
        assert_eq!(title(3, Side::Bottom), TitleRoom::Hidden);
        // y: plot 0 (top left) draws over both rows.
        assert_eq!(title(0, Side::Left), TitleRoom::Hidden);
        assert_eq!(title(2, Side::Left), TitleRoom::Hidden);
        assert_eq!(title(1, Side::Left), TitleRoom::Dropped);
        assert_eq!(
            result.run_titles,
            vec![
                RunTitle {
                    figure: 2,
                    side: Side::Bottom,
                    members: vec![2, 0, 1, 3]
                },
                RunTitle {
                    figure: 0,
                    side: Side::Left,
                    members: vec![0, 1, 2, 3]
                },
            ]
        );
    }

    #[test]
    fn single_column_titles_stay_with_their_plot() {
        let scale = scale();
        let all: Vec<Option<SideKeys>> = (0..2).map(|_| Some(keys(&scale, "x"))).collect();
        let result = collect(Collect::Keep, Collect::X, (1, 2), &grid_spans(1, 2), &all);
        assert_eq!(result.collected[0].title(Side::Bottom), TitleRoom::Dropped);
        assert_eq!(result.collected[1].title(Side::Bottom), TitleRoom::Drawn);
        assert!(result.run_titles.is_empty());
    }

    #[test]
    fn design_spans_compare_their_extent() {
        let scale = scale();
        // "AB\nCC": C spans both columns, so A's and B's axes are not C's.
        let spans = vec![span(0, 0, 0, 0), span(1, 0, 1, 0), span(0, 1, 1, 1)];
        let all: Vec<Option<SideKeys>> = (0..3).map(|_| Some(keys(&scale, "x"))).collect();
        let result = collect(Collect::Both, Collect::Both, (2, 2), &spans, &all);
        assert!(!result.collected[0].axis_hidden(Side::Bottom));
        assert!(!result.collected[1].axis_hidden(Side::Bottom));
        // Titles ignore the extent: one x title under the whole run.
        assert_eq!(result.run_titles[0].figure, 2);
        assert_eq!(result.collected[0].title(Side::Bottom), TitleRoom::Dropped);
        // y: A keeps its axis, B drops it (same rows, equal).
        assert!(result.collected[1].axis_hidden(Side::Left));
    }

    #[test]
    fn unknown_values_suggest_a_close_one() {
        let error = Collect::parse(Some("colect"), "axes")
            .unwrap_err()
            .to_string();
        assert!(error.contains("layout.axes"), "{error}");
        assert!(error.contains("did you mean 'collect'"), "{error}");
    }
}

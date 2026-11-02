//! Contours of z over an (x, y) grid: the shared core of `stat="contour"`,
//! `"contourf"`, `"density2d"` and `"density2df"` (lets-plot `ContourStatUtil`,
//! `ContourFillHelper`), and the first two stats.
//!
//! The grid is built from (x, y, z) rows in any order: the sorted distinct x and y
//! values are its columns and rows (lets-plot #1357 needs x-major sorted rows). A
//! missing node (no row, or a non-finite z) leaves the cells around it empty;
//! z = 0 is a value (lets-plot #1356 drops it).
//!
//! Each cell is split into four triangles around its centre, whose value is the
//! mean of the two middle corner values (lets-plot; it settles saddle cells). z is
//! linear on a triangle, so:
//! - isolines: one segment per crossed triangle (lets-plot's table, higher z on the
//!   right), joined into paths; closed paths repeat their first vertex;
//! - isobands: every triangle clipped to `low <= z < high`; edges shared by two
//!   pieces cancel and the rest join into closed rings, outer rings counter-clockwise
//!   and holes clockwise (in data space), so the nonzero fill rule leaves holes.
//!   Vertices are symbolic (a grid point, or where a grid edge crosses a level), so
//!   shared edges match exactly.
//!
//! Levels: `bins` (default 10) equal steps over the z range, or `binwidth` steps,
//! each shifted half a step up (lets-plot `computeLevels`). `..level..` of a line is
//! its level; of a band, lets-plot's fill level: the z minimum (lowest band), the
//! mean of its two levels, or the z maximum (highest band). Bands fill by
//! `..level..` by default, as ggplot2 (lets-plot draws them all in one blue).
//!
//! What it does not do: scattered (non-grid) x/y, interpolation of missing nodes.

use super::bin::MAX_BIN_COUNT;
use super::groups::{GroupColumns, per_group_pieces};
use super::{Stat, StatContext};
use crate::data::{Computed, ContinuousColumn, LayerData, Variable};
use crate::parallel;
use crate::parameters::Parameters;
use crate::scale::breaks::is_degenerate_range;
use crate::spec::Aesthetic;
use crate::{Error, Result};
use ahash::AHashMap;

/// lets-plot `ContourStat.DEF_BIN_COUNT`.
const DEFAULT_LEVEL_COUNT: usize = 10;

/// Grids with more nodes than this are refused (about 128 MB of z).
const MAX_GRID_NODES: usize = 1 << 24;

/// `stat="contour"`: isolines of z.
pub struct ContourStat;

/// `stat="contourf"`: filled bands between the isolines of z.
pub struct ContourfStat;

impl Stat for ContourStat {
    fn name(&self) -> &'static str {
        "contour"
    }

    fn parameters(&self) -> &'static [&'static str] {
        &["bins", "binwidth"]
    }

    fn aesthetics(&self) -> &'static [Aesthetic] {
        &[Aesthetic::X, Aesthetic::Y, Aesthetic::Z]
    }

    fn required_aesthetics(&self) -> &'static [Aesthetic] {
        &[Aesthetic::X, Aesthetic::Y, Aesthetic::Z]
    }

    fn compute(
        &self,
        data: LayerData,
        parameters: &Parameters,
        _context: &StatContext,
    ) -> Result<LayerData> {
        compute_from_rows(&data, parameters, false)
    }
}

impl Stat for ContourfStat {
    fn name(&self) -> &'static str {
        "contourf"
    }

    fn parameters(&self) -> &'static [&'static str] {
        ContourStat.parameters()
    }

    fn aesthetics(&self) -> &'static [Aesthetic] {
        ContourStat.aesthetics()
    }

    fn required_aesthetics(&self) -> &'static [Aesthetic] {
        ContourStat.required_aesthetics()
    }

    /// ggplot2's default (`fill = after_stat(level)`); lets-plot draws every band
    /// in one blue (deviation, the user's choice).
    fn default_mapping(&self) -> &'static [(Aesthetic, Computed)] {
        &[(Aesthetic::Fill, Computed::Level)]
    }

    fn compute(
        &self,
        data: LayerData,
        parameters: &Parameters,
        _context: &StatContext,
    ) -> Result<LayerData> {
        compute_from_rows(&data, parameters, true)
    }
}

/// Contours of the (x, y, z) rows of every group.
fn compute_from_rows(data: &LayerData, parameters: &Parameters, filled: bool) -> Result<LayerData> {
    let options = LevelOptions::from_parameters(parameters)?;
    let empty = ContinuousColumn::from_values(Vec::new());
    let x = data.continuous(Aesthetic::X).unwrap_or(&empty);
    let y = data.continuous(Aesthetic::Y).unwrap_or(&empty);
    if data.get(Aesthetic::Z).is_some() && data.continuous(Aesthetic::Z).is_none() {
        return Err(Error::spec(
            parameters.path_of("z"),
            "z must be numeric (the height of each grid node), got a discrete column; \
             map z to a numeric column, or use geom_tile(aes(fill=...)) for categories",
        ));
    }
    let z = data.continuous(Aesthetic::Z).unwrap_or(&empty);
    per_group_pieces(data, |rows| {
        let value = |column: &ContinuousColumn, row: usize| column.get(row).unwrap_or(f64::NAN);
        let nodes: Vec<(f64, f64, f64)> = rows
            .iter()
            .map(|&row| (value(x, row), value(y, row), value(z, row)))
            .collect();
        let Some(grid) = Grid::from_nodes(&nodes)? else {
            return Ok(contour_columns(Vec::new()));
        };
        let pieces = if filled {
            grid.bands(&options, true)
        } else {
            grid.lines(&options)
        };
        Ok(contour_columns(pieces))
    })
}

/// One output piece (a path, or a band's rings) and its `..level..`.
pub(super) struct Piece {
    pub(super) points: Vec<(f64, f64)>,
    pub(super) level: f64,
}

/// x, y and `..level..` columns, one output group per piece.
pub(super) fn contour_columns(pieces: Vec<Piece>) -> (GroupColumns, Option<Vec<usize>>) {
    let total: usize = pieces.iter().map(|piece| piece.points.len()).sum();
    let mut x = Vec::with_capacity(total);
    let mut y = Vec::with_capacity(total);
    let mut level = Vec::with_capacity(total);
    let mut sizes = Vec::with_capacity(pieces.len());
    for piece in pieces {
        sizes.push(piece.points.len());
        level.extend(std::iter::repeat_n(piece.level, piece.points.len()));
        for (point_x, point_y) in piece.points {
            x.push(point_x);
            y.push(point_y);
        }
    }
    (
        vec![
            (Variable::Aesthetic(Aesthetic::X), x),
            (Variable::Aesthetic(Aesthetic::Y), y),
            (Variable::Computed(Computed::Level), level),
        ],
        Some(sizes),
    )
}

/// `bins` and `binwidth` (lets-plot `BinStatUtil.BinOptions` for levels).
#[derive(Clone, Copy, Debug)]
pub(super) struct LevelOptions {
    count: usize,
    /// Positive, or `None`.
    width: Option<f64>,
}

impl LevelOptions {
    pub(super) fn from_parameters(parameters: &Parameters) -> Result<Self> {
        let count = parameters
            .number("bins")?
            .unwrap_or(DEFAULT_LEVEL_COUNT as f64);
        // As lets-plot: an integer clamped to 1..500.
        let count = (count.trunc().max(1.0) as usize).min(MAX_BIN_COUNT);
        let width = parameters.number("binwidth")?.filter(|width| *width > 0.0);
        Ok(LevelOptions { count, width })
    }

    /// lets-plot `computeLevels`: `None` for a degenerate z range.
    pub(super) fn levels(&self, (low, high): (f64, f64)) -> Option<Vec<f64>> {
        if is_degenerate_range(low, high) {
            return None;
        }
        let span = high - low;
        let (count, width) = match self.width {
            Some(width) => {
                let count = (span / width).min(MAX_BIN_COUNT as f64).ceil();
                (count.max(0.0) as usize, width)
            }
            None => (self.count, span / self.count as f64),
        };
        Some(
            (0..count)
                .map(|index| index as f64 * width + low + width / 2.0)
                .collect(),
        )
    }
}

/// z on a rectangular grid.
pub(super) struct Grid {
    /// Column positions, ascending.
    xs: Vec<f64>,
    /// Row positions, ascending.
    ys: Vec<f64>,
    /// Row-major (row = y index); NaN marks a missing node.
    z: Vec<f64>,
}

/// A contour vertex: a grid point (`low == high`, `level` unused) or where the
/// grid edge between points `low` and `high` crosses level `level`.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash, PartialOrd, Ord)]
struct Vertex {
    low: u32,
    high: u32,
    level: u32,
}

impl Vertex {
    fn point(id: u32) -> Vertex {
        Vertex {
            low: id,
            high: id,
            level: u32::MAX,
        }
    }

    fn is_point(self) -> bool {
        self.low == self.high
    }
}

/// A polygon corner while clipping a triangle.
#[derive(Clone, Copy, Debug)]
struct Corner {
    vertex: Vertex,
    z: f64,
}

impl Grid {
    /// A grid from known positions and row-major z.
    pub(super) fn new(xs: Vec<f64>, ys: Vec<f64>, z: Vec<f64>) -> Grid {
        debug_assert_eq!(xs.len() * ys.len(), z.len());
        Grid { xs, ys, z }
    }

    /// The grid of (x, y, z) nodes in any order; `None` with fewer than two
    /// distinct finite x or y. A repeated (x, y) keeps its last z.
    pub(super) fn from_nodes(nodes: &[(f64, f64, f64)]) -> Result<Option<Grid>> {
        let finite: Vec<&(f64, f64, f64)> = nodes
            .iter()
            .filter(|(x, y, _)| x.is_finite() && y.is_finite())
            .collect();
        let distinct = |values: Vec<f64>| -> Vec<f64> {
            let mut values = values;
            values.sort_by(f64::total_cmp);
            values.dedup();
            values
        };
        let xs = distinct(finite.iter().map(|node| node.0).collect());
        let ys = distinct(finite.iter().map(|node| node.1).collect());
        if xs.len() < 2 || ys.len() < 2 {
            return Ok(None);
        }
        let node_count = xs.len().saturating_mul(ys.len());
        if node_count > MAX_GRID_NODES {
            return Err(Error::InvalidData {
                message: format!(
                    "contours need z on a grid of x and y values, but the {} rows have {} \
                     distinct x and {} distinct y values ({node_count} grid nodes, at most \
                     {MAX_GRID_NODES}); for scattered points use geom_density2d(), or \
                     interpolate z onto a grid first",
                    finite.len(),
                    xs.len(),
                    ys.len()
                ),
                notes: String::new(),
            });
        }
        let mut z = vec![f64::NAN; node_count];
        let index = |values: &[f64], value: f64| {
            values
                .binary_search_by(|probe| probe.total_cmp(&value))
                .unwrap_or_default()
        };
        for &&(x, y, value) in &finite {
            z[index(&ys, y) * xs.len() + index(&xs, x)] = value;
        }
        Ok(Some(Grid { xs, ys, z }))
    }

    /// Range of the finite z values.
    pub(super) fn z_range(&self) -> Option<(f64, f64)> {
        self.z
            .iter()
            .filter(|value| value.is_finite())
            .fold(None, |range, &value| match range {
                None => Some((value, value)),
                Some((low, high)) => Some((f64::min(low, value), f64::max(high, value))),
            })
    }

    /// Isolines at the levels of `options`, level by level.
    pub(super) fn lines(&self, options: &LevelOptions) -> Vec<Piece> {
        let Some(levels) = self.z_range().and_then(|range| options.levels(range)) else {
            return Vec::new();
        };
        let cells = Cells::new(self);
        let indices: Vec<usize> = (0..levels.len()).collect();
        let parallel = parallel::is_worth(self.z.len() * levels.len());
        let per_level: Vec<Vec<Piece>> = parallel::ordered_map(indices, parallel, |index| {
            let level = levels[index];
            cells
                .isolines(index as u32, level, &levels)
                .into_iter()
                .map(|points| Piece { points, level })
                .collect()
        });
        per_level.into_iter().flatten().collect()
    }

    /// Filled bands between the levels of `options`, lowest first; one piece per
    /// band with its rings one after another. `lowest` keeps the band below the
    /// first level (lets-plot's `density2df` drops it).
    pub(super) fn bands(&self, options: &LevelOptions, lowest: bool) -> Vec<Piece> {
        let Some((z_low, z_high)) = self.z_range() else {
            return Vec::new();
        };
        let Some(levels) = options.levels((z_low, z_high)) else {
            return Vec::new();
        };
        // lets-plot `computeFillLevels`.
        let mut fill_levels = vec![z_low];
        fill_levels.extend(levels.windows(2).map(|pair| (pair[0] + pair[1]) / 2.0));
        fill_levels.push(z_high);
        let cells = Cells::new(self);
        let first = usize::from(!lowest);
        let bands: Vec<usize> = (first..=levels.len()).collect();
        let parallel = parallel::is_worth(self.z.len() * bands.len());
        let pieces = parallel::ordered_map(bands, parallel, |band| Piece {
            points: cells.isoband(band, &levels),
            level: fill_levels[band],
        });
        pieces
            .into_iter()
            .filter(|piece| !piece.points.is_empty())
            .collect()
    }
}

/// The grid with the centre value of every complete cell.
struct Cells<'a> {
    grid: &'a Grid,
    /// Per cell (row-major); NaN when a corner is missing.
    centres: Vec<f64>,
}

impl<'a> Cells<'a> {
    fn new(grid: &'a Grid) -> Cells<'a> {
        let columns = grid.xs.len() - 1;
        let rows = grid.ys.len() - 1;
        let mut centres = Vec::with_capacity(columns * rows);
        for row in 0..rows {
            for column in 0..columns {
                let corners = Self::corner_ids(grid, column, row).map(|id| grid.z[id as usize]);
                centres.push(if corners.iter().all(|value| value.is_finite()) {
                    let (low, high) = min_max(&corners);
                    // lets-plot: the mean of the two middle values.
                    (corners.iter().sum::<f64>() - low - high) / 2.0
                } else {
                    f64::NAN
                });
            }
        }
        Cells { grid, centres }
    }

    /// Counter-clockwise corners of a cell (x and y ascending).
    fn corner_ids(grid: &Grid, column: usize, row: usize) -> [u32; 4] {
        let width = grid.xs.len();
        let node = |column: usize, row: usize| (row * width + column) as u32;
        [
            node(column, row),
            node(column + 1, row),
            node(column + 1, row + 1),
            node(column, row + 1),
        ]
    }

    fn columns(&self) -> usize {
        self.grid.xs.len() - 1
    }

    fn centre_id(&self, cell: usize) -> u32 {
        (self.grid.z.len() + cell) as u32
    }

    /// z of a grid point (node or cell centre).
    fn z_of(&self, id: u32) -> f64 {
        let id = id as usize;
        match id.checked_sub(self.grid.z.len()) {
            Some(cell) => self.centres[cell],
            None => self.grid.z[id],
        }
    }

    /// Position of a grid point.
    fn position_of(&self, id: u32) -> (f64, f64) {
        let width = self.grid.xs.len();
        let id = id as usize;
        match id.checked_sub(self.grid.z.len()) {
            Some(cell) => {
                let (column, row) = (cell % self.columns(), cell / self.columns());
                (
                    (self.grid.xs[column] + self.grid.xs[column + 1]) / 2.0,
                    (self.grid.ys[row] + self.grid.ys[row + 1]) / 2.0,
                )
            }
            None => (self.grid.xs[id % width], self.grid.ys[id / width]),
        }
    }

    /// Position of a vertex.
    fn position(&self, vertex: Vertex, levels: &[f64]) -> (f64, f64) {
        let (low_x, low_y) = self.position_of(vertex.low);
        if vertex.is_point() {
            return (low_x, low_y);
        }
        let (high_x, high_y) = self.position_of(vertex.high);
        let (low_z, high_z) = (self.z_of(vertex.low), self.z_of(vertex.high));
        let ratio = (levels[vertex.level as usize] - low_z) / (high_z - low_z);
        (
            low_x + (high_x - low_x) * ratio,
            low_y + (high_y - low_y) * ratio,
        )
    }

    /// Where the edge between points `a` and `b` crosses a level: a point when one
    /// end lies on it.
    fn crossing(&self, a: u32, b: u32, level_index: u32, level: f64) -> Vertex {
        if self.z_of(a) == level {
            return Vertex::point(a);
        }
        if self.z_of(b) == level {
            return Vertex::point(b);
        }
        Vertex {
            low: a.min(b),
            high: a.max(b),
            level: level_index,
        }
    }

    /// Corner values and the centre of every complete cell, with its index.
    fn complete_cells(&self) -> impl Iterator<Item = (usize, [Corner; 4], Corner)> + '_ {
        let columns = self.columns();
        self.centres
            .iter()
            .enumerate()
            .filter(|(_, centre)| centre.is_finite())
            .map(move |(cell, &centre)| {
                let ids = Self::corner_ids(self.grid, cell % columns, cell / columns);
                let corners = ids.map(|id| Corner {
                    vertex: Vertex::point(id),
                    z: self.grid.z[id as usize],
                });
                let centre = Corner {
                    vertex: Vertex::point(self.centre_id(cell)),
                    z: centre,
                };
                (cell, corners, centre)
            })
    }

    /// Isolines of one level (lets-plot `computeSegments`, `joinSegments`).
    fn isolines(&self, level_index: u32, level: f64, levels: &[f64]) -> Vec<Vec<(f64, f64)>> {
        let mut segments: Vec<(Vertex, Vertex)> = Vec::new();
        for (_, corners, centre) in self.complete_cells() {
            let (low, high) = min_max(&corners.map(|corner| corner.z));
            // lets-plot: flat cells and cells the level does not cross are skipped.
            if low == high || !(level > low && level <= high) {
                continue;
            }
            for side in 0..4 {
                let triangle = [corners[side], corners[(side + 1) % 4], centre];
                if let Some(segment) = self.triangle_segment(&triangle, level_index, level) {
                    segments.push(segment);
                }
            }
        }
        join(&segments)
            .into_iter()
            .filter_map(|path| self.to_points(&path, levels))
            .collect()
    }

    /// lets-plot `intersectionSegment`: the segment where a counter-clockwise
    /// triangle crosses the level, higher values on its right.
    fn triangle_segment(
        &self,
        triangle: &[Corner; 3],
        level_index: u32,
        level: f64,
    ) -> Option<(Vertex, Vertex)> {
        let above = |index: usize| usize::from(triangle[index].z >= level);
        let edge = |from: usize, to: usize| {
            self.crossing(
                triangle[from].vertex.low,
                triangle[to].vertex.low,
                level_index,
                level,
            )
        };
        let (start, end) = match (above(0), above(1), above(2)) {
            (1, 0, 0) => (edge(2, 0), edge(0, 1)),
            (0, 1, 0) => (edge(0, 1), edge(1, 2)),
            (0, 0, 1) => (edge(1, 2), edge(2, 0)),
            (1, 1, 0) => (edge(0, 2), edge(2, 1)),
            (1, 0, 1) => (edge(2, 1), edge(1, 0)),
            (0, 1, 1) => (edge(1, 0), edge(0, 2)),
            _ => return None,
        };
        (start != end).then_some((start, end))
    }

    /// The closed rings of band `band` (between `levels[band - 1]` and
    /// `levels[band]`; open below the first and above the last), one after another.
    fn isoband(&self, band: usize, levels: &[f64]) -> Vec<(f64, f64)> {
        let low = band
            .checked_sub(1)
            .map(|index| (index as u32, levels[index]));
        let high = levels.get(band).map(|&level| (band as u32, level));
        let below_band = |z: f64| low.is_some_and(|(_, level)| z < level);
        let above_band = |z: f64| high.is_some_and(|(_, level)| z >= level);
        let mut edges = Edges::default();
        let mut polygon: Vec<Corner> = Vec::with_capacity(8);
        let mut clipped: Vec<Corner> = Vec::with_capacity(8);
        for (_, corners, centre) in self.complete_cells() {
            let (low_z, high_z) = min_max(&[
                corners[0].z,
                corners[1].z,
                corners[2].z,
                corners[3].z,
                centre.z,
            ]);
            if below_band(high_z) || above_band(low_z) {
                continue;
            }
            if !below_band(low_z) && !above_band(high_z) {
                // The whole cell is in the band: its outline is enough.
                for side in 0..4 {
                    edges.add(corners[side].vertex, corners[(side + 1) % 4].vertex);
                }
                continue;
            }
            for side in 0..4 {
                polygon.clear();
                polygon.extend([corners[side], corners[(side + 1) % 4], centre]);
                if let Some((index, level)) = low {
                    self.clip(&polygon, index, level, true, &mut clipped);
                    std::mem::swap(&mut polygon, &mut clipped);
                }
                if let Some((index, level)) = high {
                    self.clip(&polygon, index, level, false, &mut clipped);
                    std::mem::swap(&mut polygon, &mut clipped);
                }
                polygon.dedup_by(|next, previous| next.vertex == previous.vertex);
                while polygon.len() > 1 && polygon[0].vertex == polygon[polygon.len() - 1].vertex {
                    polygon.pop();
                }
                if polygon.len() < 3 {
                    continue;
                }
                for index in 0..polygon.len() {
                    edges.add(
                        polygon[index].vertex,
                        polygon[(index + 1) % polygon.len()].vertex,
                    );
                }
            }
        }
        let mut points = Vec::new();
        for ring in join(&edges.remaining()) {
            if let Some(ring) = self.to_points(&ring, levels) {
                let ring = drop_straight_vertices(ring);
                if ring.len() >= 4 {
                    points.extend(ring);
                }
            }
        }
        points
    }

    /// Sutherland-Hodgman: the part of `polygon` with z >= level (`keep_above`) or
    /// z < level.
    fn clip(
        &self,
        polygon: &[Corner],
        level_index: u32,
        level: f64,
        keep_above: bool,
        out: &mut Vec<Corner>,
    ) {
        out.clear();
        let inside = |corner: &Corner| (corner.z >= level) == keep_above;
        for index in 0..polygon.len() {
            let current = polygon[index];
            let previous = polygon[(index + polygon.len() - 1) % polygon.len()];
            let crossing = || {
                // The edge lies on the grid edge of a crossing end, else between points.
                let (a, b) = if !previous.vertex.is_point() {
                    (previous.vertex.low, previous.vertex.high)
                } else if !current.vertex.is_point() {
                    (current.vertex.low, current.vertex.high)
                } else {
                    (previous.vertex.low, current.vertex.low)
                };
                Corner {
                    vertex: self.crossing(a, b, level_index, level),
                    z: level,
                }
            };
            if inside(&current) {
                if !inside(&previous) {
                    out.push(crossing());
                }
                out.push(current);
            } else if inside(&previous) {
                out.push(crossing());
            }
        }
    }

    /// Positions of a vertex path, consecutive duplicates dropped; `None` when
    /// fewer than two remain.
    fn to_points(&self, path: &[Vertex], levels: &[f64]) -> Option<Vec<(f64, f64)>> {
        let mut points: Vec<(f64, f64)> = Vec::with_capacity(path.len());
        for &vertex in path {
            let point = self.position(vertex, levels);
            if points.last() != Some(&point) {
                points.push(point);
            }
        }
        // A closed path stays closed despite rounding (lets-plot `convertPaths`).
        if path.len() > 2 && path[0] == path[path.len() - 1] {
            let first = points[0];
            if points.last() != Some(&first) {
                points.push(first);
            }
        }
        (points.len() > 1).then_some(points)
    }
}

/// Directed edges of band pieces; an edge cancels its reverse (shared by two
/// pieces).
#[derive(Default)]
struct Edges {
    counts: AHashMap<(Vertex, Vertex), u32>,
}

impl Edges {
    fn add(&mut self, from: Vertex, to: Vertex) {
        if from == to {
            return;
        }
        if let Some(count) = self.counts.get_mut(&(to, from)) {
            *count -= 1;
            if *count == 0 {
                self.counts.remove(&(to, from));
            }
            return;
        }
        *self.counts.entry((from, to)).or_insert(0) += 1;
    }

    /// The edges left, sorted (rings come out in the same order on every run).
    fn remaining(self) -> Vec<(Vertex, Vertex)> {
        let mut edges: Vec<(Vertex, Vertex)> = self
            .counts
            .into_iter()
            .flat_map(|(edge, count)| std::iter::repeat_n(edge, count as usize))
            .collect();
        edges.sort_unstable();
        edges
    }
}

/// Join directed segments into paths: open paths from the starts nothing ends
/// at, then closed ones (their first vertex repeated at the end).
fn join(segments: &[(Vertex, Vertex)]) -> Vec<Vec<Vertex>> {
    let mut outgoing: AHashMap<Vertex, Vec<usize>> = AHashMap::with_capacity(segments.len());
    let mut incoming: AHashMap<Vertex, usize> = AHashMap::with_capacity(segments.len());
    for (index, &(start, end)) in segments.iter().enumerate() {
        outgoing.entry(start).or_default().push(index);
        *incoming.entry(end).or_default() += 1;
    }
    let mut used = vec![false; segments.len()];
    let mut paths = Vec::new();
    let mut walk = |first: usize, used: &mut Vec<bool>| {
        let mut path = vec![segments[first].0];
        let mut current = first;
        loop {
            used[current] = true;
            let end = segments[current].1;
            path.push(end);
            let next = outgoing
                .get(&end)
                .and_then(|candidates| candidates.iter().copied().find(|&next| !used[next]));
            match next {
                Some(next) => current = next,
                None => break,
            }
        }
        paths.push(path);
    };
    for index in 0..segments.len() {
        if !used[index] && !incoming.contains_key(&segments[index].0) {
            walk(index, &mut used);
        }
    }
    for index in 0..segments.len() {
        if !used[index] {
            walk(index, &mut used);
        }
    }
    paths
}

/// Drop ring vertices in the middle of a horizontal or vertical run (grid nodes
/// along the grid border or a hole). `ring` repeats its first vertex at the end.
fn drop_straight_vertices(ring: Vec<(f64, f64)>) -> Vec<(f64, f64)> {
    let mut open: Vec<(f64, f64)> = ring[..ring.len() - 1].to_vec();
    let mut index = 0;
    let mut since_removal = 0;
    // Walk around (more than once if needed) until a full lap removes nothing.
    while open.len() >= 3 && since_removal <= open.len() {
        let count = open.len();
        index %= count;
        let (previous, current, next) = (
            open[(index + count - 1) % count],
            open[index],
            open[(index + 1) % count],
        );
        if (previous.0 == current.0 && current.0 == next.0)
            || (previous.1 == current.1 && current.1 == next.1)
        {
            open.remove(index);
            since_removal = 0;
            index = index.saturating_sub(1);
        } else {
            index += 1;
            since_removal += 1;
        }
    }
    if let Some(&first) = open.first() {
        open.push(first);
    }
    open
}

fn min_max(values: &[f64]) -> (f64, f64) {
    values
        .iter()
        .fold((f64::INFINITY, f64::NEG_INFINITY), |(low, high), &value| {
            (low.min(value), high.max(value))
        })
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Nodes of z(x, y) on an n x n grid over [-1, 1]^2.
    fn surface(n: usize, z: impl Fn(f64, f64) -> f64) -> Vec<(f64, f64, f64)> {
        let step = 2.0 / (n - 1) as f64;
        let mut nodes = Vec::new();
        for row in 0..n {
            for column in 0..n {
                let (x, y) = (-1.0 + column as f64 * step, -1.0 + row as f64 * step);
                nodes.push((x, y, z(x, y)));
            }
        }
        nodes
    }

    fn options(count: usize) -> LevelOptions {
        LevelOptions { count, width: None }
    }

    /// Signed area of rings stored one after another (each closed).
    fn rings(points: &[(f64, f64)]) -> Vec<Vec<(f64, f64)>> {
        let mut rings = Vec::new();
        let mut current: Vec<(f64, f64)> = Vec::new();
        for &point in points {
            current.push(point);
            if current.len() > 1 && point == current[0] {
                rings.push(std::mem::take(&mut current));
            }
        }
        assert!(current.is_empty(), "every ring is closed");
        rings
    }

    fn signed_area(ring: &[(f64, f64)]) -> f64 {
        ring.windows(2)
            .map(|pair| pair[0].0 * pair[1].1 - pair[1].0 * pair[0].1)
            .sum::<f64>()
            / 2.0
    }

    #[test]
    fn levels_follow_lets_plot() {
        let levels = options(4).levels((0.0, 8.0)).unwrap();
        assert_eq!(levels, vec![1.0, 3.0, 5.0, 7.0]);
        let by_width = LevelOptions {
            count: 10,
            width: Some(3.0),
        };
        assert_eq!(by_width.levels((0.0, 8.0)).unwrap(), vec![1.5, 4.5, 7.5]);
        assert!(options(10).levels((2.0, 2.0)).is_none());
    }

    #[test]
    fn cone_isolines_are_circles() {
        let grid = Grid::from_nodes(&surface(41, |x, y| (x * x + y * y).sqrt()))
            .unwrap()
            .unwrap();
        let level = 0.5;
        let cells = Cells::new(&grid);
        let lines = cells.isolines(0, level, &[level]);
        assert_eq!(lines.len(), 1);
        let line = &lines[0];
        assert_eq!(line.first(), line.last(), "a closed ring");
        for &(x, y) in line {
            assert!(((x * x + y * y).sqrt() - level).abs() < 0.01, "({x}, {y})");
        }
        // Higher values on the right: counter-clockwise around the low centre.
        assert!(signed_area(line) > 0.0);
    }

    #[test]
    fn saddle_cells_follow_the_centre_value() {
        // One cell, z = x * y: corners -1, 1, -1, 1; middle values -1 and 1 -> centre 0.
        let nodes = vec![
            (0.0, 0.0, 1.0),
            (1.0, 0.0, -1.0),
            (1.0, 1.0, 1.0),
            (0.0, 1.0, -1.0),
        ];
        let grid = Grid::from_nodes(&nodes).unwrap().unwrap();
        let cells = Cells::new(&grid);
        assert_eq!(cells.centres, vec![0.0]);
        // Above the centre: two separate corner caps, as ggplot2's mean rule.
        assert_eq!(cells.isolines(0, 0.5, &[0.5]).len(), 2);
        assert_eq!(cells.isolines(0, -0.5, &[-0.5]).len(), 2);
    }

    #[test]
    fn unsorted_rows_give_the_same_lines() {
        let nodes = surface(15, |x, y| x * x - y);
        let mut shuffled = nodes.clone();
        shuffled.reverse();
        shuffled.swap(3, 100);
        let lines = |nodes: &[(f64, f64, f64)]| {
            Grid::from_nodes(nodes)
                .unwrap()
                .unwrap()
                .lines(&options(5))
                .into_iter()
                .map(|piece| (piece.level, piece.points))
                .collect::<Vec<_>>()
        };
        assert_eq!(lines(&nodes), lines(&shuffled));
    }

    #[test]
    fn zero_is_a_value_and_missing_nodes_leave_holes() {
        // z = 0 on the left half: the 0.5 line still runs through the middle.
        let nodes = surface(11, |x, _| if x < 0.0 { 0.0 } else { 1.0 });
        let grid = Grid::from_nodes(&nodes).unwrap().unwrap();
        let lines = grid.lines(&options(1));
        assert_eq!(lines.len(), 1);
        assert!(
            lines[0]
                .points
                .iter()
                .all(|&(x, _)| (-0.2..0.0).contains(&x))
        );
        // Drop the centre node: the band around it gets a hole of four cells.
        let mut holed = surface(11, |_, _| 1.0);
        holed.retain(|&(x, y, _)| !(x.abs() < 1e-9 && y.abs() < 1e-9));
        let mut bands_input = holed.clone();
        bands_input[0].2 = 0.0; // a z range, so there are levels
        let grid = Grid::from_nodes(&bands_input).unwrap().unwrap();
        let bands = grid.bands(&options(1), true);
        let total: f64 = bands
            .iter()
            .flat_map(|band| rings(&band.points))
            .map(|ring| signed_area(&ring))
            .sum();
        // The square is 4; the hole is 4 cells of 0.2 x 0.2.
        assert!((total - (4.0 - 0.16)).abs() < 1e-9, "{total}");
    }

    #[test]
    fn band_areas_sum_to_the_grid() {
        let grid = Grid::from_nodes(&surface(31, |x, y| (3.0 * x).sin() * (2.0 * y).cos()))
            .unwrap()
            .unwrap();
        let bands = grid.bands(&options(7), true);
        assert_eq!(bands.len(), 8);
        let mut total = 0.0;
        for band in &bands {
            let area: f64 = rings(&band.points)
                .iter()
                .map(|ring| signed_area(ring))
                .sum();
            assert!(area > 0.0, "band {} has area {area}", band.level);
            total += area;
        }
        assert!((total - 4.0).abs() < 1e-9, "{total}");
    }

    #[test]
    fn cone_band_area_matches_the_disc() {
        let grid = Grid::from_nodes(&surface(201, |x, y| (x * x + y * y).sqrt()))
            .unwrap()
            .unwrap();
        // Levels 0.25, 0.75, 1.25 ...: the lowest band is the disc of radius 0.25.
        let bands = grid.bands(
            &LevelOptions {
                count: 1,
                width: Some(0.5),
            },
            true,
        );
        let disc: f64 = rings(&bands[0].points)
            .iter()
            .map(|ring| signed_area(ring))
            .sum();
        let expected = std::f64::consts::PI * 0.25 * 0.25;
        assert!((disc - expected).abs() / expected < 0.01, "{disc}");
        // The next band has the disc as a clockwise hole.
        let next = rings(&bands[1].points);
        assert!(next.iter().any(|ring| signed_area(ring) < 0.0));
    }

    #[test]
    fn straight_runs_lose_their_middle_vertices() {
        let ring = vec![
            (0.0, 0.0),
            (1.0, 0.0),
            (2.0, 0.0),
            (2.0, 1.0),
            (2.0, 2.0),
            (0.0, 2.0),
            (0.0, 1.0),
            (0.0, 0.0),
        ];
        assert_eq!(
            drop_straight_vertices(ring),
            vec![(0.0, 0.0), (2.0, 0.0), (2.0, 2.0), (0.0, 2.0), (0.0, 0.0)]
        );
    }

    #[test]
    fn huge_scattered_data_is_refused_with_advice() {
        let nodes: Vec<(f64, f64, f64)> = (0..5000)
            .map(|index| (index as f64, (index * 7919 % 5000) as f64, 1.0))
            .collect();
        let error = Grid::from_nodes(&nodes).err().unwrap().to_string();
        assert!(error.contains("geom_density2d"), "{error}");
    }
}

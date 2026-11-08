//! Streamlines of a 2D vector field on a regular grid: a line-by-line port of
//! matplotlib's `streamplot` (matplotlib 3.10, `lib/matplotlib/streamplot.py`).
//!
//! The field is integrated with matplotlib's adaptive second-order Runge-Kutta
//! (Heun) scheme from seed points that spiral in from the boundary of a
//! `30 * density` occupancy mask; each mask cell is crossed by at most one line.
//! Non-finite field values stop a line, as matplotlib's masked arrays do.
//! Arrows sit at half the arc length of each line, as matplotlib places them.
//!
//! What it does not do: draw anything, interpolate scattered vectors onto a grid
//! (that is the caller's job, e.g. scVelo's `compute_velocity_on_grid`), or vary
//! color and line width along a line.

use crate::{Error, Result};

/// Which way lines are integrated from their seed points.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum IntegrationDirection {
    /// Along the field.
    Forward,
    /// Against the field.
    Backward,
    /// Both ways, each with half of `maxlength`.
    Both,
}

/// The options of matplotlib's `streamplot` that shape the lines.
#[derive(Debug, Clone, PartialEq)]
pub struct StreamplotOptions {
    /// Closeness of lines per axis: the mask has `30 * density` cells.
    pub density: (f64, f64),
    /// Lines shorter than this (in axes coordinates) are dropped.
    pub minlength: f64,
    /// Longest line (in axes coordinates).
    pub maxlength: f64,
    /// Which way to integrate from each seed.
    pub direction: IntegrationDirection,
    /// Stop a line when it enters a cell another line crossed.
    pub broken_streamlines: bool,
    /// Seeds in data coordinates; `None` spirals in from the boundary.
    pub start_points: Option<Vec<[f64; 2]>>,
}

impl Default for StreamplotOptions {
    fn default() -> Self {
        StreamplotOptions {
            density: (1.0, 1.0),
            minlength: 0.1,
            maxlength: 4.0,
            direction: IntegrationDirection::Both,
            broken_streamlines: true,
            start_points: None,
        }
    }
}

/// One streamline in data coordinates.
#[derive(Debug, Clone, PartialEq)]
pub struct Streamline {
    /// x of each vertex.
    pub x: Vec<f64>,
    /// y of each vertex.
    pub y: Vec<f64>,
}

/// An arrow from `(x, y)` to `(x_end, y_end)` in data coordinates.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct StreamArrow {
    /// Tail x.
    pub x: f64,
    /// Tail y.
    pub y: f64,
    /// Head x.
    pub x_end: f64,
    /// Head y.
    pub y_end: f64,
}

impl Streamline {
    /// matplotlib's arrow: from the first vertex at or past half the arc length
    /// to the middle of the segment that starts there. `None` for a single vertex.
    pub fn arrow(&self) -> Option<StreamArrow> {
        let n = self.x.len();
        if n < 2 {
            return None;
        }
        let mut cumulative = Vec::with_capacity(n - 1);
        let mut total = 0.0;
        for index in 1..n {
            total += (self.x[index] - self.x[index - 1]).hypot(self.y[index] - self.y[index - 1]);
            cumulative.push(total);
        }
        let half = total / 2.0;
        // np.searchsorted(side="left"): first index with value >= half.
        let start = cumulative.partition_point(|&value| value < half).min(n - 2);
        Some(StreamArrow {
            x: self.x[start],
            y: self.y[start],
            x_end: (self.x[start] + self.x[start + 1]) / 2.0,
            y_end: (self.y[start] + self.y[start + 1]) / 2.0,
        })
    }
}

/// Integrates streamlines of the field `(u, v)` given on the regular grid
/// `x` (columns) by `y` (rows); `u[row][column]` is the x component at
/// `(x[column], y[row])`. Non-finite components are treated as missing.
pub fn streamplot(
    x: &[f64],
    y: &[f64],
    u: &[Vec<f64>],
    v: &[Vec<f64>],
    options: &StreamplotOptions,
) -> Result<Vec<Streamline>> {
    let grid = Grid::new(x, y)?;
    let u = Field::new(u, &grid, "u")?;
    let v = Field::new(v, &grid, "v")?;
    let mut mask = StreamMask::new(options.density)?;
    let domain = DomainMap::new(&grid, &mask);
    for (name, value) in [
        ("minlength", options.minlength),
        ("maxlength", options.maxlength),
    ] {
        if !value.is_finite() {
            return Err(Error::spec(
                name,
                format!("must be a finite number, got {value}"),
            ));
        }
    }
    let maxlength = match options.direction {
        IntegrationDirection::Both => options.maxlength / 2.0,
        _ => options.maxlength,
    };
    let integrator = Integrator::new(
        &grid,
        &domain,
        (mask.nx, mask.ny),
        &u,
        &v,
        options,
        maxlength,
    );

    let mut trajectories = Vec::new();
    match &options.start_points {
        None => {
            for (mask_x, mask_y) in starting_points(mask.nx, mask.ny) {
                if !mask.is_filled(mask_x, mask_y) {
                    let (grid_x, grid_y) = domain.mask_to_grid(mask_x, mask_y);
                    if let Some(trajectory) = integrator.integrate(&mut mask, grid_x, grid_y) {
                        trajectories.push(trajectory);
                    }
                }
            }
        }
        Some(points) => {
            for (index, &[start_x, start_y]) in points.iter().enumerate() {
                let inside = grid.x_origin <= start_x
                    && start_x <= grid.x_origin + grid.width
                    && grid.y_origin <= start_y
                    && start_y <= grid.y_origin + grid.height;
                if !inside {
                    return Err(Error::spec(
                        format!("start_points[{index}]"),
                        format!(
                            "starting point ({start_x}, {start_y}) is outside the grid \
                             x in [{}, {}], y in [{}, {}]; pass points inside the grid",
                            grid.x_origin,
                            grid.x_origin + grid.width,
                            grid.y_origin,
                            grid.y_origin + grid.height
                        ),
                    ));
                }
            }
            for &[start_x, start_y] in points {
                let (grid_x, grid_y) =
                    domain.data_to_grid(start_x - grid.x_origin, start_y - grid.y_origin);
                let grid_x = grid_x.clamp(0.0, (grid.nx - 1) as f64);
                let grid_y = grid_y.clamp(0.0, (grid.ny - 1) as f64);
                if let Some(trajectory) = integrator.integrate(&mut mask, grid_x, grid_y) {
                    trajectories.push(trajectory);
                }
            }
        }
    }

    Ok(trajectories
        .into_iter()
        .map(|trajectory| {
            let (x, y) = trajectory
                .into_iter()
                .map(|(grid_x, grid_y)| {
                    let (data_x, data_y) = domain.grid_to_data(grid_x, grid_y);
                    (data_x + grid.x_origin, data_y + grid.y_origin)
                })
                .unzip();
            Streamline { x, y }
        })
        .collect())
}

/// matplotlib's `Grid`: a regular, strictly increasing grid.
struct Grid {
    nx: usize,
    ny: usize,
    dx: f64,
    dy: f64,
    x_origin: f64,
    y_origin: f64,
    width: f64,
    height: f64,
}

impl Grid {
    fn new(x: &[f64], y: &[f64]) -> Result<Self> {
        for (name, values) in [("x", x), ("y", y)] {
            if values.len() < 2 {
                return Err(Error::spec(
                    name,
                    format!(
                        "needs at least 2 grid values, got {}; pass the grid coordinates \
                         of every column (x) or row (y)",
                        values.len()
                    ),
                ));
            }
            if let Some(value) = values.iter().find(|value| !value.is_finite()) {
                return Err(Error::spec(
                    name,
                    format!("grid values must be finite, got {value}"),
                ));
            }
            if values.windows(2).any(|pair| pair[1] - pair[0] <= 0.0) {
                return Err(Error::spec(
                    name,
                    format!("'{name}' must be strictly increasing"),
                ));
            }
            let n = values.len();
            let step = (values[n - 1] - values[0]) / (n - 1) as f64;
            // np.allclose(np.diff(values), step): rtol 1e-5, atol 1e-8.
            if values
                .windows(2)
                .any(|pair| ((pair[1] - pair[0]) - step).abs() > 1e-8 + 1e-5 * step.abs())
            {
                return Err(Error::spec(
                    name,
                    format!("'{name}' values must be equally spaced"),
                ));
            }
        }
        Ok(Grid {
            nx: x.len(),
            ny: y.len(),
            dx: x[1] - x[0],
            dy: y[1] - y[0],
            x_origin: x[0],
            y_origin: y[0],
            width: x[x.len() - 1] - x[0],
            height: y[y.len() - 1] - y[0],
        })
    }

    fn within_grid(&self, x: f64, y: f64) -> bool {
        0.0 <= x && x <= (self.nx - 1) as f64 && 0.0 <= y && y <= (self.ny - 1) as f64
    }
}

/// A field on the grid, row-major; non-finite values are missing (masked).
struct Field {
    values: Vec<f64>,
    nx: usize,
    ny: usize,
}

impl Field {
    fn new(rows: &[Vec<f64>], grid: &Grid, name: &str) -> Result<Self> {
        let shape_error = || {
            Error::spec(
                name,
                format!(
                    "'{name}' must have one row per y value and one column per x value: \
                     expected shape ({}, {}), got {} rows of lengths {:?}",
                    grid.ny,
                    grid.nx,
                    rows.len(),
                    rows.iter().map(Vec::len).take(5).collect::<Vec<_>>()
                ),
            )
        };
        if rows.len() != grid.ny || rows.iter().any(|row| row.len() != grid.nx) {
            return Err(shape_error());
        }
        Ok(Field {
            values: rows.iter().flatten().copied().collect(),
            nx: grid.nx,
            ny: grid.ny,
        })
    }

    fn map(&self, function: impl Fn(f64) -> f64) -> Field {
        Field {
            values: self.values.iter().map(|&value| function(value)).collect(),
            nx: self.nx,
            ny: self.ny,
        }
    }

    /// matplotlib's `interpgrid` for a scalar point: `None` when a corner is missing.
    fn interpolate(&self, x: f64, y: f64) -> Option<f64> {
        let column = x as usize;
        let row = y as usize;
        let column_next = if column == self.nx - 1 {
            column
        } else {
            column + 1
        };
        let row_next = if row == self.ny - 1 { row } else { row + 1 };
        let at = |row: usize, column: usize| self.values[row * self.nx + column];
        let a00 = at(row, column);
        let a01 = at(row, column_next);
        let a10 = at(row_next, column);
        let a11 = at(row_next, column_next);
        if !(a00.is_finite() && a01.is_finite() && a10.is_finite() && a11.is_finite()) {
            return None;
        }
        let xt = x - column as f64;
        let yt = y - row as f64;
        let a0 = a00 * (1.0 - xt) + a01 * xt;
        let a1 = a10 * (1.0 - xt) + a11 * xt;
        Some(a0 * (1.0 - yt) + a1 * yt)
    }
}

/// matplotlib's `StreamMask`: cells crossed by a line so far.
struct StreamMask {
    nx: usize,
    ny: usize,
    filled: Vec<bool>,
    /// Cell the current line is in. Kept across lines, as matplotlib does.
    current: Option<(usize, usize)>,
    /// Cells filled by the current line, to undo a line that is too short.
    trajectory: Vec<(usize, usize)>,
}

/// The line entered a cell another line crossed, or left the grid.
struct InvalidIndex;

impl StreamMask {
    fn new(density: (f64, f64)) -> Result<Self> {
        let cells = |value: f64, name: &str| -> Result<usize> {
            // A mask of one cell has no spacing (matplotlib divides by zero).
            if !value.is_finite() || 30.0 * value < 2.0 {
                return Err(Error::spec(
                    "density",
                    format!(
                        "{name} density must be a finite number >= 1/15 \
                         (the mask has 30 * density cells), got {value}"
                    ),
                ));
            }
            Ok((30.0 * value) as usize)
        };
        let nx = cells(density.0, "x")?;
        let ny = cells(density.1, "y")?;
        // Guards the mask allocation (one byte per cell).
        const MAX_CELLS: usize = 100_000_000;
        if nx.saturating_mul(ny) > MAX_CELLS {
            return Err(Error::spec(
                "density",
                format!(
                    "density ({}, {}) is too large: the mask would have {nx} x {ny} cells \
                     (at most {MAX_CELLS}); use a density of a few hundred at most",
                    density.0, density.1
                ),
            ));
        }
        Ok(StreamMask {
            nx,
            ny,
            filled: vec![false; nx * ny],
            current: None,
            trajectory: Vec::new(),
        })
    }

    fn is_filled(&self, x: usize, y: usize) -> bool {
        self.filled[y * self.nx + x]
    }

    fn start_trajectory(
        &mut self,
        x: usize,
        y: usize,
        broken: bool,
    ) -> std::result::Result<(), InvalidIndex> {
        self.trajectory.clear();
        self.update_trajectory(x, y, broken)
    }

    fn undo_trajectory(&mut self) {
        for &(x, y) in &self.trajectory {
            self.filled[y * self.nx + x] = false;
        }
    }

    fn update_trajectory(
        &mut self,
        x: usize,
        y: usize,
        broken: bool,
    ) -> std::result::Result<(), InvalidIndex> {
        if self.current != Some((x, y)) {
            if !self.is_filled(x, y) {
                self.trajectory.push((x, y));
                self.filled[y * self.nx + x] = true;
                self.current = Some((x, y));
            } else if broken {
                return Err(InvalidIndex);
            }
        }
        Ok(())
    }
}

/// matplotlib's `DomainMap`: conversions between data, grid and mask coordinates.
struct DomainMap {
    x_grid_to_mask: f64,
    y_grid_to_mask: f64,
    x_mask_to_grid: f64,
    y_mask_to_grid: f64,
    x_data_to_grid: f64,
    y_data_to_grid: f64,
}

impl DomainMap {
    fn new(grid: &Grid, mask: &StreamMask) -> Self {
        let x_grid_to_mask = (mask.nx - 1) as f64 / (grid.nx - 1) as f64;
        let y_grid_to_mask = (mask.ny - 1) as f64 / (grid.ny - 1) as f64;
        DomainMap {
            x_grid_to_mask,
            y_grid_to_mask,
            x_mask_to_grid: 1.0 / x_grid_to_mask,
            y_mask_to_grid: 1.0 / y_grid_to_mask,
            x_data_to_grid: 1.0 / grid.dx,
            y_data_to_grid: 1.0 / grid.dy,
        }
    }

    /// Nearest mask cell, rounding half to even as Python's `round`.
    fn grid_to_mask(&self, x: f64, y: f64) -> (usize, usize) {
        (
            (x * self.x_grid_to_mask).round_ties_even() as usize,
            (y * self.y_grid_to_mask).round_ties_even() as usize,
        )
    }

    fn mask_to_grid(&self, x: usize, y: usize) -> (f64, f64) {
        (
            x as f64 * self.x_mask_to_grid,
            y as f64 * self.y_mask_to_grid,
        )
    }

    fn data_to_grid(&self, x: f64, y: f64) -> (f64, f64) {
        (x * self.x_data_to_grid, y * self.y_data_to_grid)
    }

    fn grid_to_data(&self, x: f64, y: f64) -> (f64, f64) {
        (x / self.x_data_to_grid, y / self.y_data_to_grid)
    }
}

/// Why a field evaluation failed (matplotlib's `OutOfBounds` and
/// `TerminateTrajectory` exceptions).
enum Stop {
    OutOfBounds,
    Terminate,
}

/// matplotlib's `_get_integrator`.
struct Integrator<'a> {
    grid: &'a Grid,
    domain: &'a DomainMap,
    mask_nx: usize,
    mask_ny: usize,
    u: Field,
    v: Field,
    speed: Field,
    minlength: f64,
    maxlength: f64,
    direction: IntegrationDirection,
    broken: bool,
}

impl<'a> Integrator<'a> {
    fn new(
        grid: &'a Grid,
        domain: &'a DomainMap,
        (mask_nx, mask_ny): (usize, usize),
        u: &Field,
        v: &Field,
        options: &StreamplotOptions,
        maxlength: f64,
    ) -> Self {
        let u = u.map(|value| value * domain.x_data_to_grid);
        let v = v.map(|value| value * domain.y_data_to_grid);
        let x_scale = (grid.nx - 1) as f64;
        let y_scale = (grid.ny - 1) as f64;
        let speed = Field {
            values: u
                .values
                .iter()
                .zip(&v.values)
                .map(|(&u, &v)| {
                    let u_axes = u / x_scale;
                    let v_axes = v / y_scale;
                    (u_axes * u_axes + v_axes * v_axes).sqrt()
                })
                .collect(),
            nx: u.nx,
            ny: u.ny,
        };
        Integrator {
            grid,
            domain,
            mask_nx,
            mask_ny,
            u,
            v,
            speed,
            minlength: options.minlength,
            maxlength,
            direction: options.direction,
            broken: options.broken_streamlines,
        }
    }

    fn forward_time(&self, x: f64, y: f64) -> std::result::Result<(f64, f64), Stop> {
        if !self.grid.within_grid(x, y) {
            return Err(Stop::OutOfBounds);
        }
        let ds_dt = self.speed.interpolate(x, y).ok_or(Stop::Terminate)?;
        if ds_dt == 0.0 {
            return Err(Stop::Terminate);
        }
        let dt_ds = 1.0 / ds_dt;
        let ui = self.u.interpolate(x, y).ok_or(Stop::Terminate)?;
        let vi = self.v.interpolate(x, y).ok_or(Stop::Terminate)?;
        Ok((ui * dt_ds, vi * dt_ds))
    }

    fn time(&self, backward: bool, x: f64, y: f64) -> std::result::Result<(f64, f64), Stop> {
        let (dx, dy) = self.forward_time(x, y)?;
        Ok(if backward { (-dx, -dy) } else { (dx, dy) })
    }

    fn integrate(&self, mask: &mut StreamMask, x0: f64, y0: f64) -> Option<Vec<(f64, f64)>> {
        let (mask_x, mask_y) = self.domain.grid_to_mask(x0, y0);
        mask.start_trajectory(mask_x, mask_y, self.broken).ok()?;
        let mut total = 0.0;
        let mut trajectory = Vec::new();
        if matches!(
            self.direction,
            IntegrationDirection::Both | IntegrationDirection::Backward
        ) {
            let (length, mut points) = self.integrate_rk12(mask, x0, y0, true);
            total += length;
            points.reverse();
            trajectory.extend(points);
        }
        if matches!(
            self.direction,
            IntegrationDirection::Both | IntegrationDirection::Forward
        ) {
            mask.current = Some(self.domain.grid_to_mask(x0, y0));
            let (length, points) = self.integrate_rk12(mask, x0, y0, false);
            total += length;
            trajectory.extend(points.into_iter().skip(1));
        }
        if total > self.minlength {
            Some(trajectory)
        } else {
            mask.undo_trajectory();
            None
        }
    }

    /// matplotlib's `_integrate_rk12`: Heun's method with an adaptive step.
    fn integrate_rk12(
        &self,
        mask: &mut StreamMask,
        x0: f64,
        y0: f64,
        backward: bool,
    ) -> (f64, Vec<(f64, f64)>) {
        const MAX_ERROR: f64 = 0.003;
        let max_step = (1.0 / self.mask_nx as f64)
            .min(1.0 / self.mask_ny as f64)
            .min(0.1);
        let mut step = max_step;
        let mut total = 0.0;
        let (mut xi, mut yi) = (x0, y0);
        let mut trajectory = Vec::new();
        let x_scale = (self.grid.nx - 1) as f64;
        let y_scale = (self.grid.ny - 1) as f64;
        loop {
            let slopes = if self.grid.within_grid(xi, yi) {
                trajectory.push((xi, yi));
                self.time(backward, xi, yi).and_then(|(k1x, k1y)| {
                    let (k2x, k2y) = self.time(backward, xi + step * k1x, yi + step * k1y)?;
                    Ok((k1x, k1y, k2x, k2y))
                })
            } else {
                Err(Stop::OutOfBounds)
            };
            let (k1x, k1y, k2x, k2y) = match slopes {
                Ok(slopes) => slopes,
                Err(Stop::OutOfBounds) => {
                    if !trajectory.is_empty() {
                        total += self.euler_step(&mut trajectory, backward);
                    }
                    break;
                }
                Err(Stop::Terminate) => break,
            };
            let dx1 = step * k1x;
            let dy1 = step * k1y;
            let dx2 = step * 0.5 * (k1x + k2x);
            let dy2 = step * 0.5 * (k1y + k2y);
            let error = ((dx2 - dx1) / x_scale).hypot((dy2 - dy1) / y_scale);
            if error < MAX_ERROR {
                xi += dx2;
                yi += dy2;
                if !self.grid.within_grid(xi, yi) {
                    break;
                }
                let (mask_x, mask_y) = self.domain.grid_to_mask(xi, yi);
                if mask.update_trajectory(mask_x, mask_y, self.broken).is_err() {
                    break;
                }
                if total + step > self.maxlength {
                    break;
                }
                total += step;
            }
            step = if error == 0.0 {
                max_step
            } else {
                max_step.min(0.85 * step * (MAX_ERROR / error).powf(0.5))
            };
        }
        (total, trajectory)
    }

    /// matplotlib's `_euler_step`: extends a line to the grid boundary.
    fn euler_step(&self, trajectory: &mut Vec<(f64, f64)>, backward: bool) -> f64 {
        let Some(&(xi, yi)) = trajectory.last() else {
            return 0.0;
        };
        // The last point was evaluated before, so this cannot fail; stop if it does.
        let Ok((cx, cy)) = self.time(backward, xi, yi) else {
            return 0.0;
        };
        let x_last = (self.grid.nx - 1) as f64;
        let y_last = (self.grid.ny - 1) as f64;
        let step_x = if cx == 0.0 {
            f64::INFINITY
        } else if cx < 0.0 {
            xi / -cx
        } else {
            (x_last - xi) / cx
        };
        let step_y = if cy == 0.0 {
            f64::INFINITY
        } else if cy < 0.0 {
            yi / -cy
        } else {
            (y_last - yi) / cy
        };
        let step = step_x.min(step_y);
        trajectory.push((xi + cx * step, yi + cy * step));
        step
    }
}

/// matplotlib's `_gen_starting_points`: mask cells spiralling in from a corner.
fn starting_points(nx: usize, ny: usize) -> Vec<(usize, usize)> {
    let (mut x_first, mut y_first) = (0i64, 1i64);
    let (mut x_last, mut y_last) = (nx as i64 - 1, ny as i64 - 1);
    let (mut x, mut y) = (0i64, 0i64);
    let mut direction = 0; // right, up, left, down
    let mut points = Vec::with_capacity(nx * ny);
    for _ in 0..nx * ny {
        points.push((x as usize, y as usize));
        match direction {
            0 => {
                x += 1;
                if x >= x_last {
                    x_last -= 1;
                    direction = 1;
                }
            }
            1 => {
                y += 1;
                if y >= y_last {
                    y_last -= 1;
                    direction = 2;
                }
            }
            2 => {
                x -= 1;
                if x <= x_first {
                    x_first += 1;
                    direction = 3;
                }
            }
            _ => {
                y -= 1;
                if y <= y_first {
                    y_first += 1;
                    direction = 0;
                }
            }
        }
    }
    points
}

#[cfg(test)]
mod tests {
    use super::*;

    fn uniform(nx: usize, ny: usize, value: f64) -> Vec<Vec<f64>> {
        vec![vec![value; nx]; ny]
    }

    #[test]
    fn spiral_visits_every_cell_once_starting_on_the_boundary() {
        let points = starting_points(4, 3);
        assert_eq!(&points[..4], &[(0, 0), (1, 0), (2, 0), (3, 0)]);
        let mut sorted = points.clone();
        sorted.sort_unstable();
        sorted.dedup();
        assert_eq!(sorted.len(), 12);
    }

    #[test]
    fn uniform_flow_gives_straight_lines_with_arrows_along_it() {
        let x = [0.0, 1.0, 2.0, 3.0];
        let y = [0.0, 1.0, 2.0];
        let lines = streamplot(
            &x,
            &y,
            &uniform(4, 3, 1.0),
            &uniform(4, 3, 0.0),
            &StreamplotOptions::default(),
        )
        .expect("valid field");
        assert!(!lines.is_empty());
        for line in &lines {
            assert!(line.y.iter().all(|&value| value == line.y[0]));
            let arrow = line.arrow().expect("line has two vertices");
            assert!(arrow.x_end > arrow.x);
        }
    }

    #[test]
    fn rounding_is_half_to_even_as_python() {
        let grid = Grid::new(&[0.0, 1.0, 2.0], &[0.0, 1.0, 2.0]).expect("grid");
        let mask = StreamMask::new((0.1, 0.1)).expect("mask of 3 cells");
        let domain = DomainMap::new(&grid, &mask);
        assert_eq!(domain.grid_to_mask(0.5, 1.5), (0, 2));
    }

    #[test]
    fn invalid_inputs_name_the_argument() {
        let options = StreamplotOptions::default();
        let field = uniform(3, 2, 1.0);
        let error = streamplot(&[0.0, 1.0, 3.0], &[0.0, 1.0], &field, &field, &options)
            .expect_err("uneven grid");
        assert!(error.to_string().contains("at 'x'"), "{error}");
        let error = streamplot(&[0.0, 1.0, 2.0], &[0.0, 1.0], &field[..1], &field, &options)
            .expect_err("short u");
        assert!(error.to_string().contains("at 'u'"), "{error}");
        let small = StreamplotOptions {
            density: (1.0, 0.0),
            ..StreamplotOptions::default()
        };
        let error = streamplot(&[0.0, 1.0, 2.0], &[0.0, 1.0], &field, &field, &small)
            .expect_err("density 0");
        assert!(error.to_string().contains("y density"), "{error}");
        let huge = StreamplotOptions {
            density: (1e6, 1e300),
            ..StreamplotOptions::default()
        };
        let error = streamplot(&[0.0, 1.0, 2.0], &[0.0, 1.0], &field, &field, &huge)
            .expect_err("density too large");
        assert!(error.to_string().contains("too large"), "{error}");
    }
}

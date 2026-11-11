//! `coord_3d`: a static 3D view of x, y and z (tier 1: no rotation, no surfaces).
//!
//! Kept apart from the 2D [`crate::coord::Coord`] trait: a 3D coord maps three
//! scaled positions to a pixel and a depth. Layers are drawn by their own 2D
//! geoms on projected positions, rows ordered back to front (painter's
//! algorithm), so every backend draws ordinary scene nodes. The axis box (back
//! panes, pane grid, ticks and labels on three edges) is in
//! [`crate::layout::box3d`].
//!
//! Neither lets-plot nor ggplot2 has 3D; the view follows matplotlib `mplot3d`:
//! azimuth and elevation in degrees (default -60 and 30), a 4:4:3 box, an
//! orthographic projection unless `perspective=True`. See deviations.md.
//!
//! `geom_surface` (`crate::geom::surface`) sorts its own cells.
//!
//! What it does not do: rotation in HTML, depth sorting across layers
//! (layers stack in layer order), splitting a path to sort its segments.

use crate::data::{Column, ContinuousColumn, LayerData};
use crate::scale::{ContinuousPositionScale, PositionScale};
use crate::scene::Rect;
use crate::spec::{Aesthetic, CoordSpec};
use crate::style::Styles;
use crate::{Error, Result};

/// Box side lengths relative to x (matplotlib's default `box_aspect` 4:4:3).
const BOX_ASPECT: [f64; 3] = [1.0, 1.0, 0.75];
/// Camera distance from the box centre, in box widths, under `perspective=True`.
const CAMERA_DISTANCE: f64 = 3.0;
/// Default azimuth and elevation in degrees (matplotlib `view_init`).
pub const DEFAULT_AZIMUTH: f64 = -60.0;
/// Default elevation in degrees.
pub const DEFAULT_ELEVATION: f64 = 30.0;
/// Geoms that draw under `coord_3d`.
pub const GEOMS: &[&str] = &["point", "path", "surface"];

/// The camera of a `coord_3d` plot.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct View {
    /// Azimuth in radians.
    azimuth: f64,
    /// Elevation in radians.
    elevation: f64,
    /// Perspective (true) or orthographic projection.
    perspective: bool,
}

impl View {
    /// The view of a `coord_3d()` spec; angles must be finite.
    pub fn from_spec(spec: &CoordSpec) -> Result<View> {
        let angle = |value: Option<f64>, default: f64, key: &str| match value {
            None => Ok(default),
            Some(value) if value.is_finite() => Ok(value),
            Some(value) => Err(Error::spec(
                format!("coord.{key}"),
                format!("expected a finite angle in degrees, got {value}"),
            )),
        };
        Ok(View {
            azimuth: angle(spec.azimuth, DEFAULT_AZIMUTH, "azimuth")?.to_radians(),
            elevation: angle(spec.elevation, DEFAULT_ELEVATION, "elevation")?.to_radians(),
            perspective: spec.perspective.unwrap_or(false),
        })
    }

    /// Unit vector from the box centre towards the viewer, in box axes.
    pub fn eye(&self) -> [f64; 3] {
        let (sin_azimuth, cos_azimuth) = self.azimuth.sin_cos();
        let (sin_elevation, cos_elevation) = self.elevation.sin_cos();
        [
            cos_elevation * cos_azimuth,
            cos_elevation * sin_azimuth,
            sin_elevation,
        ]
    }

    /// Screen right, screen up and depth (towards the viewer) of a point whose
    /// scaled positions are in 0..1, in box units around the box centre.
    fn rotate(&self, position: [f64; 3]) -> [f64; 3] {
        let [x, y, z] = [0, 1, 2].map(|axis| (position[axis] - 0.5) * BOX_ASPECT[axis]);
        let (sin_azimuth, cos_azimuth) = self.azimuth.sin_cos();
        let (sin_elevation, cos_elevation) = self.elevation.sin_cos();
        let right = -x * sin_azimuth + y * cos_azimuth;
        let up =
            -x * sin_elevation * cos_azimuth - y * sin_elevation * sin_azimuth + z * cos_elevation;
        let depth =
            x * cos_elevation * cos_azimuth + y * cos_elevation * sin_azimuth + z * sin_elevation;
        if self.perspective {
            let factor = CAMERA_DISTANCE / (CAMERA_DISTANCE - depth);
            [right * factor, up * factor, depth]
        } else {
            [right, up, depth]
        }
    }
}

/// A [`View`] fitted into a rectangle: maps scaled positions to pixels.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Projector {
    view: View,
    /// Pixels per box unit.
    scale: f64,
    /// Pixel position of the box's projected bounding-box centre.
    centre: (f64, f64),
    /// Rotated coordinates of that centre.
    offset: (f64, f64),
}

impl Projector {
    /// The view scaled so the whole box fits in `rect`, centred.
    pub fn fit(view: View, rect: Rect) -> Projector {
        let mut low = (f64::INFINITY, f64::INFINITY);
        let mut high = (f64::NEG_INFINITY, f64::NEG_INFINITY);
        for corner in 0..8 {
            let position = [0, 1, 2].map(|axis| f64::from((corner >> axis) & 1));
            let [right, up, _] = view.rotate(position);
            low = (low.0.min(right), low.1.min(up));
            high = (high.0.max(right), high.1.max(up));
        }
        let (width, height) = ((high.0 - low.0).max(1e-9), (high.1 - low.1).max(1e-9));
        let scale = (rect.width / width).min(rect.height / height).max(0.0);
        Projector {
            view,
            scale,
            centre: (rect.x + rect.width / 2.0, rect.y + rect.height / 2.0),
            offset: ((low.0 + high.0) / 2.0, (low.1 + high.1) / 2.0),
        }
    }

    /// The camera.
    pub fn view(&self) -> View {
        self.view
    }

    /// Pixel x, pixel y and depth (larger is nearer) of scaled positions in 0..1.
    /// Missing values give NaN.
    pub fn project(&self, position: [f64; 3]) -> (f64, f64, f64) {
        let [right, up, depth] = self.view.rotate(position);
        (
            self.centre.0 + (right - self.offset.0) * self.scale,
            self.centre.1 - (up - self.offset.1) * self.scale,
            depth,
        )
    }

    /// Pixel position of scaled positions (see [`Self::project`]).
    pub fn pixel(&self, position: [f64; 3]) -> (f64, f64) {
        let (x, y, _) = self.project(position);
        (x, y)
    }
}

/// The position scale that maps 0..1 to itself: projected layers carry their
/// screen position as a fraction of the panel.
pub fn identity_scale() -> PositionScale {
    let mut scale = ContinuousPositionScale::default();
    scale.multiplicative_expand = 0.0;
    scale.additive_expand = 0.0;
    scale.train_range(0.0, 1.0);
    PositionScale::Continuous(scale)
}

/// The scales of a 3D panel, data axes.
pub struct Scales3d<'a> {
    /// x.
    pub x: &'a PositionScale,
    /// y.
    pub y: &'a PositionScale,
    /// z.
    pub z: &'a PositionScale,
}

/// A layer's rows projected into `panel` and ordered back to front, ready for its
/// 2D geom drawn with [`identity_scale`] on both axes: x and y become fractions
/// of the panel (y up). Points sort row by row, paths (`by_group`) group by group
/// on their mean depth, rows within a group kept in order.
pub fn project_layer(
    data: &LayerData,
    styles: &Styles,
    scales: &Scales3d,
    projector: &Projector,
    panel: Rect,
    by_group: bool,
) -> Result<(LayerData, Styles)> {
    let rows = data.row_count();
    let column = |aesthetic: Aesthetic| data.continuous(aesthetic);
    let (Some(x), Some(y), Some(z)) = (
        column(Aesthetic::X),
        column(Aesthetic::Y),
        column(Aesthetic::Z),
    ) else {
        return Ok((data.take(&[]), styles.take(&[])));
    };
    let value = |column: &ContinuousColumn, row: usize| column.get(row).unwrap_or(f64::NAN);
    let mut screen_x = Vec::with_capacity(rows);
    let mut screen_y = Vec::with_capacity(rows);
    let mut depth = Vec::with_capacity(rows);
    for row in 0..rows {
        let position = [
            scales.x.map(value(x, row)),
            scales.y.map(value(y, row)),
            scales.z.map(value(z, row)),
        ];
        let (pixel_x, pixel_y, row_depth) = projector.project(position);
        screen_x.push((pixel_x - panel.x) / panel.width);
        screen_y.push(1.0 - (pixel_y - panel.y) / panel.height);
        depth.push(row_depth);
    }
    let order = match (by_group, data.group_index()) {
        (true, Some(groups)) => group_order(groups, &depth),
        (true, None) => (0..rows as u32).collect(),
        (false, _) => {
            let mut order: Vec<u32> = (0..rows as u32).collect();
            // Missing depths first: they draw nothing.
            order.sort_by(|left, right| {
                let key = |row: &u32| depth[*row as usize];
                match (key(left).is_nan(), key(right).is_nan()) {
                    (true, true) => std::cmp::Ordering::Equal,
                    (true, false) => std::cmp::Ordering::Less,
                    (false, true) => std::cmp::Ordering::Greater,
                    (false, false) => key(left).total_cmp(&key(right)),
                }
            });
            order
        }
    };
    let mut projected = data.take(&order);
    let pick = |values: &[f64]| {
        ContinuousColumn::from_values(order.iter().map(|&row| values[row as usize]).collect())
    };
    projected.insert(Aesthetic::X, Column::Continuous(pick(&screen_x)))?;
    projected.insert(Aesthetic::Y, Column::Continuous(pick(&screen_y)))?;
    if by_group && let Some(groups) = projected.group_index() {
        // Number groups in drawing order: paths draw by group index.
        let mut rank: Vec<u32> = Vec::new();
        let mut seen: Vec<u32> = Vec::new();
        let renumbered: Vec<u32> = groups
            .iter()
            .map(|group| match seen.iter().position(|known| known == group) {
                Some(at) => rank[at],
                None => {
                    seen.push(*group);
                    rank.push(rank.len() as u32);
                    rank.len() as u32 - 1
                }
            })
            .collect();
        projected.set_group_index(Some(renumbered))?;
    }
    Ok((projected, styles.take(&order)))
}

/// Rows grouped, groups ordered by their mean finite depth (far first, groups
/// without one first), rows within a group in data order.
fn group_order(groups: &[u32], depth: &[f64]) -> Vec<u32> {
    let count = groups.iter().max().map_or(0, |max| *max as usize + 1);
    let mut sums = vec![(0.0, 0usize); count];
    let mut members: Vec<Vec<u32>> = vec![Vec::new(); count];
    for (row, &group) in groups.iter().enumerate() {
        members[group as usize].push(row as u32);
        if depth[row].is_finite() {
            sums[group as usize].0 += depth[row];
            sums[group as usize].1 += 1;
        }
    }
    let mean = |group: usize| match sums[group] {
        (_, 0) => f64::NEG_INFINITY,
        (sum, count) => sum / count as f64,
    };
    let mut order: Vec<usize> = (0..count).collect();
    order.sort_by(|left, right| mean(*left).total_cmp(&mean(*right)));
    order
        .into_iter()
        .flat_map(|group| std::mem::take(&mut members[group]))
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    fn rect() -> Rect {
        Rect {
            x: 0.0,
            y: 0.0,
            width: 200.0,
            height: 100.0,
        }
    }

    fn view(azimuth: f64, elevation: f64) -> View {
        View {
            azimuth: azimuth.to_radians(),
            elevation: elevation.to_radians(),
            perspective: false,
        }
    }

    #[test]
    fn the_box_fits_and_is_centred() {
        let projector = Projector::fit(view(-60.0, 30.0), rect());
        let (mut low, mut high) = ((f64::INFINITY, f64::INFINITY), (0.0f64, 0.0f64));
        for corner in 0..8 {
            let position = [0, 1, 2].map(|axis| f64::from((corner >> axis) & 1));
            let (x, y) = projector.pixel(position);
            low = (low.0.min(x), low.1.min(y));
            high = (high.0.max(x), high.1.max(y));
        }
        assert!(low.0 >= -1e-9 && low.1 >= -1e-9 && high.0 <= 200.0 + 1e-9);
        assert!(high.1 <= 100.0 + 1e-9);
        // The tighter dimension is filled.
        assert!((high.1 - low.1 - 100.0).abs() < 1e-9 || (high.0 - low.0 - 200.0).abs() < 1e-9);
        assert!(((low.0 + high.0) / 2.0 - 100.0).abs() < 1e-9);
    }

    #[test]
    fn z_points_up_and_the_eye_is_nearest() {
        let projector = Projector::fit(view(-60.0, 30.0), rect());
        let (_, bottom, _) = projector.project([0.5, 0.5, 0.0]);
        let (_, top, _) = projector.project([0.5, 0.5, 1.0]);
        assert!(top < bottom);
        // From the front (azimuth 0, elevation 0) x points at the viewer.
        let front = Projector::fit(view(0.0, 0.0), rect());
        let (_, _, near) = front.project([1.0, 0.5, 0.5]);
        let (_, _, far) = front.project([0.0, 0.5, 0.5]);
        assert!(near > far);
        // ... and y points right.
        let (left, _) = front.pixel([0.5, 0.0, 0.5]);
        let (right, _) = front.pixel([0.5, 1.0, 0.5]);
        assert!(right > left);
    }

    #[test]
    fn perspective_shrinks_the_far_side() {
        let mut far_view = view(0.0, 0.0);
        far_view.perspective = true;
        let projector = Projector::fit(far_view, rect());
        let width = |x: f64| {
            let (left, _) = projector.pixel([x, 0.0, 0.5]);
            let (right, _) = projector.pixel([x, 1.0, 0.5]);
            right - left
        };
        assert!(width(1.0) > width(0.0));
    }

    #[test]
    fn groups_order_by_mean_depth() {
        let order = group_order(&[0, 0, 1, 1, 2], &[5.0, 3.0, -1.0, 0.0, f64::NAN]);
        assert_eq!(order, vec![4, 2, 3, 0, 1]);
    }

    #[test]
    fn non_finite_angles_are_rejected() {
        let spec = CoordSpec {
            name: "3d".into(),
            xlim: None,
            ylim: None,
            ratio: None,
            flip: false,
            expand: None,
            azimuth: Some(f64::NAN),
            elevation: None,
            perspective: None,
            theta: None,
            start: None,
            direction: None,
            transform_bkgr: None,
            unsupported: Default::default(),
        };
        let error = View::from_spec(&spec).unwrap_err().to_string();
        assert!(error.contains("coord.azimuth"), "{error}");
    }
}

//! `stat="hull"`: the convex hull of each group, as plotnine's `stat_hull`
//! (ggplot2 has none).
//!
//! Per group: the hull vertices counter-clockwise from the lowest-left point, the
//! first repeated at the end so a path closes, with `..area..` (the enclosed area).
//! Andrew's monotone chain; collinear points on an edge are left out.
//!
//! What it does not do: hulls of fewer than three non-collinear points (the group
//! draws nothing; plotnine raises a Qhull error).

use super::groups::per_group;
use super::{Stat, StatContext};
use crate::Result;
use crate::data::{Computed, LayerData, Variable};
use crate::parameters::Parameters;
use crate::spec::Aesthetic;

/// Convex hulls.
pub struct HullStat;

impl Stat for HullStat {
    fn name(&self) -> &'static str {
        "hull"
    }

    fn aesthetics(&self) -> &'static [Aesthetic] {
        &[Aesthetic::X, Aesthetic::Y]
    }

    fn required_aesthetics(&self) -> &'static [Aesthetic] {
        &[Aesthetic::X, Aesthetic::Y]
    }

    fn compute(
        &self,
        data: LayerData,
        parameters: &Parameters,
        _context: &StatContext,
    ) -> Result<LayerData> {
        let (Some(x), Some(y)) = (data.continuous(Aesthetic::X), data.continuous(Aesthetic::Y))
        else {
            return Err(crate::Error::spec(
                parameters.path_of("mapping"),
                "stat_hull needs numeric x and y positions",
            ));
        };
        per_group(&data, |rows| {
            let points: Vec<(f64, f64)> = rows
                .iter()
                .filter_map(|&row| Some((x.get(row)?, y.get(row)?)))
                .filter(|(x, y)| x.is_finite() && y.is_finite())
                .collect();
            let hull = convex_hull(points);
            if hull.len() < 3 {
                return Ok(Vec::new());
            }
            let area = polygon_area(&hull);
            let mut xs: Vec<f64> = hull.iter().map(|point| point.0).collect();
            let mut ys: Vec<f64> = hull.iter().map(|point| point.1).collect();
            xs.push(xs[0]);
            ys.push(ys[0]);
            let n = xs.len();
            Ok(vec![
                (Variable::Aesthetic(Aesthetic::X), xs),
                (Variable::Aesthetic(Aesthetic::Y), ys),
                (Variable::Computed(Computed::Area), vec![area; n]),
            ])
        })
    }
}

/// Twice the signed area of the triangle o, a, b: positive for a left turn.
fn cross(o: (f64, f64), a: (f64, f64), b: (f64, f64)) -> f64 {
    (a.0 - o.0) * (b.1 - o.1) - (a.1 - o.1) * (b.0 - o.0)
}

/// Hull vertices counter-clockwise, starting at the lowest x (then lowest y).
fn convex_hull(mut points: Vec<(f64, f64)>) -> Vec<(f64, f64)> {
    points.sort_by(|a, b| a.0.total_cmp(&b.0).then(a.1.total_cmp(&b.1)));
    points.dedup();
    if points.len() < 3 {
        return points;
    }
    let mut hull: Vec<(f64, f64)> = Vec::with_capacity(2 * points.len());
    for pass in [false, true] {
        let start = hull.len();
        let ordered: Box<dyn Iterator<Item = &(f64, f64)>> = if pass {
            Box::new(points.iter().rev())
        } else {
            Box::new(points.iter())
        };
        for &point in ordered {
            while hull.len() >= start + 2
                && cross(hull[hull.len() - 2], hull[hull.len() - 1], point) <= 0.0
            {
                hull.pop();
            }
            hull.push(point);
        }
        hull.pop();
    }
    hull
}

/// Shoelace area of a simple polygon.
fn polygon_area(vertices: &[(f64, f64)]) -> f64 {
    let n = vertices.len();
    let twice: f64 = (0..n)
        .map(|index| {
            let (a, b) = (vertices[index], vertices[(index + 1) % n]);
            a.0 * b.1 - b.0 * a.1
        })
        .sum();
    twice.abs() / 2.0
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn square_with_inner_and_edge_points() {
        let points = vec![
            (0.0, 0.0),
            (2.0, 0.0),
            (1.0, 0.0),
            (2.0, 2.0),
            (0.0, 2.0),
            (1.0, 1.0),
            (0.0, 2.0),
        ];
        let hull = convex_hull(points);
        assert_eq!(hull, [(0.0, 0.0), (2.0, 0.0), (2.0, 2.0), (0.0, 2.0)]);
        assert_eq!(polygon_area(&hull), 4.0);
    }

    #[test]
    fn collinear_points_have_no_hull() {
        assert!(convex_hull(vec![(0.0, 0.0), (1.0, 1.0), (2.0, 2.0)]).len() < 3);
    }
}

//! `position="dodge"`: place groups that share an x side by side, as lets-plot's
//! `DodgePos`.
//!
//! Each group gets a slot by its group index; every x position (x, xmin, xmax)
//! moves towards the slot and shrinks by the number of groups:
//! `(v + (slot - (n - 1) / 2) * resolution * width - x) / n + x`. `width` is the
//! option, else the row's `width` (bars), else 1. Nothing moves when no x holds
//! more than one group.
//!
//! What it does not do: `dodgev` (vertical dodging).

use std::collections::HashMap;
use std::collections::HashSet;

use super::Position;
use super::shift::shift_positions;
use crate::Result;
use crate::data::LayerData;
use crate::parameters::Parameters;
use crate::spec::Aesthetic;

/// Puts groups side by side.
pub struct DodgePosition;

impl Position for DodgePosition {
    fn name(&self) -> &'static str {
        "dodge"
    }

    fn parameters(&self) -> &'static [&'static str] {
        &["width"]
    }

    fn adjust(&self, data: LayerData, parameters: &Parameters) -> Result<LayerData> {
        dodge(data, parameters.number("width")?, None)
    }
}

/// Dodge `data` (lets-plot `BaseDodgePos.translate`). `jitter` adds a per-row x
/// offset before dodging, around the unjittered x (jitterdodge).
pub(super) fn dodge(
    data: LayerData,
    width: Option<f64>,
    jitter: Option<&[f64]>,
) -> Result<LayerData> {
    let Some(x) = data.continuous(Aesthetic::X).cloned() else {
        return Ok(data);
    };
    let jittered = |row: usize, value: f64| value + jitter.map_or(0.0, |jitter| jitter[row]);
    if !dodging_needed(&data) {
        return shift_positions(data, Aesthetic::is_x_position, jittered);
    }
    let group_count = (0..data.row_count())
        .map(|row| data.group_of(row))
        .collect::<HashSet<_>>()
        .len() as f64;
    let median = (group_count - 1.0) / 2.0;
    let resolution = data.resolution(Aesthetic::X);
    let widths = data.continuous(Aesthetic::Width).cloned();
    let groups: Vec<f64> = (0..data.row_count())
        .map(|row| data.group_of(row) as f64)
        .collect();
    shift_positions(data, Aesthetic::is_x_position, |row, value| {
        let size = width
            .or_else(|| {
                widths
                    .as_ref()
                    .map(|widths| widths.get(row).unwrap_or(f64::NAN))
            })
            .unwrap_or(1.0);
        let Some(center) = x.get(row).filter(|center| center.is_finite()) else {
            return jittered(row, value);
        };
        if !size.is_finite() {
            return jittered(row, value);
        }
        let offset = (groups[row] - median) * resolution * size;
        (jittered(row, value) + offset - center) / group_count + center
    })
}

/// lets-plot `isDodgingNeeded`: some x holds rows of more than one group.
fn dodging_needed(data: &LayerData) -> bool {
    let Some(x) = data.continuous(Aesthetic::X) else {
        return false;
    };
    let mut group_by_location: HashMap<u64, u32> = HashMap::new();
    for row in 0..data.row_count() {
        let Some(location) = x.get(row).filter(|value| !value.is_nan()) else {
            continue;
        };
        // Boxed doubles compare by bits, as in lets-plot.
        let key = location.to_bits();
        let group = data.group_of(row);
        if *group_by_location.entry(key).or_insert(group) != group {
            return true;
        }
    }
    false
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::{Column, ContinuousColumn};
    use crate::spec::Options;

    #[test]
    fn groups_split_the_bar_width() {
        let mut data = LayerData::new(3);
        let column = |values: Vec<f64>| Column::Continuous(ContinuousColumn::from_values(values));
        data.insert(Aesthetic::X, column(vec![0.0, 0.0, 1.0]))
            .unwrap();
        data.insert(Aesthetic::Xmin, column(vec![-0.45, -0.45, 0.55]))
            .unwrap();
        data.insert(Aesthetic::Width, column(vec![0.9; 3])).unwrap();
        data.set_group_index(Some(vec![0, 1, 1])).unwrap();
        let options = Options::new();
        let data = DodgePosition
            .adjust(data, &Parameters::new(&options, ""))
            .unwrap();
        let x = data.continuous(Aesthetic::X).unwrap();
        let xmin = data.continuous(Aesthetic::Xmin).unwrap();
        // Two groups: centers at -/+ 0.225, each 0.45 wide.
        assert!((x.get(0).unwrap() + 0.225).abs() < 1e-12);
        assert!((x.get(1).unwrap() - 0.225).abs() < 1e-12);
        assert!((xmin.get(1).unwrap() - 0.0).abs() < 1e-12);
        assert!((x.get(2).unwrap() - 1.225).abs() < 1e-12);
    }
}

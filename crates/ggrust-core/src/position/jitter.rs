//! `position="jitter"` and `position="jitterdodge"`: random offsets so
//! overlapping points show, as lets-plot's `JitterPos` / `JitterDodgePos`.
//!
//! Each row with a finite x and y moves by `(2u - 1) x width x resolution(x)` and
//! `(2v - 1) x height x resolution(y)` (u, v uniform, drawn x then y in row
//! order; width and height default to 0.4). The generator is Kotlin's, so a
//! `seed` gives lets-plot's points; without one the seed is
//! [`DEFAULT_SEED`](super::random::DEFAULT_SEED), so renders are reproducible.
//! Jitterdodge jitters (`jitter_width`, `jitter_height`), then dodges by group
//! (`dodge_width`) around the unjittered x.

use super::Position;
use super::dodge::dodge;
use super::random::{DEFAULT_SEED, KotlinRandom};
use super::shift::shift_positions;
use crate::Result;
use crate::data::LayerData;
use crate::parameters::Parameters;
use crate::spec::Aesthetic;

/// lets-plot `JitterPos.DEF_JITTER_WIDTH` / `DEF_JITTER_HEIGHT`.
const DEFAULT_SIZE: f64 = 0.4;

/// Random offsets.
pub struct JitterPosition;

impl Position for JitterPosition {
    fn name(&self) -> &'static str {
        "jitter"
    }

    fn parameters(&self) -> &'static [&'static str] {
        &["width", "height", "seed"]
    }

    fn adjust(&self, data: LayerData, parameters: &Parameters) -> Result<LayerData> {
        let (dx, dy) = offsets(
            &data,
            parameters.number("width")?,
            parameters.number("height")?,
            seed(parameters)?,
        );
        let data = shift_positions(data, Aesthetic::is_x_position, |row, value| value + dx[row])?;
        shift_positions(data, Aesthetic::is_y_position, |row, value| value + dy[row])
    }
}

/// Random offsets, then side by side by group.
pub struct JitterDodgePosition;

impl Position for JitterDodgePosition {
    fn name(&self) -> &'static str {
        "jitterdodge"
    }

    fn parameters(&self) -> &'static [&'static str] {
        &["dodge_width", "jitter_width", "jitter_height", "seed"]
    }

    fn adjust(&self, data: LayerData, parameters: &Parameters) -> Result<LayerData> {
        let (dx, dy) = offsets(
            &data,
            parameters.number("jitter_width")?,
            parameters.number("jitter_height")?,
            seed(parameters)?,
        );
        let data = shift_positions(data, Aesthetic::is_y_position, |row, value| value + dy[row])?;
        dodge(data, parameters.number("dodge_width")?, Some(&dx))
    }
}

fn seed(parameters: &Parameters) -> Result<i64> {
    Ok(parameters
        .number("seed")?
        .map_or(DEFAULT_SEED, |seed| seed as i64))
}

/// Per-row (x, y) offsets in axis units; rows without a finite x and y keep
/// their place and draw no numbers.
fn offsets(
    data: &LayerData,
    width: Option<f64>,
    height: Option<f64>,
    seed: i64,
) -> (Vec<f64>, Vec<f64>) {
    let rows = data.row_count();
    let (Some(x), Some(y)) = (data.continuous(Aesthetic::X), data.continuous(Aesthetic::Y)) else {
        return (vec![0.0; rows], vec![0.0; rows]);
    };
    let horizontal = width.unwrap_or(DEFAULT_SIZE) * data.resolution(Aesthetic::X);
    let vertical = height.unwrap_or(DEFAULT_SIZE) * data.resolution(Aesthetic::Y);
    let mut random = KotlinRandom::new(seed);
    let finite = |column: &crate::data::ContinuousColumn, row: usize| {
        column.get(row).is_some_and(f64::is_finite)
    };
    (0..rows)
        .map(|row| {
            if !(finite(x, row) && finite(y, row)) {
                return (0.0, 0.0);
            }
            let dx = (2.0 * random.next_double() - 1.0) * horizontal;
            let dy = (2.0 * random.next_double() - 1.0) * vertical;
            (dx, dy)
        })
        .unzip()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::{Column, ContinuousColumn};
    use crate::spec::Options;

    fn jittered(options: &str) -> Vec<(f64, f64)> {
        let mut data = LayerData::new(4);
        let column = |values: Vec<f64>| Column::Continuous(ContinuousColumn::from_values(values));
        data.insert(Aesthetic::X, column(vec![0.0, 0.0, 1.0, 1.0]))
            .unwrap();
        data.insert(Aesthetic::Y, column(vec![1.0, 1.5, 2.0, 2.5]))
            .unwrap();
        let options: Options = serde_json::from_str(options).unwrap();
        let data = JitterPosition
            .adjust(data, &Parameters::new(&options, ""))
            .unwrap();
        let (x, y) = (
            data.continuous(Aesthetic::X).unwrap(),
            data.continuous(Aesthetic::Y).unwrap(),
        );
        (0..4)
            .map(|row| (x.get(row).unwrap(), y.get(row).unwrap()))
            .collect()
    }

    #[test]
    fn offsets_stay_within_width_and_height_times_resolution() {
        let points = jittered(r#"{"width": 0.3, "height": 0.2, "seed": 7}"#);
        for ((x, y), (x0, y0)) in
            points
                .iter()
                .zip([(0.0, 1.0), (0.0, 1.5), (1.0, 2.0), (1.0, 2.5)])
        {
            assert!((x - x0).abs() <= 0.3 && (y - y0).abs() <= 0.2 * 0.5);
        }
        // The same seed gives the same points; another seed does not.
        assert_eq!(
            points,
            jittered(r#"{"width": 0.3, "height": 0.2, "seed": 7}"#)
        );
        assert_ne!(
            points,
            jittered(r#"{"width": 0.3, "height": 0.2, "seed": 8}"#)
        );
        assert_eq!(jittered("{}"), jittered("{}"));
    }
}

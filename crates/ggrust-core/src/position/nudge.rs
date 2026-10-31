//! `position="nudge"`: move every row by a fixed amount, as lets-plot's `NudgePos`.
//!
//! `x` and `y` (default 0) are added to every x-axis and y-axis position column
//! in data units. The axes do not grow to the nudged positions (lets-plot), so
//! a discrete axis keeps its spacing. What it does not do: `unit="size"` or
//! `unit="px"`.

use super::Position;
use super::shift::shift_positions;
use crate::data::LayerData;
use crate::parameters::Parameters;
use crate::spec::Aesthetic;
use crate::{Error, Result};

/// Moves rows by a constant offset.
pub struct NudgePosition;

impl Position for NudgePosition {
    fn name(&self) -> &'static str {
        "nudge"
    }

    fn parameters(&self) -> &'static [&'static str] {
        &["x", "y", "unit"]
    }

    fn trains_axes(&self) -> bool {
        false
    }

    fn adjust(&self, data: LayerData, parameters: &Parameters) -> Result<LayerData> {
        match parameters.string("unit")? {
            None | Some("identity") => {}
            Some(other) => {
                return Err(Error::spec(
                    parameters.path_of("unit"),
                    format!(
                        "unsupported nudge unit '{other}'; use 'identity' (data units), \
                         'size' and 'px' are not supported yet"
                    ),
                ));
            }
        }
        let x = parameters.number("x")?.unwrap_or(0.0);
        let y = parameters.number("y")?.unwrap_or(0.0);
        let data = match x {
            0.0 => data,
            x => shift_positions(data, Aesthetic::is_x_position, |_, value| value + x)?,
        };
        match y {
            0.0 => Ok(data),
            y => shift_positions(data, Aesthetic::is_y_position, |_, value| value + y),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::{Column, ContinuousColumn};
    use crate::spec::Options;

    #[test]
    fn nudge_moves_both_axes_in_data_units() {
        let mut data = LayerData::new(2);
        let column =
            |values: &[f64]| Column::Continuous(ContinuousColumn::from_values(values.to_vec()));
        data.insert(Aesthetic::X, column(&[1.0, 2.0])).unwrap();
        data.insert(Aesthetic::Y, column(&[3.0, 4.0])).unwrap();
        let options: Options = serde_json::from_str(r#"{"x": 0.5, "y": -1}"#).unwrap();
        let data = NudgePosition
            .adjust(data, &Parameters::new(&options, ""))
            .unwrap();
        assert_eq!(data.continuous(Aesthetic::X).unwrap().values(), [1.5, 2.5]);
        assert_eq!(data.continuous(Aesthetic::Y).unwrap().values(), [2.0, 3.0]);
    }

    #[test]
    fn other_units_are_an_error() {
        let options: Options = serde_json::from_str(r#"{"unit": "px"}"#).unwrap();
        let error = NudgePosition
            .adjust(LayerData::new(0), &Parameters::new(&options, "position"))
            .unwrap_err();
        assert!(error.to_string().contains("unsupported nudge unit 'px'"));
    }
}

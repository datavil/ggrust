//! Shared by positions: move every position column of one axis row by row.

use crate::Result;
use crate::data::{Column, LayerData};
use crate::spec::Aesthetic;

/// Replace each present value of the continuous columns whose aesthetic matches
/// `on_axis` (e.g. `y`, `ymin`, `ymax`) with `shift(row, value)`.
pub fn shift_positions(
    mut data: LayerData,
    on_axis: fn(Aesthetic) -> bool,
    shift: impl Fn(usize, f64) -> f64,
) -> Result<LayerData> {
    let aesthetics: Vec<Aesthetic> = data
        .aesthetics()
        .filter(|aesthetic| on_axis(*aesthetic))
        .collect();
    for aesthetic in aesthetics {
        let Some(column) = data.continuous(aesthetic) else {
            continue;
        };
        let values = (0..column.len())
            .map(|row| column.get(row).map(|value| shift(row, value)))
            .collect();
        data.insert(
            aesthetic,
            Column::Continuous(crate::data::ContinuousColumn::from_options(values)),
        )?;
    }
    Ok(data)
}

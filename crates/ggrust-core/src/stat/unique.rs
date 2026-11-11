//! `stat="unique"`: drops duplicate rows, as ggplot2's `StatUnique`.
//!
//! Two rows are duplicates when every column (aesthetics and stat variables) and
//! the group are equal; numbers compare by value bits, categories by code. The
//! first row of each set of duplicates is kept, in data order.
//!
//! What it does not do: compare columns that are not mapped (they do not reach
//! the engine).

use std::collections::HashSet;

use super::{Stat, StatContext};
use crate::Result;
use crate::data::{Column, LayerData};
use crate::parameters::Parameters;
use crate::spec::Aesthetic;

/// Drops duplicate rows.
pub struct UniqueStat;

impl Stat for UniqueStat {
    fn name(&self) -> &'static str {
        "unique"
    }

    fn aesthetics(&self) -> &'static [Aesthetic] {
        &[]
    }

    fn required_aesthetics(&self) -> &'static [Aesthetic] {
        &[]
    }

    fn compute(
        &self,
        data: LayerData,
        _parameters: &Parameters,
        _context: &StatContext,
    ) -> Result<LayerData> {
        let columns: Vec<&Column> = data.columns().map(|(_, column)| column).collect();
        let mut seen: HashSet<Vec<u64>> = HashSet::with_capacity(data.row_count());
        let kept: Vec<u32> = (0..data.row_count())
            .filter(|&row| {
                let mut key = Vec::with_capacity(columns.len() + 1);
                key.push(u64::from(data.group_of(row)));
                key.extend(columns.iter().map(|column| match column {
                    // Missing values share one key: u64::MAX is no f64 bit pattern
                    // a finite or NaN number produces here, nor a code.
                    Column::Continuous(values) => values.get(row).map_or(u64::MAX, f64::to_bits),
                    Column::Discrete(values) => values.code(row).map_or(u64::MAX, u64::from),
                }));
                seen.insert(key)
            })
            .map(|row| row as u32)
            .collect();
        if kept.len() == data.row_count() {
            return Ok(data);
        }
        Ok(data.take(&kept))
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::ContinuousColumn;
    use crate::spec::Options;

    #[test]
    fn keeps_the_first_of_each_duplicate_per_group() {
        let mut data = LayerData::new(5);
        let column =
            |values: &[f64]| Column::Continuous(ContinuousColumn::from_values(values.to_vec()));
        data.insert(Aesthetic::X, column(&[1.0, 1.0, 2.0, 1.0, f64::NAN]))
            .unwrap();
        data.insert(Aesthetic::Y, column(&[5.0, 5.0, 5.0, 5.0, 3.0]))
            .unwrap();
        data.set_group_index(Some(vec![0, 0, 0, 1, 0])).unwrap();
        let options = Options::default();
        let result = UniqueStat
            .compute(
                data,
                &Parameters::new(&options, ""),
                &StatContext::default(),
            )
            .unwrap();
        let x = result.continuous(Aesthetic::X).unwrap();
        let xs: Vec<Option<f64>> = (0..x.len()).map(|row| x.get(row)).collect();
        assert_eq!(xs.len(), 4);
        assert_eq!(xs[..3], [Some(1.0), Some(2.0), Some(1.0)]);
    }
}

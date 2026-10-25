//! Row groups, as lets-plot computes them (`DataProcessing.computeGroups`).
//!
//! One variable: rows get the index of their value in the variable's distinct
//! values (discrete: level order; continuous: first appearance), with missing
//! (null, NaN, +/-inf) as one extra group after them. Several variables: the combinations,
//! numbered in order of first appearance.

use std::collections::HashMap;

use super::Column;

/// Group index of every row for the given grouping columns, or `None` (one group)
/// when there are no columns. All columns must have the same length.
pub fn group_index(columns: &[&Column]) -> Option<Vec<u32>> {
    let (first, rest) = columns.split_first()?;
    let mut groups = column_groups(first);
    for column in rest {
        let next = column_groups(column);
        let mut by_pair: HashMap<(u32, u32), u32> = HashMap::new();
        for (group, other) in groups.iter_mut().zip(next) {
            let count = by_pair.len() as u32;
            *group = *by_pair.entry((*group, other)).or_insert(count);
        }
    }
    Some(groups)
}

fn column_groups(column: &Column) -> Vec<u32> {
    match column {
        Column::Discrete(column) => {
            let missing = column.levels().len() as u32;
            (0..column.len())
                .map(|row| column.code(row).unwrap_or(missing))
                .collect()
        }
        Column::Continuous(column) => {
            // lets-plot's Python layer turns NaN and +/-inf into None, so they join the
            // missing group; finite values compare by bits like Kotlin's boxed Double.
            let mut by_value: HashMap<Option<u64>, u32> = HashMap::new();
            let mut groups: Vec<u32> = (0..column.len())
                .map(|row| {
                    let key = column
                        .get(row)
                        .filter(|value| value.is_finite())
                        .map(f64::to_bits);
                    let count = by_value.len() as u32;
                    *by_value.entry(key).or_insert(count)
                })
                .collect();
            // Missing values come after every distinct value.
            if let Some(&missing) = by_value.get(&None) {
                let last = by_value.len() as u32 - 1;
                for group in &mut groups {
                    if *group == missing {
                        *group = last;
                    } else if *group > missing {
                        *group -= 1;
                    }
                }
            }
            groups
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::ContinuousColumn;
    use arrow_array::{ArrayRef, DictionaryArray, RecordBatch, StringArray, types::Int32Type};
    use std::sync::Arc;

    fn discrete(array: ArrayRef) -> Column {
        let batch = RecordBatch::try_from_iter([("value", array)]).unwrap();
        crate::data::DataFrame::new(batch.schema(), vec![batch])
            .unwrap()
            .column("value")
            .unwrap()
    }

    #[test]
    fn no_columns_is_one_group() {
        assert_eq!(group_index(&[]), None);
    }

    #[test]
    fn discrete_groups_follow_level_order_and_missing_comes_last() {
        let strings = discrete(Arc::new(StringArray::from(vec![
            Some("T"),
            None,
            Some("B"),
            Some("T"),
        ])));
        // Levels in first appearance: T, B; missing is group 2.
        assert_eq!(group_index(&[&strings]), Some(vec![0, 2, 1, 0]));
        let dictionary: DictionaryArray<Int32Type> = vec!["b", "a", "b"]
            .into_iter()
            .collect::<DictionaryArray<Int32Type>>();
        let categorical = discrete(Arc::new(dictionary));
        // Dictionary order (b, a) as stored.
        assert_eq!(group_index(&[&categorical]), Some(vec![0, 1, 0]));
    }

    #[test]
    fn continuous_groups_are_first_appearance_with_missing_last() {
        let column = Column::Continuous(ContinuousColumn::from_options(vec![
            None,
            Some(2.0),
            Some(f64::NAN),
            Some(1.0),
            Some(2.0),
            Some(f64::INFINITY),
            None,
        ]));
        // null, NaN and inf are one missing group, after 2 and 1.
        assert_eq!(group_index(&[&column]), Some(vec![2, 0, 2, 1, 0, 2, 2]));
    }

    #[test]
    fn combinations_number_in_first_appearance() {
        let letters = discrete(Arc::new(StringArray::from(vec!["a", "b", "a", "b", "a"])));
        let numbers =
            Column::Continuous(ContinuousColumn::from_values(vec![2.0, 1.0, 1.0, 1.0, 2.0]));
        assert_eq!(
            group_index(&[&numbers, &letters]),
            Some(vec![0, 1, 2, 1, 0])
        );
    }
}

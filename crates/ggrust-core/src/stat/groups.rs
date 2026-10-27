//! Runs a stat once per group, as lets-plot's `DataProcessing.buildStatData`.
//!
//! Groups are visited in order of first appearance. Each group's stat rows get the
//! group's other columns as one value: the mean of a continuous column, the first
//! present value of a discrete one. Output groups are numbered in visiting order.
//!
//! What it does not do: reorder groups (`order_by`), or keep source rows (stat rows
//! summarise many input rows).

use rayon::prelude::*;

use crate::Result;
use crate::data::{Column, ContinuousColumn, LayerData, SourceRows, Variable};

/// Stat columns of one group: every column has the same number of rows.
pub type GroupColumns = Vec<(Variable, Vec<f64>)>;

/// Input rows of every group, groups in first-appearance order.
pub fn rows_by_group(data: &LayerData) -> Vec<Vec<usize>> {
    let mut slot_of_group: Vec<Option<usize>> = vec![None; data.group_count()];
    let mut groups: Vec<Vec<usize>> = Vec::new();
    for row in 0..data.row_count() {
        let group = data.group_of(row) as usize;
        let slot = *slot_of_group[group].get_or_insert_with(|| {
            groups.push(Vec::new());
            groups.len() - 1
        });
        groups[slot].push(row);
    }
    groups
}

/// Input rows of every group, groups in group index order (lets-plot positions
/// sort by group, e.g. the stacking order).
pub fn rows_by_group_id(data: &LayerData) -> Vec<Vec<usize>> {
    let mut groups: Vec<Vec<usize>> = vec![Vec::new(); data.group_count()];
    for row in 0..data.row_count() {
        groups[data.group_of(row) as usize].push(row);
    }
    groups
}

/// Apply `compute` to the rows of every group (in parallel) and merge the results
/// in group order. Columns of `data` the stat does not produce are carried over
/// (mean / first value).
pub fn per_group(
    data: &LayerData,
    compute: impl Fn(&[usize]) -> Result<GroupColumns> + Sync,
) -> Result<LayerData> {
    let mut groups = rows_by_group(data);
    if groups.is_empty() {
        // No rows: one empty group, so the stat still names its variables.
        groups.push(Vec::new());
    }
    let mut stat_columns: Vec<(Variable, Vec<f64>)> = Vec::new();
    let mut sizes: Vec<usize> = Vec::with_capacity(groups.len());
    let computed: Vec<Result<GroupColumns>> = groups.par_iter().map(|rows| compute(rows)).collect();
    for columns in computed {
        let columns = columns?;
        let size = columns.first().map_or(0, |(_, values)| values.len());
        for (variable, values) in columns {
            match stat_columns
                .iter_mut()
                .find(|(known, _)| *known == variable)
            {
                Some((_, all)) => all.extend(values),
                None => {
                    // A variable missing from earlier groups is NaN there.
                    let earlier: usize = sizes.iter().sum();
                    let mut all = vec![f64::NAN; earlier];
                    all.extend(values);
                    stat_columns.push((variable, all));
                }
            }
        }
        sizes.push(size);
    }
    let row_count: usize = sizes.iter().sum();
    let mut result = LayerData::new(row_count);
    for (variable, mut values) in stat_columns {
        values.resize(row_count, f64::NAN);
        result.insert(
            variable,
            Column::Continuous(ContinuousColumn::from_values(values)),
        )?;
    }
    for aesthetic in data.aesthetics().collect::<Vec<_>>() {
        if result.contains(aesthetic) {
            continue;
        }
        let Some(column) = data.get(aesthetic) else {
            continue;
        };
        result.insert(aesthetic, carried(column, &groups, &sizes))?;
    }
    let group_index: Vec<u32> = sizes
        .iter()
        .enumerate()
        .flat_map(|(group, size)| std::iter::repeat_n(group as u32, *size))
        .collect();
    result.set_group_index((groups.len() > 1).then_some(group_index))?;
    result.set_source_rows(SourceRows::Aggregated)?;
    Ok(result)
}

/// One value per group, repeated for the group's stat rows.
fn carried(column: &Column, groups: &[Vec<usize>], sizes: &[usize]) -> Column {
    match column {
        Column::Continuous(column) => {
            let mut values = Vec::new();
            for (rows, size) in groups.iter().zip(sizes) {
                let present: Vec<f64> = rows
                    .iter()
                    .filter_map(|&row| column.get(row))
                    .filter(|value| value.is_finite())
                    .collect();
                let mean = (!present.is_empty())
                    .then(|| present.iter().sum::<f64>() / present.len() as f64);
                values.extend(std::iter::repeat_n(mean, *size));
            }
            Column::Continuous(ContinuousColumn::from_options(values))
        }
        Column::Discrete(column) => {
            let mut codes = Vec::new();
            for (rows, size) in groups.iter().zip(sizes) {
                let first = rows.iter().find_map(|&row| column.code(row));
                codes.extend(std::iter::repeat_n(first, *size));
            }
            Column::Discrete(column.with_codes(&codes))
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::Computed;
    use crate::spec::Aesthetic;

    #[test]
    fn groups_in_first_appearance_with_carried_columns() {
        let mut data = LayerData::new(4);
        data.insert(
            Aesthetic::X,
            Column::Continuous(ContinuousColumn::from_values(vec![1.0, 2.0, 3.0, 4.0])),
        )
        .unwrap();
        data.insert(
            Aesthetic::Size,
            Column::Continuous(ContinuousColumn::from_values(vec![1.0, 10.0, 3.0, 20.0])),
        )
        .unwrap();
        data.set_group_index(Some(vec![1, 0, 1, 0])).unwrap();
        let result = per_group(&data, |rows| {
            Ok(vec![(
                Variable::Computed(Computed::Count),
                vec![rows.len() as f64],
            )])
        })
        .unwrap();
        assert_eq!(result.row_count(), 2);
        // Group 1 is met first.
        let size = result.continuous(Aesthetic::Size).unwrap();
        assert_eq!((size.get(0), size.get(1)), (Some(2.0), Some(15.0)));
        assert_eq!(result.group_index(), Some(&[0, 1][..]));
        assert_eq!(result.source_rows(), &SourceRows::Aggregated);
    }

    #[test]
    fn no_rows_still_names_the_stat_variables() {
        let result = per_group(&LayerData::new(0), |rows| {
            Ok(vec![(
                Variable::Computed(Computed::Count),
                vec![1.0; rows.len()],
            )])
        })
        .unwrap();
        assert_eq!(result.row_count(), 0);
        assert!(result.contains(Computed::Count));
    }
}

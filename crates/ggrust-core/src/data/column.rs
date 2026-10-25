//! Typed columns: continuous (`f64`) or discrete (levels + codes).

use std::collections::HashMap;
use std::sync::Arc;

use arrow_array::cast::AsArray;
use arrow_array::types::UInt32Type;
use arrow_array::{Array, ArrayRef, BooleanArray, Float64Array, UInt32Array};
use arrow_schema::DataType;

use crate::{Error, Result};

/// A column ready for mapping onto an aesthetic.
#[derive(Debug, Clone, PartialEq)]
pub enum Column {
    /// Numbers.
    Continuous(ContinuousColumn),
    /// Categories: strings, booleans, categoricals.
    Discrete(DiscreteColumn),
}

impl Column {
    /// Number of rows.
    pub fn len(&self) -> usize {
        match self {
            Column::Continuous(column) => column.len(),
            Column::Discrete(column) => column.len(),
        }
    }

    /// Whether the column has no rows.
    pub fn is_empty(&self) -> bool {
        self.len() == 0
    }

    /// Convert an Arrow array. `name` is only used in error messages.
    ///
    /// - floats pass through without copying; integers and decimals are cast to `f64`
    ///   (integers beyond 2^53 lose precision)
    /// - strings become levels in order of first appearance (as lets-plot does);
    ///   they are read in place, never cast
    /// - dictionaries (polars Categorical/Enum, pandas category) keep dictionary
    ///   order, dropping levels that no row uses (as lets-plot does for factor levels)
    /// - booleans become the levels `"true"`/`"false"` in order of first appearance
    /// - an all-null column becomes an all-missing continuous column
    pub(super) fn from_arrow(name: &str, array: &ArrayRef) -> Result<Self> {
        let discrete = |column: DiscreteColumn| Ok(Column::Discrete(column));
        match array.data_type() {
            DataType::Float64 => Ok(Column::Continuous(ContinuousColumn {
                values: array.as_primitive().clone(),
            })),
            DataType::Null => Ok(Column::Continuous(ContinuousColumn {
                values: Float64Array::new_null(array.len()),
            })),
            data_type if data_type.is_numeric() => {
                let values = cast(name, array, &DataType::Float64)?;
                Ok(Column::Continuous(ContinuousColumn {
                    values: values.as_primitive().clone(),
                }))
            }
            DataType::Utf8 => discrete(DiscreteColumn::from_strings(
                array.as_string::<i32>().iter(),
            )),
            DataType::LargeUtf8 => discrete(DiscreteColumn::from_strings(
                array.as_string::<i64>().iter(),
            )),
            DataType::Utf8View => {
                discrete(DiscreteColumn::from_strings(array.as_string_view().iter()))
            }
            DataType::Boolean => discrete(DiscreteColumn::from_booleans(array.as_boolean())),
            DataType::Dictionary(_, value_type) if is_label_type(value_type) => {
                let dictionary = array.as_any_dictionary();
                let keys = dictionary_keys(name, dictionary.keys())?;
                let labels = labels(name, dictionary.values())?;
                DiscreteColumn::from_dictionary(name, &keys, &labels).map(Column::Discrete)
            }
            other => Err(unsupported(name, other)),
        }
    }

    /// Merge chunk columns of one data column, in order.
    ///
    /// Discrete levels keep the first chunk's order; levels new in later chunks
    /// are appended in their own order.
    pub(super) fn concat(name: &str, mut parts: Vec<Column>) -> Result<Self> {
        if parts.len() == 1 {
            return Ok(parts.remove(0));
        }
        let mismatch = || Error::InvalidData(format!("column '{name}': chunks differ in kind"));
        match parts.first() {
            Some(Column::Continuous(_)) => {
                let arrays = parts
                    .iter()
                    .map(|part| match part {
                        Column::Continuous(column) => Ok(&column.values as &dyn Array),
                        Column::Discrete(_) => Err(mismatch()),
                    })
                    .collect::<Result<Vec<_>>>()?;
                let values = arrow_select::concat::concat(&arrays)
                    .map_err(|error| Error::InvalidData(error.to_string()))?;
                Ok(Column::Continuous(ContinuousColumn {
                    values: values.as_primitive().clone(),
                }))
            }
            Some(Column::Discrete(_)) => {
                let columns = parts
                    .iter()
                    .map(|part| match part {
                        Column::Discrete(column) => Ok(column),
                        Column::Continuous(_) => Err(mismatch()),
                    })
                    .collect::<Result<Vec<_>>>()?;
                Ok(Column::Discrete(DiscreteColumn::merge(&columns)))
            }
            None => Err(Error::InvalidData(format!("column '{name}' has no chunks"))),
        }
    }
}

/// Continuous values. Missing rows are Arrow nulls; NaN stays NaN.
#[derive(Debug, Clone, PartialEq)]
pub struct ContinuousColumn {
    values: Float64Array,
}

impl ContinuousColumn {
    /// Number of rows.
    pub fn len(&self) -> usize {
        self.values.len()
    }

    /// Whether the column has no rows.
    pub fn is_empty(&self) -> bool {
        self.values.is_empty()
    }

    /// Value of a row, `None` if null.
    pub fn get(&self, row: usize) -> Option<f64> {
        self.values.is_valid(row).then(|| self.values.value(row))
    }

    /// Raw values, including whatever sits in null slots. Pair with [`Self::is_valid`].
    pub fn values(&self) -> &[f64] {
        self.values.values()
    }

    /// Whether a row is non-null.
    pub fn is_valid(&self, row: usize) -> bool {
        self.values.is_valid(row)
    }

    /// Number of null rows.
    pub fn null_count(&self) -> usize {
        self.values.null_count()
    }
}

/// Discrete values as codes into an ordered list of levels.
#[derive(Debug, Clone, PartialEq)]
pub struct DiscreteColumn {
    levels: Arc<[String]>,
    codes: UInt32Array,
}

impl DiscreteColumn {
    /// Number of rows.
    pub fn len(&self) -> usize {
        self.codes.len()
    }

    /// Whether the column has no rows.
    pub fn is_empty(&self) -> bool {
        self.codes.is_empty()
    }

    /// Levels in their default order. Every level is used by at least one row.
    pub fn levels(&self) -> &[String] {
        &self.levels
    }

    /// Level index of a row, `None` if null.
    pub fn code(&self, row: usize) -> Option<u32> {
        self.codes.is_valid(row).then(|| self.codes.value(row))
    }

    /// Level label of a row, `None` if null.
    pub fn label(&self, row: usize) -> Option<&str> {
        self.code(row)
            .map(|code| self.levels[code as usize].as_str())
    }

    /// Number of null rows.
    pub fn null_count(&self) -> usize {
        self.codes.null_count()
    }

    fn from_strings<'a>(strings: impl Iterator<Item = Option<&'a str>>) -> Self {
        let mut level_index: HashMap<&str, u32> = HashMap::new();
        let mut levels: Vec<String> = Vec::new();
        let codes: UInt32Array = strings
            .map(|value| {
                value.map(|text| {
                    *level_index.entry(text).or_insert_with(|| {
                        levels.push(text.to_owned());
                        (levels.len() - 1) as u32
                    })
                })
            })
            .collect();
        DiscreteColumn {
            levels: levels.into(),
            codes,
        }
    }

    fn from_booleans(booleans: &BooleanArray) -> Self {
        let mut levels: Vec<String> = Vec::new();
        let mut true_code = None;
        let mut false_code = None;
        let codes: UInt32Array = booleans
            .iter()
            .map(|value| {
                value.map(|flag| {
                    let code = if flag {
                        &mut true_code
                    } else {
                        &mut false_code
                    };
                    *code.get_or_insert_with(|| {
                        levels.push(flag.to_string());
                        (levels.len() - 1) as u32
                    })
                })
            })
            .collect();
        DiscreteColumn {
            levels: levels.into(),
            codes,
        }
    }

    /// Union of levels in first-seen order, with codes remapped.
    fn merge(columns: &[&DiscreteColumn]) -> Self {
        let mut level_index: HashMap<&str, u32> = HashMap::new();
        let mut levels: Vec<String> = Vec::new();
        let remaps: Vec<Vec<u32>> = columns
            .iter()
            .map(|column| {
                column
                    .levels
                    .iter()
                    .map(|level| {
                        *level_index.entry(level.as_str()).or_insert_with(|| {
                            levels.push(level.clone());
                            (levels.len() - 1) as u32
                        })
                    })
                    .collect()
            })
            .collect();
        let codes: UInt32Array = columns
            .iter()
            .zip(&remaps)
            .flat_map(|(column, remap)| {
                column
                    .codes
                    .iter()
                    .map(move |code| code.map(|code| remap[code as usize]))
            })
            .collect();
        DiscreteColumn {
            levels: levels.into(),
            codes,
        }
    }

    /// Keep dictionary order, drop unused or null levels, and merge duplicates.
    fn from_dictionary(name: &str, keys: &UInt32Array, values: &[Option<String>]) -> Result<Self> {
        let mut used = vec![false; values.len()];
        for key in keys.iter().flatten() {
            let slot = used.get_mut(key as usize).ok_or_else(|| {
                Error::InvalidData(format!(
                    "column '{name}': dictionary key {key} out of range for {} values",
                    values.len()
                ))
            })?;
            *slot = true;
        }
        let mut level_index: HashMap<&str, u32> = HashMap::new();
        let mut levels: Vec<String> = Vec::new();
        let remap: Vec<Option<u32>> = values
            .iter()
            .zip(&used)
            .map(|(value, &is_used)| match value.as_deref() {
                Some(text) if is_used => Some(*level_index.entry(text).or_insert_with(|| {
                    levels.push(text.to_owned());
                    (levels.len() - 1) as u32
                })),
                _ => None,
            })
            .collect();
        let codes: UInt32Array = keys
            .iter()
            .map(|key| key.and_then(|key| remap[key as usize]))
            .collect();
        Ok(DiscreteColumn {
            levels: levels.into(),
            codes,
        })
    }
}

fn is_label_type(data_type: &DataType) -> bool {
    data_type.is_numeric()
        || matches!(
            data_type,
            DataType::Utf8 | DataType::LargeUtf8 | DataType::Utf8View | DataType::Boolean
        )
}

/// Dictionary keys as `u32`. Negative or oversized keys are an error, not nulls.
fn dictionary_keys(name: &str, keys: &dyn Array) -> Result<UInt32Array> {
    let converted = cast(name, keys, &DataType::UInt32)?;
    if converted.null_count() != keys.null_count() {
        return Err(Error::InvalidData(format!(
            "column '{name}': dictionary keys must be non-negative and fit in u32"
        )));
    }
    Ok(converted.as_primitive::<UInt32Type>().clone())
}

/// Dictionary values as labels. Strings are read in place; numbers and
/// booleans use Arrow's string formatting.
fn labels(name: &str, values: &dyn Array) -> Result<Vec<Option<String>>> {
    let owned = |iter: &mut dyn Iterator<Item = Option<&str>>| {
        iter.map(|value| value.map(str::to_owned)).collect()
    };
    Ok(match values.data_type() {
        DataType::Utf8 => owned(&mut values.as_string::<i32>().iter()),
        DataType::LargeUtf8 => owned(&mut values.as_string::<i64>().iter()),
        DataType::Utf8View => owned(&mut values.as_string_view().iter()),
        _ => {
            let strings = cast(name, values, &DataType::LargeUtf8)?;
            owned(&mut strings.as_string::<i64>().iter())
        }
    })
}

fn cast(name: &str, array: &dyn Array, to: &DataType) -> Result<ArrayRef> {
    arrow_cast::cast(array, to).map_err(|_| unsupported(name, array.data_type()))
}

fn unsupported(name: &str, data_type: &DataType) -> Error {
    Error::UnsupportedColumnType {
        name: name.to_owned(),
        data_type: data_type.to_string(),
    }
}

#[cfg(test)]
mod tests {
    use arrow_array::{Int8Array, Int64Array};

    use super::*;

    #[test]
    fn negative_or_oversized_dictionary_keys_are_an_error() {
        // Only reachable through FFI: safe Rust cannot build such a dictionary.
        for keys in [
            Arc::new(Int8Array::from(vec![-1, 0])) as ArrayRef,
            Arc::new(Int64Array::from(vec![i64::from(u32::MAX) + 1])),
        ] {
            let error = dictionary_keys("c", keys.as_ref()).unwrap_err().to_string();
            assert!(
                error.contains("dictionary keys must be non-negative"),
                "{error}"
            );
        }
    }

    #[test]
    fn null_dictionary_keys_stay_null() {
        let keys = Int8Array::from(vec![Some(0), None]);
        assert_eq!(dictionary_keys("c", &keys).unwrap().null_count(), 1);
    }
}

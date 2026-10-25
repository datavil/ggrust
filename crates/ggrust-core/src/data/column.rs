//! Typed columns: continuous (`f64`) or discrete (levels + codes).

use std::collections::HashMap;
use std::sync::Arc;

use arrow_array::cast::AsArray;
use arrow_array::types::UInt32Type;
use arrow_array::{Array, ArrayRef, Float64Array, StringArray, UInt32Array};
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
    /// - strings become levels in order of first appearance (as lets-plot does)
    /// - dictionaries (polars Categorical/Enum, pandas category) keep dictionary
    ///   order, dropping levels that no row uses (as lets-plot does for factor levels)
    /// - booleans become the levels `"true"`/`"false"` in order of first appearance
    /// - an all-null column becomes an all-missing continuous column
    pub(super) fn from_arrow(name: &str, array: &ArrayRef) -> Result<Self> {
        let data_type = array.data_type();
        match data_type {
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
            DataType::Utf8 | DataType::LargeUtf8 | DataType::Utf8View | DataType::Boolean => {
                let strings = cast(name, array, &DataType::Utf8)?;
                Ok(Column::Discrete(DiscreteColumn::from_strings(
                    strings.as_string::<i32>(),
                )))
            }
            DataType::Dictionary(_, value_type) if is_label_type(value_type) => {
                let dictionary = array.as_any_dictionary();
                let keys = cast(name, dictionary.keys(), &DataType::UInt32)?;
                let values = cast(name, dictionary.values(), &DataType::Utf8)?;
                DiscreteColumn::from_dictionary(
                    name,
                    keys.as_primitive::<UInt32Type>(),
                    values.as_string::<i32>(),
                )
                .map(Column::Discrete)
            }
            other => Err(unsupported(name, other)),
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

    fn from_strings(strings: &StringArray) -> Self {
        let mut level_index: HashMap<&str, u32> = HashMap::new();
        let mut levels: Vec<String> = Vec::new();
        let codes: UInt32Array = strings
            .iter()
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

    /// Keep dictionary order, drop unused or null levels, and merge duplicates.
    fn from_dictionary(name: &str, keys: &UInt32Array, values: &StringArray) -> Result<Self> {
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
            .map(|(value, &is_used)| match value {
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

fn cast(name: &str, array: &dyn Array, to: &DataType) -> Result<ArrayRef> {
    arrow_cast::cast(array, to).map_err(|_| unsupported(name, array.data_type()))
}

fn unsupported(name: &str, data_type: &DataType) -> Error {
    Error::UnsupportedColumnType {
        name: name.to_owned(),
        data_type: data_type.to_string(),
    }
}

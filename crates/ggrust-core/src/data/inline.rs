//! Data vectors given in a mapping (`aes(x=[1, 2, 3])`) become data columns, as
//! lets-plot's `ReplaceDataVectorsInAesMappingChange` does before building.
//!
//! Every aesthetic except `group` (whose list names columns) may hold a list. It
//! becomes a column named after the aesthetic, or the aesthetic plus 1, 2, ... when
//! that name is taken by any data column of the plot or an earlier vector. The
//! column joins the data of the spec (plot or layer) whose mapping held it.
//!
//! What it does not do: datetimes or mixed-type lists (numbers with strings).

use std::collections::BTreeSet;
use std::sync::Arc;

use arrow_array::{ArrayRef, BooleanArray, Float64Array, RecordBatch, StringArray};
use arrow_schema::{Field, Schema};

use super::DataFrame;
use crate::spec::{Aesthetic, Mapping, MappingValue, Value};
use crate::{Error, Result};

/// Whether a mapping value is a data vector rather than column names.
pub fn is_vector(aesthetic: Aesthetic, value: &MappingValue) -> bool {
    aesthetic != Aesthetic::Group
        && matches!(
            value,
            MappingValue::Columns(_) | MappingValue::Other(Value::Array(_))
        )
}

/// Replace the vectors of `mapping` by column names and add those columns to
/// `frame` (a new frame when there is none). `used_names` holds every data column
/// name of the plot and grows with the names given out. Returns the new frame, or
/// `None` when the mapping holds no vector.
pub fn inline_vectors(
    mapping: &mut Mapping,
    frame: Option<&DataFrame>,
    used_names: &mut BTreeSet<String>,
    path: &str,
) -> Result<Option<DataFrame>> {
    let mut frame = frame.cloned();
    let mut changed = false;
    for (aesthetic, value) in mapping.iter_mut() {
        if !is_vector(*aesthetic, value) {
            continue;
        }
        let at = format!("{path}.{aesthetic}");
        let array = vector_array(value, &at)?;
        let name = unused_name(aesthetic.as_str(), used_names);
        frame = Some(match frame {
            Some(frame) if array.len() != frame.row_count() => {
                return Err(Error::InvalidSpec(format!(
                    "at '{at}': the list has {} values but the data has {} rows",
                    array.len(),
                    frame.row_count()
                )));
            }
            Some(frame) => frame.with_column(&name, array)?,
            None => DataFrame::from_columns(vec![(name.clone(), array)])?,
        });
        *value = MappingValue::Column(name);
        changed = true;
    }
    Ok(frame.filter(|_| changed))
}

/// lets-plot `genVarName`: `base`, else `base1`, `base2`, ...
fn unused_name(base: &str, used_names: &mut BTreeSet<String>) -> String {
    let mut name = base.to_owned();
    let mut counter = 1;
    while used_names.contains(&name) {
        name = format!("{base}{counter}");
        counter += 1;
    }
    used_names.insert(name.clone());
    name
}

/// The values of a vector as one Arrow array: numbers (and nulls) as Float64,
/// texts as Utf8, booleans as Boolean. All nulls is a Float64 column.
fn vector_array(value: &MappingValue, path: &str) -> Result<ArrayRef> {
    let items: Vec<Value> = match value {
        MappingValue::Columns(texts) => texts.iter().cloned().map(Value::String).collect(),
        MappingValue::Other(Value::Array(items)) => items.clone(),
        _ => Vec::new(),
    };
    let present = || items.iter().filter(|item| !item.is_null());
    if present().all(Value::is_number) {
        return Ok(Arc::new(Float64Array::from(
            items.iter().map(Value::as_f64).collect::<Vec<_>>(),
        )));
    }
    if present().all(Value::is_string) {
        return Ok(Arc::new(StringArray::from(
            items.iter().map(Value::as_str).collect::<Vec<_>>(),
        )));
    }
    if present().all(Value::is_boolean) {
        return Ok(Arc::new(BooleanArray::from(
            items.iter().map(Value::as_bool).collect::<Vec<_>>(),
        )));
    }
    Err(Error::InvalidSpec(format!(
        "at '{path}': a list of values mixes types; expected only numbers, only \
         texts or only booleans (nulls allowed)"
    )))
}

impl DataFrame {
    /// A frame of one batch holding these columns.
    pub fn from_columns(columns: Vec<(String, ArrayRef)>) -> Result<DataFrame> {
        let fields: Vec<Field> = columns
            .iter()
            .map(|(name, array)| Field::new(name, array.data_type().clone(), true))
            .collect();
        let schema = Arc::new(Schema::new(fields));
        let arrays = columns.into_iter().map(|(_, array)| array).collect();
        let batch = RecordBatch::try_new(schema.clone(), arrays)
            .map_err(|error| Error::InvalidData(error.to_string()))?;
        DataFrame::new(schema, vec![batch])
    }

    /// This frame with one more column, which must have one value per row.
    /// Existing columns are shared, not copied.
    pub fn with_column(&self, name: &str, array: ArrayRef) -> Result<DataFrame> {
        if array.len() != self.row_count() {
            return Err(Error::InvalidData(format!(
                "have {} items but the data has {} rows",
                array.len(),
                self.row_count()
            )));
        }
        let mut fields: Vec<Field> = self
            .schema
            .fields()
            .iter()
            .map(|field| field.as_ref().clone())
            .collect();
        fields.push(Field::new(name, array.data_type().clone(), true));
        let schema = Arc::new(Schema::new(fields));
        let mut offset = 0;
        let mut batches = Vec::with_capacity(self.batches.len());
        for batch in &self.batches {
            let mut columns = batch.columns().to_vec();
            columns.push(array.slice(offset, batch.num_rows()));
            offset += batch.num_rows();
            batches.push(
                RecordBatch::try_new(schema.clone(), columns)
                    .map_err(|error| Error::InvalidData(error.to_string()))?,
            );
        }
        if batches.is_empty() {
            return DataFrame::new(schema, Vec::new());
        }
        DataFrame::new(schema, batches)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn mapping(json: &str) -> Mapping {
        serde_json::from_str(json).unwrap()
    }

    #[test]
    fn vectors_become_columns_named_after_the_aesthetic() {
        let mut used = BTreeSet::from(["x".to_owned()]);
        let mut mapping = mapping(r#"{"x": [1, 2, null], "y": "y", "color": ["a", "b", "a"]}"#);
        let frame = inline_vectors(&mut mapping, None, &mut used, "mapping")
            .unwrap()
            .unwrap();
        assert_eq!(mapping[&Aesthetic::X], MappingValue::Column("x1".into()));
        assert_eq!(mapping[&Aesthetic::Y], MappingValue::Column("y".into()));
        assert_eq!(
            mapping[&Aesthetic::Color],
            MappingValue::Column("color".into())
        );
        assert_eq!(frame.row_count(), 3);
        assert!(matches!(
            frame.column("x1").unwrap(),
            super::super::Column::Continuous(_)
        ));
        assert!(matches!(
            frame.column("color").unwrap(),
            super::super::Column::Discrete(_)
        ));
        assert!(used.contains("x1") && used.contains("color"));
    }

    #[test]
    fn group_lists_stay_column_names() {
        let mut mapping = mapping(r#"{"group": ["a", "b"]}"#);
        let frame = inline_vectors(&mut mapping, None, &mut BTreeSet::new(), "mapping").unwrap();
        assert!(frame.is_none());
        assert!(matches!(
            mapping[&Aesthetic::Group],
            MappingValue::Columns(_)
        ));
    }

    #[test]
    fn lengths_and_mixed_types_are_reported() {
        let frame = DataFrame::from_columns(vec![(
            "a".into(),
            Arc::new(Float64Array::from(vec![1.0, 2.0])) as ArrayRef,
        )])
        .unwrap();
        let mut short = mapping(r#"{"x": [1, 2, 3]}"#);
        let error = inline_vectors(&mut short, Some(&frame), &mut BTreeSet::new(), "mapping")
            .unwrap_err()
            .to_string();
        assert!(
            error.contains("'mapping.x'") && error.contains("3 values") && error.contains("2 rows"),
            "{error}"
        );
        let mut mixed = mapping(r#"{"x": [1, "a"]}"#);
        let error = inline_vectors(&mut mixed, None, &mut BTreeSet::new(), "mapping")
            .unwrap_err()
            .to_string();
        assert!(error.contains("mixes types"), "{error}");
    }
}

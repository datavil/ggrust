//! Plot data: Arrow record batches and the typed columns the engine reads.
//!
//! What it does: holds the frames that cross from Python (zero-copy Arrow) and
//! converts a column to a [`Column`] on demand, so only mapped columns are touched.
//!
//! What it does not do: decide scales or category order overrides. Ordering from
//! `as_discrete` / `factor_levels` is applied by the mapping stage on top of the
//! default order found here.

mod column;

use arrow_array::{ArrayRef, RecordBatch};
use arrow_schema::SchemaRef;

pub use column::{Column, ContinuousColumn, DiscreteColumn};

use crate::{Error, Result};

/// A table of named columns, stored as Arrow record batches (chunks).
#[derive(Debug, Clone)]
pub struct DataFrame {
    schema: SchemaRef,
    batches: Vec<RecordBatch>,
    row_count: usize,
}

impl DataFrame {
    /// Wrap record batches that share `schema`. No data is copied.
    pub fn new(schema: SchemaRef, batches: Vec<RecordBatch>) -> Result<Self> {
        if let Some(batch) = batches.iter().find(|batch| batch.schema() != schema) {
            return Err(Error::InvalidData(format!(
                "record batch schema {:?} does not match frame schema {:?}",
                batch.schema(),
                schema
            )));
        }
        let row_count = batches.iter().map(RecordBatch::num_rows).sum();
        Ok(DataFrame {
            schema,
            batches,
            row_count,
        })
    }

    /// Number of rows.
    pub fn row_count(&self) -> usize {
        self.row_count
    }

    /// Column names, in schema order.
    pub fn column_names(&self) -> impl Iterator<Item = &str> {
        self.schema
            .fields()
            .iter()
            .map(|field| field.name().as_str())
    }

    /// Whether a column exists.
    pub fn contains(&self, name: &str) -> bool {
        self.schema.index_of(name).is_ok()
    }

    /// Convert one column for plotting. Copies only when the column has several
    /// chunks or a type that needs casting (e.g. integers to `f64`).
    pub fn column(&self, name: &str) -> Result<Column> {
        let index = self
            .schema
            .index_of(name)
            .map_err(|_| Error::MissingColumn {
                name: name.to_owned(),
                available: self.column_names().collect::<Vec<_>>().join(", "),
            })?;
        let array = self.concatenated(index)?;
        Column::from_arrow(name, &array)
    }

    /// One column as a single Arrow array.
    fn concatenated(&self, index: usize) -> Result<ArrayRef> {
        let chunks: Vec<&dyn arrow_array::Array> = self
            .batches
            .iter()
            .map(|batch| batch.column(index).as_ref())
            .collect();
        match chunks.as_slice() {
            [] => Ok(arrow_array::new_empty_array(
                self.schema.field(index).data_type(),
            )),
            [_] => Ok(self.batches[0].column(index).clone()),
            _ => arrow_select::concat::concat(&chunks)
                .map_err(|error| Error::InvalidData(error.to_string())),
        }
    }
}

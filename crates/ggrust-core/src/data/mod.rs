//! Plot data: Arrow record batches and the typed columns the engine reads.
//!
//! What it does: holds the frames that cross from Python (zero-copy Arrow) and
//! converts a column to a [`Column`] on demand, so only mapped columns are touched.
//! Each converted column is cached on the frame, so layers, facets, tooltips and
//! the subplots of a `gggrid` that read the same column convert it once.
//!
//! What it does not do: decide scales or category order overrides. Ordering from
//! `as_discrete` / `factor_levels` is applied by the mapping stage on top of the
//! default order found here.

pub mod as_discrete;
mod column;
mod group;
pub mod inline;
mod layer;
mod resolution;

use std::sync::{Arc, OnceLock};

use arrow_array::{RecordBatch, new_empty_array};
use arrow_schema::{Schema, SchemaRef};

pub use column::{Column, ContinuousColumn, DiscreteColumn};
pub use group::group_index;
pub use layer::{Computed, LayerData, SourceRows, Variable};
pub use resolution::resolution;

use crate::{Error, Result};

/// A table of named columns, stored as Arrow record batches (chunks).
///
/// Clones share the batches and the column cache.
#[derive(Debug, Clone)]
pub struct DataFrame {
    schema: SchemaRef,
    batches: Vec<RecordBatch>,
    row_count: usize,
    /// Converted columns by schema index, filled on first read (`None`: the
    /// conversion failed). Columns are Arrow-backed, so handing out a cached one
    /// is a reference count bump.
    columns: Arc<[OnceLock<Option<Column>>]>,
    /// Every column name of the source data, when only some were sent (see
    /// [`Self::with_source_columns`]); empty: the schema's names.
    source_columns: Arc<[String]>,
}

impl DataFrame {
    /// Wrap record batches whose columns match `schema` by name and type.
    /// Metadata and nullability may differ. No data is copied.
    pub fn new(schema: SchemaRef, batches: Vec<RecordBatch>) -> Result<Self> {
        let expected = describe_fields(&schema);
        if let Some(batch) = batches
            .iter()
            .find(|batch| describe_fields(&batch.schema()) != expected)
        {
            return Err(Error::data(format!(
                "record batch columns [{}] do not match frame columns [{}]",
                describe_fields(&batch.schema()),
                expected
            )));
        }
        let row_count = batches.iter().map(RecordBatch::num_rows).sum();
        let columns = schema.fields().iter().map(|_| OnceLock::new()).collect();
        Ok(DataFrame {
            schema,
            batches,
            row_count,
            columns,
            source_columns: Arc::from([]),
        })
    }

    /// The same frame, knowing every column name of its source (Python sends only
    /// the referenced columns), so a missing column error lists them all.
    pub fn with_source_columns(mut self, names: &[String]) -> Self {
        self.source_columns = names.into();
        self
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

    /// Convert one column for plotting.
    ///
    /// Each chunk is converted on its own and the results are merged, so
    /// dictionary chunks with different dictionaries combine correctly.
    /// Float64 single-chunk columns are not copied. The result is cached: later
    /// reads of the same column return it without converting again, and threads
    /// reading it at the same time wait for one conversion. A failed conversion
    /// is cached as `None` and converted again to rebuild its error.
    pub fn column(&self, name: &str) -> Result<Column> {
        let index = self.index_of(name)?;
        match self.columns[index].get_or_init(|| self.convert(name, index).ok()) {
            Some(column) => Ok(column.clone()),
            None => self.convert(name, index),
        }
    }

    /// Convert the column at `index` from its Arrow chunks.
    fn convert(&self, name: &str, index: usize) -> Result<Column> {
        let parts = if self.batches.is_empty() {
            let data_type = self.schema.field(index).data_type();
            vec![Column::from_arrow(name, &new_empty_array(data_type))?]
        } else {
            self.batches
                .iter()
                .map(|batch| Column::from_arrow(name, batch.column(index)))
                .collect::<Result<Vec<_>>>()?
        };
        Column::concat(name, parts)
    }

    /// Index of a uniquely named column.
    fn index_of(&self, name: &str) -> Result<usize> {
        let mut matches = self
            .schema
            .fields()
            .iter()
            .enumerate()
            .filter(|(_, field)| field.name() == name)
            .map(|(index, _)| index);
        match (matches.next(), matches.count()) {
            (Some(index), 0) => Ok(index),
            (Some(_), duplicates) => Err(Error::data(format!(
                "column '{name}' appears {} times; column names must be unique",
                duplicates + 1
            ))),
            (None, _) => Err(Error::MissingColumn {
                name: name.to_owned(),
                available: if self.source_columns.is_empty() {
                    self.column_names().map(str::to_owned).collect()
                } else {
                    self.source_columns.iter().cloned().collect()
                },
                path: String::new(),
                notes: String::new(),
            }),
        }
    }
}

/// `name: type` for every field, used to compare and report schemas.
fn describe_fields(schema: &Schema) -> String {
    schema
        .fields()
        .iter()
        .map(|field| format!("{}: {}", field.name(), field.data_type()))
        .collect::<Vec<_>>()
        .join(", ")
}

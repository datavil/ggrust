//! The columns of one layer, keyed by aesthetic, as stats and positions see them.

use std::collections::BTreeMap;

use super::{Column, ContinuousColumn};
use crate::spec::Aesthetic;
use crate::{Error, Result};

/// Columns of one layer keyed by aesthetic. Every column has `row_count` rows.
#[derive(Debug, Clone, Default, PartialEq)]
pub struct LayerData {
    row_count: usize,
    columns: BTreeMap<Aesthetic, Column>,
}

impl LayerData {
    /// An empty layer with a fixed number of rows.
    pub fn new(row_count: usize) -> Self {
        LayerData {
            row_count,
            columns: BTreeMap::new(),
        }
    }

    /// Number of rows.
    pub fn row_count(&self) -> usize {
        self.row_count
    }

    /// Add or replace a column. Its length must equal [`Self::row_count`].
    pub fn insert(&mut self, aesthetic: Aesthetic, column: Column) -> Result<()> {
        if column.len() != self.row_count {
            return Err(Error::InvalidData(format!(
                "column for aesthetic '{aesthetic}' has {} rows, expected {}",
                column.len(),
                self.row_count
            )));
        }
        self.columns.insert(aesthetic, column);
        Ok(())
    }

    /// The column mapped to an aesthetic.
    pub fn get(&self, aesthetic: Aesthetic) -> Option<&Column> {
        self.columns.get(&aesthetic)
    }

    /// The column mapped to an aesthetic, if it is continuous.
    pub fn continuous(&self, aesthetic: Aesthetic) -> Option<&ContinuousColumn> {
        match self.columns.get(&aesthetic) {
            Some(Column::Continuous(column)) => Some(column),
            _ => None,
        }
    }

    /// Mapped aesthetics, in order.
    pub fn aesthetics(&self) -> impl Iterator<Item = Aesthetic> + '_ {
        self.columns.keys().copied()
    }

    /// Whether an aesthetic is mapped.
    pub fn contains(&self, aesthetic: Aesthetic) -> bool {
        self.columns.contains_key(&aesthetic)
    }
}

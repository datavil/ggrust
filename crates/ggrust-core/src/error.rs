//! Error type shared by the whole engine.
//!
//! Messages must say what was wrong, which parameter, and what is accepted.

/// Every fallible engine function returns this error.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// The plot specification is invalid.
    #[error("invalid plot specification: {0}")]
    InvalidSpec(String),
    /// A mapped column is not in the data.
    #[error("column '{name}' not found in data; available columns: {available}")]
    MissingColumn {
        /// The requested column.
        name: String,
        /// Comma-separated names of the columns that exist.
        available: String,
    },
    /// A column has an Arrow type ggrust cannot plot.
    #[error(
        "column '{name}' has unsupported type {data_type}; expected numbers, strings, booleans or categoricals"
    )]
    UnsupportedColumnType {
        /// The column.
        name: String,
        /// The Arrow data type, e.g. `Timestamp(ms)`.
        data_type: String,
    },
    /// Arrow data is malformed (e.g. batches with mismatched schemas).
    #[error("invalid data: {0}")]
    InvalidData(String),
}

/// Engine result alias.
pub type Result<T> = std::result::Result<T, Error>;

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn invalid_spec_message() {
        let error = Error::InvalidSpec("unknown geom 'pont'".into());
        assert_eq!(
            error.to_string(),
            "invalid plot specification: unknown geom 'pont'"
        );
    }
}

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
    /// Rendering failed (e.g. an impossible image size).
    #[error("render failed: {0}")]
    Render(String),
    /// Arrow data is malformed (e.g. batches with mismatched schemas).
    #[error("invalid data: {0}")]
    InvalidData(String),
}

impl Error {
    /// The same error with `notes` (e.g. warnings collected before it) appended to
    /// its message, so a failure explains what was ignored on the way.
    pub fn with_notes(self, notes: &[String]) -> Self {
        if notes.is_empty() {
            return self;
        }
        let suffix: String = notes.iter().map(|note| format!("\nnote: {note}")).collect();
        match self {
            Error::InvalidSpec(message) => Error::InvalidSpec(message + &suffix),
            Error::Render(message) => Error::Render(message + &suffix),
            Error::InvalidData(message) => Error::InvalidData(message + &suffix),
            other => Error::InvalidData(format!("{other}{suffix}")),
        }
    }
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

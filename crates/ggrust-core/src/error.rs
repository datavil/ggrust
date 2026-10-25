//! Error type shared by the whole engine.
//!
//! Messages must say what was wrong, which parameter, and what is accepted.

/// Every fallible engine function returns this error.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// The plot specification is invalid.
    #[error("invalid plot specification: {0}")]
    InvalidSpec(String),
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

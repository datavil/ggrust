//! Error type shared by the whole engine.
//!
//! Messages must say what was wrong, where (the spec path or column), what is
//! accepted, and how to fix it. Each kind maps to one Python exception class in the
//! bindings (`SpecError`, `ColumnNotFoundError`, ...), so the kinds are part of the API.

/// Every fallible engine function returns this error.
#[derive(Debug, thiserror::Error)]
pub enum Error {
    /// The plot specification is invalid.
    #[error("invalid plot specification{}: {message}{notes}", location(path))]
    InvalidSpec {
        /// Spec path of the offending value, e.g. `layers[0].size` (empty: the whole spec).
        path: String,
        /// What was wrong, what is accepted, and how to fix it.
        message: String,
        /// Notes appended by [`Error::with_notes`] (empty, or `\nnote: ...` lines).
        notes: String,
    },
    /// A mapped column is not in the data.
    #[error(
        "{}column '{name}' not found in data; available columns: {}{}{notes}",
        prefix(path),
        listing(available),
        crate::suggest::did_you_mean(name, available.iter().map(String::as_str))
    )]
    MissingColumn {
        /// The requested column.
        name: String,
        /// Names of the columns that exist (boxed: keeps `Error` small).
        available: Box<[String]>,
        /// Spec path of the mapping that names it, e.g. `layers[0].mapping.x` (empty
        /// when unknown).
        path: String,
        /// Notes appended by [`Error::with_notes`] (empty, or `\nnote: ...` lines).
        notes: String,
    },
    /// A column has an Arrow type ggrust cannot plot.
    #[error(
        "{}column '{name}' has unsupported type {data_type}; expected numbers, strings, booleans, categoricals, dates, times or durations; convert the column first (e.g. a list to a string){notes}",
        prefix(path)
    )]
    UnsupportedColumnType {
        /// The column.
        name: String,
        /// The Arrow data type, e.g. `Timestamp(ms)`.
        data_type: String,
        /// Spec path of the mapping that reads it (empty when unknown).
        path: String,
        /// Notes appended by [`Error::with_notes`] (empty, or `\nnote: ...` lines).
        notes: String,
    },
    /// Rendering failed (e.g. an impossible image size).
    #[error("render failed: {message}{notes}")]
    Render {
        /// What failed and how to avoid it.
        message: String,
        /// Notes appended by [`Error::with_notes`] (empty, or `\nnote: ...` lines).
        notes: String,
    },
    /// A font file or directory to register does not exist.
    #[error("font not found: '{path}': {message}{notes}")]
    FontNotFound {
        /// The path given.
        path: String,
        /// What is missing and how to fix it.
        message: String,
        /// Notes appended by [`Error::with_notes`] (empty, or `\nnote: ...` lines).
        notes: String,
    },
    /// A font file cannot be registered or embedded (not a font, unsupported, or
    /// its license forbids embedding).
    #[error("invalid font '{path}': {message}{notes}")]
    InvalidFont {
        /// The font file (or family, when embedding fails).
        path: String,
        /// What was wrong, what is accepted, and how to fix it.
        message: String,
        /// Notes appended by [`Error::with_notes`] (empty, or `\nnote: ...` lines).
        notes: String,
    },
    /// Arrow data is malformed (e.g. batches with mismatched schemas).
    #[error("invalid data: {message}{notes}")]
    InvalidData {
        /// What is wrong with the data, naming the column where there is one.
        message: String,
        /// Notes appended by [`Error::with_notes`] (empty, or `\nnote: ...` lines).
        notes: String,
    },
}

/// `" at '<path>'"`, or nothing for the whole spec.
fn location(path: &str) -> String {
    if path.is_empty() || path == "." {
        String::new()
    } else {
        format!(" at '{path}'")
    }
}

/// `"at '<path>': "`, or nothing when the path is unknown.
fn prefix(path: &str) -> String {
    if path.is_empty() {
        String::new()
    } else {
        format!("at '{path}': ")
    }
}

/// Column names for a message: all of them, or the first 30 and a count of the rest
/// (wide data such as genes in columns has thousands).
fn listing(names: &[String]) -> String {
    const SHOWN: usize = 30;
    if names.len() <= SHOWN {
        return names.join(", ");
    }
    format!(
        "{}, ... ({} more)",
        names[..SHOWN].join(", "),
        names.len() - SHOWN
    )
}

/// `outer` joined with `inner` (`figures[1]` + `layers[0]`, `scales` + `[2]`).
fn join(outer: &str, inner: &str) -> String {
    if inner.is_empty() || inner == "." {
        outer.to_owned()
    } else if inner.starts_with('[') {
        format!("{outer}{inner}")
    } else {
        format!("{outer}.{inner}")
    }
}

impl Error {
    /// An invalid value at spec `path` (e.g. `layers[0].size`; empty for the whole spec).
    pub fn spec(path: impl AsRef<str>, message: impl Into<String>) -> Self {
        Error::InvalidSpec {
            path: path.as_ref().to_owned(),
            message: message.into(),
            notes: String::new(),
        }
    }

    /// A spec error from a located text, `"at '<path>': <message>"` (as the tracked
    /// serde parsing reports them); text without a location applies to the whole spec.
    pub fn spec_located(text: String) -> Self {
        match text
            .strip_prefix("at '")
            .and_then(|rest| rest.split_once("': "))
        {
            Some((path, message)) => Error::spec(path, message),
            None => Error::spec("", text),
        }
    }

    /// Malformed data.
    pub fn data(message: impl Into<String>) -> Self {
        Error::InvalidData {
            message: message.into(),
            notes: String::new(),
        }
    }

    /// A font file or directory that does not exist.
    pub fn font_not_found(path: &std::path::Path, message: impl Into<String>) -> Self {
        Error::FontNotFound {
            path: path.display().to_string(),
            message: message.into(),
            notes: String::new(),
        }
    }

    /// A font that cannot be registered or embedded.
    pub fn invalid_font(path: &std::path::Path, message: impl Into<String>) -> Self {
        Error::InvalidFont {
            path: path.display().to_string(),
            message: message.into(),
            notes: String::new(),
        }
    }

    /// A rendering failure.
    pub fn render(message: impl Into<String>) -> Self {
        Error::Render {
            message: message.into(),
            notes: String::new(),
        }
    }

    /// The same error located under `outer` (a spec path such as `figures[1]`):
    /// errors with a path join it (`figures[1].layers[0]`), other kinds note the
    /// location.
    pub fn nested(mut self, outer: &str) -> Self {
        let outer = outer.trim_end_matches('.');
        if outer.is_empty() {
            return self;
        }
        match &mut self {
            Error::InvalidSpec { path, .. }
            | Error::MissingColumn { path, .. }
            | Error::UnsupportedColumnType { path, .. } => {
                *path = join(outer, path);
                self
            }
            Error::Render { .. }
            | Error::InvalidData { .. }
            | Error::FontNotFound { .. }
            | Error::InvalidFont { .. } => self.with_notes(&[format!("in '{outer}'")]),
        }
    }

    /// The same error with `notes` (e.g. warnings collected before it) appended to
    /// its message, so a failure explains what was ignored on the way.
    pub fn with_notes(mut self, added: &[String]) -> Self {
        if added.is_empty() {
            return self;
        }
        let suffix: String = added.iter().map(|note| format!("\nnote: {note}")).collect();
        // Every kind is kept, so callers can still match on it.
        match &mut self {
            Error::InvalidSpec { notes, .. }
            | Error::MissingColumn { notes, .. }
            | Error::UnsupportedColumnType { notes, .. }
            | Error::Render { notes, .. }
            | Error::InvalidData { notes, .. }
            | Error::FontNotFound { notes, .. }
            | Error::InvalidFont { notes, .. } => notes.push_str(&suffix),
        }
        self
    }

    /// The error's message without its kind prefix and notes (for warnings that
    /// quote it), e.g. `expected a number, got "a"`.
    pub fn reason(&self) -> String {
        match self {
            Error::InvalidSpec { path, message, .. } if !location(path).is_empty() => {
                format!("at '{path}': {message}")
            }
            Error::InvalidSpec { message, .. }
            | Error::Render { message, .. }
            | Error::InvalidData { message, .. } => message.clone(),
            Error::FontNotFound { path, message, .. }
            | Error::InvalidFont { path, message, .. } => {
                format!("'{path}': {message}")
            }
            Error::MissingColumn {
                name,
                available,
                path,
                ..
            } => format!(
                "{}column '{name}' not found in data; available columns: {}{}",
                prefix(path),
                listing(available),
                crate::suggest::did_you_mean(name, available.iter().map(String::as_str))
            ),
            Error::UnsupportedColumnType {
                name,
                data_type,
                path,
                ..
            } => format!(
                "{}column '{name}' has unsupported type {data_type}",
                prefix(path)
            ),
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
        let error = Error::spec("layers[0]", "unknown geom 'pont'; did you mean 'point'?");
        assert_eq!(
            error.to_string(),
            "invalid plot specification at 'layers[0]': unknown geom 'pont'; did you mean 'point'?"
        );
        assert_eq!(
            Error::spec("", "not valid JSON").to_string(),
            "invalid plot specification: not valid JSON"
        );
    }

    #[test]
    fn located_text_is_split_into_path_and_message() {
        let Error::InvalidSpec { path, message, .. } =
            Error::spec_located("at 'theme.axis_text.size': expected a number".into())
        else {
            panic!("expected InvalidSpec");
        };
        assert_eq!(path, "theme.axis_text.size");
        assert_eq!(message, "expected a number");
        let Error::InvalidSpec { path, .. } = Error::spec_located("no location".into()) else {
            panic!("expected InvalidSpec");
        };
        assert_eq!(path, "");
    }

    #[test]
    fn nested_joins_spec_paths_and_notes_other_kinds() {
        let error = Error::spec("layers[0].size", "bad").nested("figures[1]");
        assert!(
            matches!(&error, Error::InvalidSpec { path, .. } if path == "figures[1].layers[0].size")
        );
        let error = Error::spec("", "bad").nested("figures[1].");
        assert!(matches!(&error, Error::InvalidSpec { path, .. } if path == "figures[1]"));
        let error = Error::spec("[2]", "bad").nested("figures");
        assert!(matches!(&error, Error::InvalidSpec { path, .. } if path == "figures[2]"));
        let error = Error::render("too large").nested("figures[0]");
        assert_eq!(
            error.to_string(),
            "render failed: too large\nnote: in 'figures[0]'"
        );
    }

    #[test]
    fn missing_column_suggests_a_close_name() {
        let error = Error::MissingColumn {
            name: "sepal_lenght".into(),
            available: Box::new(["sepal_length".into(), "species".into()]),
            path: String::new(),
            notes: String::new(),
        };
        assert_eq!(
            error.to_string(),
            "column 'sepal_lenght' not found in data; available columns: sepal_length, \
             species; did you mean 'sepal_length'?"
        );
        let error = error.nested("mapping.x").nested("figures[0]");
        assert!(
            error
                .to_string()
                .starts_with("at 'figures[0].mapping.x': column 'sepal_lenght' not found in data")
        );
    }

    #[test]
    fn missing_column_listing_is_capped() {
        let available: Box<[String]> = (0..100).map(|index| format!("gene{index}")).collect();
        let message = Error::MissingColumn {
            name: "x".into(),
            available,
            path: String::new(),
            notes: String::new(),
        }
        .to_string();
        assert!(
            message.ends_with("gene28, gene29, ... (70 more)"),
            "{message}"
        );
    }

    #[test]
    fn notes_keep_the_error_kind() {
        let error = Error::MissingColumn {
            name: "x".into(),
            available: Box::new(["y".into()]),
            path: String::new(),
            notes: String::new(),
        }
        .with_notes(&["one".into()])
        .with_notes(&["two".into()]);
        assert!(matches!(error, Error::MissingColumn { .. }));
        assert_eq!(
            error.to_string(),
            "column 'x' not found in data; available columns: y\nnote: one\nnote: two"
        );
        let error = Error::spec("a", "b").with_notes(&["c".into()]);
        assert_eq!(
            error.to_string(),
            "invalid plot specification at 'a': b\nnote: c"
        );
    }
}

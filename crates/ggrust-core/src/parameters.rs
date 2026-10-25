//! Typed access to a layer's options (`size=2`, `color="red"`, `bins=30`).
//!
//! Errors name the path (`layers[0].size`) and the expected type.

use crate::scene::Color;
use crate::spec::{Options, Value};
use crate::{Error, Result};

/// A layer's options plus the path used in error messages.
#[derive(Debug, Clone, Copy)]
pub struct Parameters<'a> {
    options: &'a Options,
    path: &'a str,
}

impl<'a> Parameters<'a> {
    /// Wrap `options` found at `path` (e.g. `"layers[0]"`).
    pub fn new(options: &'a Options, path: &'a str) -> Self {
        Parameters { options, path }
    }

    /// Whether the option is present and not null.
    pub fn contains(&self, name: &str) -> bool {
        self.options.get(name).is_some_and(|value| !value.is_null())
    }

    /// Option names, in order.
    pub fn names(&self) -> impl Iterator<Item = &'a str> {
        self.options.keys().map(String::as_str)
    }

    /// A finite number. Null counts as absent.
    pub fn number(&self, name: &str) -> Result<Option<f64>> {
        match self.options.get(name) {
            None | Some(Value::Null) => Ok(None),
            Some(value) => value
                .as_f64()
                .filter(|number| number.is_finite())
                .map(Some)
                .ok_or_else(|| self.error(name, "a finite number", value)),
        }
    }

    /// A color string (see [`Color::parse`]). Null counts as absent.
    pub fn color(&self, name: &str) -> Result<Option<Color>> {
        match self.options.get(name) {
            None | Some(Value::Null) => Ok(None),
            Some(Value::String(text)) => Color::parse(text).map(Some).map_err(|message| {
                Error::InvalidSpec(format!("at '{}': {message}", self.at(name)))
            }),
            Some(value) => Err(self.error(name, "a color string", value)),
        }
    }

    /// A string. Null counts as absent.
    pub fn string(&self, name: &str) -> Result<Option<&'a str>> {
        match self.options.get(name) {
            None | Some(Value::Null) => Ok(None),
            Some(Value::String(text)) => Ok(Some(text)),
            Some(value) => Err(self.error(name, "a string", value)),
        }
    }

    /// A boolean. Null counts as absent.
    pub fn boolean(&self, name: &str) -> Result<Option<bool>> {
        match self.options.get(name) {
            None | Some(Value::Null) => Ok(None),
            Some(Value::Bool(flag)) => Ok(Some(*flag)),
            Some(value) => Err(self.error(name, "true or false", value)),
        }
    }

    fn at(&self, name: &str) -> String {
        if self.path.is_empty() {
            name.to_owned()
        } else {
            format!("{}.{name}", self.path)
        }
    }

    fn error(&self, name: &str, expected: &str, value: &Value) -> Error {
        Error::InvalidSpec(format!(
            "at '{}': expected {expected}, got {value}",
            self.at(name)
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn options(json: &str) -> Options {
        serde_json::from_str(json).unwrap()
    }

    #[test]
    fn typed_getters() {
        let options =
            options(r#"{"size": 2, "color": "red", "label": "a", "flag": true, "none": null}"#);
        let parameters = Parameters::new(&options, "layers[0]");
        assert_eq!(parameters.number("size").unwrap(), Some(2.0));
        assert_eq!(
            parameters.color("color").unwrap(),
            Some(Color::rgb(255, 0, 0))
        );
        assert_eq!(parameters.string("label").unwrap(), Some("a"));
        assert_eq!(parameters.boolean("flag").unwrap(), Some(true));
        assert_eq!(parameters.number("none").unwrap(), None);
        assert_eq!(parameters.number("missing").unwrap(), None);
        assert!(!parameters.contains("none"));
    }

    #[test]
    fn errors_name_the_path_and_type() {
        let options = options(r#"{"size": "big", "color": "reddish", "fill": 3}"#);
        let parameters = Parameters::new(&options, "layers[1]");
        assert_eq!(
            parameters.number("size").unwrap_err().to_string(),
            "invalid plot specification: at 'layers[1].size': expected a finite number, got \"big\""
        );
        assert!(
            parameters
                .color("color")
                .unwrap_err()
                .to_string()
                .contains("at 'layers[1].color': invalid color 'reddish'")
        );
        assert!(
            parameters
                .color("fill")
                .unwrap_err()
                .to_string()
                .contains("expected a color string, got 3")
        );
    }
}

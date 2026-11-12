//! Axis capping (ggplot2 `guide_axis(cap=)`): the axis line is cut back to the
//! outermost breaks, at both ends or one.
//!
//! As ggplot2, the ends are visual: `"upper"` is the right end of a horizontal
//! axis and the top end of a vertical one, also on a reversed scale. Only the
//! axis line is cut; ticks, labels and the grid are unchanged, and an axis
//! without a line (the default theme) shows no difference.
//!
//! What it does not do: re-cap the line while zoomed in HTML (the zoomed line
//! spans the panel again; resetting the zoom restores the capped line).

use crate::spec::Options;
use crate::{Error, Result};

/// Which ends of the axis line stop at the outermost break.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum AxisCap {
    /// The line spans the whole panel.
    #[default]
    None,
    /// The line runs between the outermost breaks.
    Both,
    /// The line stops at the last break on the right (top) end.
    Upper,
    /// The line stops at the first break on the left (bottom) end.
    Lower,
}

const ACCEPTED: [&str; 4] = ["none", "both", "upper", "lower"];

impl AxisCap {
    /// The options of a `guide_axis(...)`: `cap` as a name or a boolean (`True` is
    /// `"both"`, `False` is `"none"`, as ggplot2). Other options (`angle`,
    /// `n_dodge`, ...) are not read here: they are warned about as unsupported.
    pub fn parse(options: &Options, path: &str) -> Result<AxisCap> {
        let at = format!("{path}.cap");
        // Other ggplot2 options are reported as unsupported with the spec.
        let Some(value) = options.get("cap") else {
            return Ok(AxisCap::None);
        };
        Ok(match value {
            serde_json::Value::Null => AxisCap::None,
            serde_json::Value::Bool(true) => AxisCap::Both,
            serde_json::Value::Bool(false) => AxisCap::None,
            serde_json::Value::String(name) => match name.as_str() {
                "none" => AxisCap::None,
                "both" => AxisCap::Both,
                "upper" => AxisCap::Upper,
                "lower" => AxisCap::Lower,
                other => {
                    return Err(Error::spec(
                        at,
                        format!(
                            "unknown cap '{other}'; expected one of 'none', 'both', 'upper', \
                             'lower', or a boolean{}",
                            crate::suggest::did_you_mean(other, ACCEPTED)
                        ),
                    ));
                }
            },
            other => {
                return Err(Error::spec(
                    at,
                    format!(
                        "cap is one of 'none', 'both', 'upper', 'lower', or a boolean, got \
                         {other}"
                    ),
                ));
            }
        })
    }

    /// The pixel span (low, high) along the axis that the line covers: `span`
    /// (the panel) with the capped ends moved to the outermost of `breaks`
    /// (pixels along the axis) inside `span`. `None` when nothing is capped, or
    /// no break lies inside the panel (the line is drawn whole, as ggplot2 draws
    /// it when the key is empty).
    pub fn line_span(
        self,
        breaks: impl IntoIterator<Item = f64>,
        span: (f64, f64),
        horizontal: bool,
    ) -> Option<(f64, f64)> {
        if self == AxisCap::None {
            return None;
        }
        let (low, high) = (span.0.min(span.1), span.0.max(span.1));
        let tolerance = 1e-9 * (high - low).abs().max(1.0);
        let (first, last) = breaks
            .into_iter()
            .filter(|pixel| *pixel >= low - tolerance && *pixel <= high + tolerance)
            .fold(None, |range: Option<(f64, f64)>, pixel| {
                Some(range.map_or((pixel, pixel), |(first, last)| {
                    (first.min(pixel), last.max(pixel))
                }))
            })?;
        // Pixels grow right on a horizontal axis and down on a vertical one, so
        // the upper (top) end of a vertical axis is its smallest pixel.
        let (cap_low, cap_high) = match (self, horizontal) {
            (AxisCap::Both, _) => (true, true),
            (AxisCap::Upper, true) | (AxisCap::Lower, false) => (false, true),
            (AxisCap::Lower, true) | (AxisCap::Upper, false) => (true, false),
            (AxisCap::None, _) => (false, false),
        };
        Some((
            if cap_low { first } else { low },
            if cap_high { last } else { high },
        ))
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn parse(text: &str) -> Result<AxisCap> {
        let options: Options = serde_json::from_str(text).unwrap();
        AxisCap::parse(&options, "guides.x")
    }

    #[test]
    fn cap_takes_names_and_booleans() {
        assert_eq!(parse(r#"{"cap": "both"}"#).unwrap(), AxisCap::Both);
        assert_eq!(parse(r#"{"cap": true}"#).unwrap(), AxisCap::Both);
        assert_eq!(parse(r#"{"cap": false}"#).unwrap(), AxisCap::None);
        assert_eq!(parse(r#"{"cap": "lower"}"#).unwrap(), AxisCap::Lower);
        assert_eq!(parse("{}").unwrap(), AxisCap::None);
        let error = parse(r#"{"cap": "uper"}"#).unwrap_err().to_string();
        assert!(error.contains("guides.x.cap"), "{error}");
        assert!(error.contains("did you mean 'upper'"), "{error}");
        let error = parse(r#"{"cap": 1}"#).unwrap_err().to_string();
        assert!(error.contains("boolean, got 1"), "{error}");
        assert_eq!(parse(r#"{"angle": 45}"#).unwrap(), AxisCap::None);
    }

    #[test]
    fn ends_are_visual() {
        let breaks = [20.0, 50.0, 80.0, 120.0];
        let span = (10.0, 100.0);
        assert_eq!(AxisCap::None.line_span(breaks, span, true), None);
        assert_eq!(
            AxisCap::Both.line_span(breaks, span, true),
            Some((20.0, 80.0))
        );
        // Horizontal: upper is the right end.
        assert_eq!(
            AxisCap::Upper.line_span(breaks, span, true),
            Some((10.0, 80.0))
        );
        assert_eq!(
            AxisCap::Lower.line_span(breaks, span, true),
            Some((20.0, 100.0))
        );
        // Vertical: upper is the top (smallest pixel).
        assert_eq!(
            AxisCap::Upper.line_span(breaks, span, false),
            Some((20.0, 100.0))
        );
        assert_eq!(
            AxisCap::Lower.line_span(breaks, span, false),
            Some((10.0, 80.0))
        );
        assert_eq!(AxisCap::Both.line_span([150.0], span, true), None);
    }
}

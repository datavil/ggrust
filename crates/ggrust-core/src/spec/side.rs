//! Side panels: which band beside the main panel a layer draws in.
//!
//! Two spec forms reach the same bands:
//! - ggside layers (`geom_xside*()`, `geom_yside*()`): `"ggside": "x" | "y"` on the
//!   layer, placed by the plot's `ggside(x_pos=, y_pos=)` (`"ggside"` on the plot).
//!   The band has its own axis across it and the panel's theme.
//! - lets-plot `ggmarginal()`: lets-plot's own layer keys `marginal`,
//!   `margin_side` (`t`, `b`, `l`, `r`) and `margin_size` (share of the plotting
//!   area). The band has no axis, background or grid.
//!
//! What it does: parse and validate those keys into [`LayerSide`], in data terms
//! (the band shares data x or data y). What it does not do: lay bands out
//! (`layout::side_panel`) or prepare their layers (`build`).

use super::{LayerSpec, Value};
use crate::error::{Error, Result};
use crate::suggest::did_you_mean;

/// A band beside the main panel, in data terms.
#[derive(Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash)]
pub struct BandKey {
    /// The band shares the data x axis (an x side panel, top or bottom margin);
    /// else data y (a y side panel, left or right margin).
    pub shares_x: bool,
    /// The far side: top for an x band, right for a y band (before `coord_flip`).
    pub far: bool,
}

/// How a band looks and trains.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum BandStyle {
    /// ggside: the panel theme and an axis across the band; layers train the
    /// shared axis.
    Ggside,
    /// lets-plot `ggmarginal()`: no axis, background or grid; layers do not train
    /// the shared axis, and values grow away from the main panel.
    Marginal,
}

/// The side panel a layer draws in.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct LayerSide {
    /// The band.
    pub band: BandKey,
    /// ggside or marginal.
    pub style: BandStyle,
    /// `margin_size`: the band's share of the plotting area along its axis
    /// (marginal layers only; `None`: the default 0.1).
    pub size: Option<f64>,
}

/// The plot's `ggside(x_pos=, y_pos=)` options.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct SideOptions {
    /// The x side panel goes on top (ggside's default), else at the bottom.
    pub x_top: bool,
    /// The y side panel goes on the right (ggside's default), else on the left.
    pub y_right: bool,
}

impl Default for SideOptions {
    fn default() -> Self {
        SideOptions {
            x_top: true,
            y_right: true,
        }
    }
}

/// Smallest and largest lets-plot `margin_size`.
pub const MARGIN_SIZE_RANGE: (f64, f64) = (0.01, 0.95);

/// lets-plot's default `margin_size`.
pub const DEFAULT_MARGIN_SIZE: f64 = 0.1;

const SIDE_OPTION_KEYS: &[&str] = &["x_pos", "y_pos"];

impl SideOptions {
    /// Parse the plot's `"ggside"` value (path `ggside`).
    pub fn parse(value: Option<&Value>) -> Result<SideOptions> {
        let mut options = SideOptions::default();
        let Some(value) = value.filter(|value| !value.is_null()) else {
            return Ok(options);
        };
        let Value::Object(map) = value else {
            return Err(Error::spec(
                "ggside",
                format!("expected ggside(x_pos=..., y_pos=...) options, got {value}"),
            ));
        };
        for (key, item) in map {
            if item.is_null() || key == "name" {
                continue;
            }
            let path = format!("ggside.{key}");
            let accepted: &[&str] = match key.as_str() {
                "x_pos" => &["top", "bottom"],
                "y_pos" => &["right", "left"],
                _ => {
                    return Err(Error::spec(
                        path,
                        format!(
                            "unknown ggside option; accepted: 'x_pos', 'y_pos'{}",
                            did_you_mean(key, SIDE_OPTION_KEYS.iter().copied())
                        ),
                    ));
                }
            };
            let text = item.as_str().unwrap_or_default();
            if !accepted.contains(&text) {
                return Err(Error::spec(
                    path,
                    format!(
                        "expected '{}' or '{}', got {item}{}",
                        accepted[0],
                        accepted[1],
                        did_you_mean(text, accepted.iter().copied())
                    ),
                ));
            }
            match key.as_str() {
                "x_pos" => options.x_top = text == "top",
                _ => options.y_right = text == "right",
            }
        }
        Ok(options)
    }
}

/// The side panel `layer` (at `path`, e.g. `layers[2]`) draws in, `None` for the
/// main panel.
pub fn layer_side(
    layer: &LayerSpec,
    path: &str,
    options: SideOptions,
) -> Result<Option<LayerSide>> {
    let marginal = match &layer.marginal {
        None | Some(Value::Null) => false,
        Some(Value::Bool(flag)) => *flag,
        Some(other) => {
            return Err(Error::spec(
                format!("{path}.marginal"),
                format!("expected true or false, got {other}"),
            ));
        }
    };
    let ggside = match &layer.ggside {
        None | Some(Value::Null) => None,
        Some(value) => match value.as_str() {
            Some("x") => Some(true),
            Some("y") => Some(false),
            _ => {
                return Err(Error::spec(
                    format!("{path}.ggside"),
                    format!(
                        "expected 'x' (a geom_xside*() layer) or 'y' (geom_yside*()), got \
                         {value}"
                    ),
                ));
            }
        },
    };
    if marginal {
        if ggside.is_some() {
            return Err(Error::spec(
                format!("{path}.ggside"),
                "a layer is either a ggside layer or a ggmarginal() layer, not both; use \
                 one of them",
            ));
        }
        let side = layer
            .margin_side
            .as_ref()
            .and_then(Value::as_str)
            .unwrap_or_default();
        let band = match side {
            "t" => BandKey {
                shares_x: true,
                far: true,
            },
            "b" => BandKey {
                shares_x: true,
                far: false,
            },
            "r" => BandKey {
                shares_x: false,
                far: true,
            },
            "l" => BandKey {
                shares_x: false,
                far: false,
            },
            _ => {
                let got = layer
                    .margin_side
                    .as_ref()
                    .map_or_else(|| "nothing".to_owned(), std::string::ToString::to_string);
                return Err(Error::spec(
                    format!("{path}.margin_side"),
                    format!(
                        "expected one of 't', 'b', 'l', 'r' (top, bottom, left, right), got \
                         {got}"
                    ),
                ));
            }
        };
        let size = match &layer.margin_size {
            None | Some(Value::Null) => None,
            Some(value) => {
                let (low, high) = MARGIN_SIZE_RANGE;
                match value.as_f64().filter(|size| (low..=high).contains(size)) {
                    Some(size) => Some(size),
                    None => {
                        return Err(Error::spec(
                            format!("{path}.margin_size"),
                            format!(
                                "expected the margin's share of the plotting area in \
                                 [{low}, {high}], got {value}"
                            ),
                        ));
                    }
                }
            }
        };
        return Ok(Some(LayerSide {
            band,
            style: BandStyle::Marginal,
            size,
        }));
    }
    if let Some(key) = [
        ("margin_side", &layer.margin_side),
        ("margin_size", &layer.margin_size),
    ]
    .into_iter()
    .find(|(_, value)| value.as_ref().is_some_and(|value| !value.is_null()))
    .map(|(key, _)| key)
    {
        return Err(Error::spec(
            format!("{path}.{key}"),
            "only marginal layers take a margin; build them with ggmarginal()",
        ));
    }
    Ok(ggside.map(|shares_x| LayerSide {
        band: BandKey {
            shares_x,
            far: if shares_x {
                options.x_top
            } else {
                options.y_right
            },
        },
        style: BandStyle::Ggside,
        size: None,
    }))
}

#[cfg(test)]
mod tests {
    use super::*;
    use serde_json::json;

    fn layer(extra: Value) -> LayerSpec {
        let mut value = json!({"geom": "tile"});
        if let (Value::Object(base), Value::Object(extra)) = (&mut value, extra) {
            base.extend(extra);
        }
        serde_json::from_value(value).expect("layer parses")
    }

    #[test]
    fn main_layers_have_no_side() {
        let side = layer_side(&layer(json!({})), "layers[0]", SideOptions::default());
        assert_eq!(side.ok().flatten(), None);
    }

    #[test]
    fn ggside_layers_follow_the_plot_options() {
        let options = SideOptions::parse(Some(&json!({"x_pos": "bottom"}))).expect("valid");
        let side = layer_side(&layer(json!({"ggside": "x"})), "layers[0]", options)
            .expect("valid")
            .expect("a side layer");
        assert_eq!(
            side.band,
            BandKey {
                shares_x: true,
                far: false
            }
        );
        assert_eq!(side.style, BandStyle::Ggside);
        let side = layer_side(&layer(json!({"ggside": "y"})), "layers[0]", options)
            .expect("valid")
            .expect("a side layer");
        assert!(!side.band.shares_x && side.band.far);
    }

    #[test]
    fn marginal_layers_read_lets_plot_keys() {
        let spec = layer(json!({"marginal": true, "margin_side": "l", "margin_size": 0.2}));
        let side = layer_side(&spec, "layers[1]", SideOptions::default())
            .expect("valid")
            .expect("a side layer");
        assert_eq!(side.style, BandStyle::Marginal);
        assert!(!side.band.shares_x && !side.band.far);
        assert_eq!(side.size, Some(0.2));
    }

    #[test]
    fn errors_name_the_key_and_the_accepted_values() {
        let message = |spec: LayerSpec| {
            layer_side(&spec, "layers[3]", SideOptions::default())
                .expect_err("invalid")
                .to_string()
        };
        let error = message(layer(json!({"ggside": "z"})));
        assert!(
            error.contains("layers[3].ggside") && error.contains("'x'"),
            "{error}"
        );
        let error = message(layer(json!({"marginal": true, "margin_side": "top"})));
        assert!(
            error.contains("layers[3].margin_side") && error.contains("'t'"),
            "{error}"
        );
        let error = message(layer(
            json!({"marginal": true, "margin_side": "t", "margin_size": 2}),
        ));
        assert!(
            error.contains("margin_size") && error.contains("0.95"),
            "{error}"
        );
        let error = message(layer(json!({"margin_side": "t"})));
        assert!(error.contains("ggmarginal()"), "{error}");
        let error = SideOptions::parse(Some(&json!({"x_pos": "right"})))
            .expect_err("invalid")
            .to_string();
        assert!(
            error.contains("ggside.x_pos") && error.contains("'top'"),
            "{error}"
        );
        let error = SideOptions::parse(Some(&json!({"xpos": "top"})))
            .expect_err("invalid")
            .to_string();
        assert!(error.contains("did you mean 'x_pos'"), "{error}");
    }
}

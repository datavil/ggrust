//! `geom_tile`: one rectangle per row, centred on (x, y), `width` x `height` big.
//!
//! Setup turns the centre and size into `xmin`/`xmax`/`ymin`/`ymax` (lets-plot
//! `TileGeom` with `DimensionsUtil.dimensionSpan`): the size is in x/y resolutions
//! (`width_unit="res"`, the default) or in axis units (`"identity"`). The spans
//! train the axes, as lets-plot's width and height spans.
//!
//! Drawing follows lets-plot: rows in order, `alpha` on the fill only, outline
//! width 0 unless `size` is given.
//!
//! What it does not do (yet): `width_unit`/`height_unit` `"size"` and `"px"`.

use super::bar::{draw_key, draw_rectangles, key_size};
use super::{DrawLayer, Geom};
use crate::coord::Panel;
use crate::data::{Column, ContinuousColumn, LayerData};
use crate::parameters::Parameters;
use crate::scene::{Color, Node, Rect};
use crate::spec::Aesthetic;
use crate::style::{StyleDefault, Styles};
use crate::{Error, Result};

/// lets-plot tile style: pen fill, paper outline of width 0.
const STYLE_DEFAULTS: &[(Aesthetic, StyleDefault)] = &[
    (Aesthetic::Color, StyleDefault::Color(Color::WHITE)),
    (
        Aesthetic::Fill,
        StyleDefault::Color(Color::rgb(0x47, 0x47, 0x47)),
    ),
    (Aesthetic::Alpha, StyleDefault::Number(f64::NAN)),
    (Aesthetic::Size, StyleDefault::Number(0.0)),
    (Aesthetic::Linetype, StyleDefault::Linetype(&[])),
];

const RECTANGLE_AESTHETICS: &[Aesthetic] = &[
    Aesthetic::Xmin,
    Aesthetic::Xmax,
    Aesthetic::Ymin,
    Aesthetic::Ymax,
];

/// Tiles (heatmap cells).
pub struct TileGeom;

impl Geom for TileGeom {
    fn name(&self) -> &'static str {
        "tile"
    }

    fn required_aesthetics(&self) -> &'static [Aesthetic] {
        &[Aesthetic::X, Aesthetic::Y]
    }

    fn optional_aesthetics(&self) -> &'static [Aesthetic] {
        &[Aesthetic::Width, Aesthetic::Height]
    }

    fn style_defaults(&self) -> &'static [(Aesthetic, StyleDefault)] {
        STYLE_DEFAULTS
    }

    fn parameters(&self) -> &'static [&'static str] {
        &["width", "height", "width_unit", "height_unit"]
    }

    fn setup_aesthetics(&self) -> &'static [Aesthetic] {
        RECTANGLE_AESTHETICS
    }

    fn span_aesthetics(&self) -> &'static [Aesthetic] {
        RECTANGLE_AESTHETICS
    }

    fn setup(&self, mut data: LayerData, parameters: &Parameters) -> Result<LayerData> {
        let (Some(x), Some(y)) = (
            data.continuous(Aesthetic::X).cloned(),
            data.continuous(Aesthetic::Y).cloned(),
        ) else {
            return Ok(data);
        };
        let (xmin, xmax) = spans(&data, parameters, &x, Aesthetic::Width, "width")?;
        let (ymin, ymax) = spans(&data, parameters, &y, Aesthetic::Height, "height")?;
        let continuous = |values| Column::Continuous(ContinuousColumn::from_values(values));
        data.insert(Aesthetic::Xmin, continuous(xmin))?;
        data.insert(Aesthetic::Xmax, continuous(xmax))?;
        data.insert(Aesthetic::Ymin, continuous(ymin))?;
        data.insert(Aesthetic::Ymax, continuous(ymax))?;
        Ok(data)
    }

    fn draw(&self, layer: &DrawLayer, panel: &Panel) -> Result<Vec<Node>> {
        draw_rectangles(layer, panel, false)
    }

    fn minimum_key_size(&self, styles: &Styles) -> Result<(f64, f64)> {
        key_size(styles)
    }

    fn draw_key(&self, styles: &Styles, cell: Rect) -> Result<Vec<Node>> {
        draw_key(styles, cell)
    }
}

/// Low and high ends of each tile along one axis: `location` -/+ half the size
/// (mapped column, else the `name` option, else 1) in the option's unit.
fn spans(
    data: &LayerData,
    parameters: &Parameters,
    location: &ContinuousColumn,
    size: Aesthetic,
    name: &str,
) -> Result<(Vec<f64>, Vec<f64>)> {
    let unit_option = format!("{name}_unit");
    let axis = if size == Aesthetic::Width {
        Aesthetic::X
    } else {
        Aesthetic::Y
    };
    let unit = match parameters.string(&unit_option)?.unwrap_or("res") {
        "res" => data.resolution(axis),
        "identity" => 1.0,
        other => {
            return Err(Error::InvalidSpec(format!(
                "at '{}': {unit_option} '{other}' is not supported by ggrust yet; \
                 supported: res, identity",
                parameters.path_of(&unit_option)
            )));
        }
    };
    let mapped = match data.get(size) {
        Some(Column::Continuous(column)) => Some(column),
        Some(_) => {
            return Err(Error::InvalidData(format!(
                "tile '{size}' must be numeric; it was mapped to a discrete column"
            )));
        }
        None => None,
    };
    let constant = parameters.number(name)?.unwrap_or(1.0);
    let mut low = Vec::with_capacity(location.len());
    let mut high = Vec::with_capacity(location.len());
    for row in 0..location.len() {
        let center = location.get(row).unwrap_or(f64::NAN);
        let extent = mapped.map_or(constant, |column| column.get(row).unwrap_or(f64::NAN));
        let half = extent * unit / 2.0;
        low.push(center - half);
        high.push(center + half);
    }
    Ok((low, high))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::spec::Options;

    fn column(values: Vec<f64>) -> Column {
        Column::Continuous(ContinuousColumn::from_values(values))
    }

    #[test]
    fn tiles_span_the_resolution_or_axis_units() {
        let mut data = LayerData::new(3);
        data.insert(Aesthetic::X, column(vec![0.0, 2.0, 4.0]))
            .unwrap();
        data.insert(Aesthetic::Y, column(vec![1.0, 1.0, 1.0]))
            .unwrap();
        data.insert(Aesthetic::Height, column(vec![1.0, 2.0, 4.0]))
            .unwrap();
        let options: Options =
            serde_json::from_str(r#"{"width": 0.5, "height_unit": "identity"}"#).unwrap();
        let parameters = Parameters::new(&options, "layers[0]");
        let tiles = TileGeom.setup(data, &parameters).unwrap();
        let get = |aesthetic| tiles.continuous(aesthetic).unwrap().get(1).unwrap();
        // x resolution 2, width 0.5 -> 1 wide.
        assert_eq!((get(Aesthetic::Xmin), get(Aesthetic::Xmax)), (1.5, 2.5));
        // height in y units.
        assert_eq!((get(Aesthetic::Ymin), get(Aesthetic::Ymax)), (0.0, 2.0));
    }

    #[test]
    fn pixel_units_are_reported() {
        let mut data = LayerData::new(1);
        data.insert(Aesthetic::X, column(vec![0.0])).unwrap();
        data.insert(Aesthetic::Y, column(vec![0.0])).unwrap();
        let options: Options = serde_json::from_str(r#"{"width_unit": "px"}"#).unwrap();
        let parameters = Parameters::new(&options, "layers[0]");
        let error = TileGeom.setup(data, &parameters).unwrap_err().to_string();
        assert!(error.contains("width_unit 'px'"), "{error}");
    }
}

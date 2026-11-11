//! What a plot computed, for Python's `layer_data()` and `build()` (ggplot2's
//! `layer_data` and `ggplot_build`): every layer's data after its stat and position,
//! as Arrow, and every panel's trained x and y scales with the breaks drawn at the
//! plot's size. Read-only: it runs the normal preparation and layout and draws
//! nothing. Single plots only; a grid's figures are inspected one by one.

use std::collections::BTreeSet;
use std::sync::Arc;

use arrow_array::types::UInt32Type;
use arrow_array::{ArrayRef, DictionaryArray, Float64Array, RecordBatch, StringArray, UInt32Array};
use arrow_schema::{DataType, Field, Schema};

use super::{Arranged, PANEL_MIN_SIZE, PlotData, PreparedLayer, prepare_plot};
use crate::data::{Column, Variable};
use crate::layout::{AxisBreaks, PlotLayout};
use crate::scale::PositionScale;
use crate::scale::format::Label;
use crate::scene::{Color, Rect};
use crate::spec::{Aesthetic, PlotSpec};
use crate::style::{LineType, StyleValues};
use crate::theme::LegendPosition;
use crate::{Error, Result};

/// The computed data and scales of a plot.
#[derive(Debug, Clone)]
pub struct PlotInspection {
    /// One table per spec layer, in layer order: `panel`, `group`, then the
    /// layer's aesthetics and stat variables.
    pub layers: Vec<RecordBatch>,
    /// One entry per panel, in panel order (one for an unfaceted plot).
    pub panels: Vec<PanelScales>,
    /// The keys of every shown style guide, in the order the scales were met.
    pub guides: Vec<GuideKeys>,
    /// Build warnings (as rendering would give).
    pub warnings: Vec<String>,
}

/// The keys of one legend or colorbar (ggplot2's `get_guide_data()`).
#[derive(Debug, Clone, PartialEq)]
pub struct GuideKeys {
    /// The style aesthetic the guide shows.
    pub aesthetic: Aesthetic,
    /// The ggnewscale scale group (0 without `new_scale_*()`).
    pub group: u32,
    /// The guide title.
    pub title: String,
    /// Key labels as drawn (`10^6` for a superscript).
    pub labels: Vec<String>,
    /// The mapped value of each key.
    pub values: Vec<GuideValue>,
}

/// One key's mapped value.
#[derive(Debug, Clone, PartialEq)]
pub enum GuideValue {
    /// A color (`#rrggbb`, with `aa` appended when not opaque), line type or text.
    Text(String),
    /// A size, width or alpha.
    Number(f64),
    /// A shape code.
    Integer(i64),
}

/// The data-axis scales of one panel (`x` is the data x, also under `coord_flip`).
#[derive(Debug, Clone, PartialEq)]
pub struct PanelScales {
    /// The x scale.
    pub x: AxisScale,
    /// The y scale.
    pub y: AxisScale,
}

/// One trained position scale with the breaks drawn on its axis.
#[derive(Debug, Clone, PartialEq)]
pub struct AxisScale {
    /// Whether the axis shows categories (positions are then 0, 1, 2, ...).
    pub discrete: bool,
    /// The trained range after limits, before expansion, in transformed units.
    pub limits: (f64, f64),
    /// The categories in axis order; empty for a continuous scale.
    pub levels: Vec<String>,
    /// The range the panel spans, after limits and expansion, in transformed units.
    pub domain: (f64, f64),
    /// Break positions in transformed units.
    pub breaks: Vec<f64>,
    /// Break labels as drawn (`10^6` for a superscript), one per break.
    pub labels: Vec<String>,
    /// The secondary axis (`sec_axis=`): its breaks placed on this scale (in
    /// its transformed units) with their labels; `None` without one.
    pub secondary: Option<(Vec<f64>, Vec<String>)>,
}

/// Prepare and lay out `spec` at its own size, returning its computed layer data
/// and panel scales.
pub fn inspect_plot(spec: &PlotSpec, data: &PlotData) -> Result<PlotInspection> {
    let plot = prepare_plot(spec, data)?;
    let (width, height) = plot.size;
    let area = Rect {
        x: 0.0,
        y: 0.0,
        width,
        height,
    };
    let arrangement = plot.arrange(area, PANEL_MIN_SIZE)?;
    let layouts: Vec<&PlotLayout> = match &arrangement.layout {
        Arranged::Single(layout) => vec![layout],
        Arranged::Facets(layout) => layout.tiles.iter().map(|tile| &tile.layout).collect(),
    };
    let screen_breaks: Vec<(ScreenAxis, ScreenAxis)> = layouts
        .iter()
        .map(|layout| {
            let side = |index: usize| layout.secondary[index].as_ref().map(|side| &side.breaks);
            ((&layout.x_breaks, side(0)), (&layout.y_breaks, side(1)))
        })
        .collect();
    if screen_breaks.len() != plot.tiles.len() {
        return Err(Error::render(format!(
            "inspecting the plot found {} panels but {} laid out; please report this",
            plot.tiles.len(),
            screen_breaks.len()
        )));
    }
    let panels = plot
        .tiles
        .iter()
        .zip(&screen_breaks)
        .map(|((_, x, y), (horizontal, vertical))| {
            // Layout breaks are screen axes: under coord_flip the data x is vertical.
            let (x_breaks, y_breaks) = match plot.flipped {
                true => (*vertical, *horizontal),
                false => (*horizontal, *vertical),
            };
            PanelScales {
                x: axis_scale(x, x_breaks),
                y: axis_scale(y, y_breaks),
            }
        })
        .collect();
    let layers = (0..spec.layers.len())
        .map(|index| {
            let parts: Vec<(u32, &PreparedLayer)> = plot
                .tiles
                .iter()
                .enumerate()
                .filter_map(|(panel, (layers, _, _))| {
                    layers.get(index).map(|layer| (panel as u32, layer))
                })
                .collect();
            layer_table(&parts, &format!("layers[{index}]"))
        })
        .collect::<Result<_>>()?;
    // `legend_position="none"` draws no guide at all.
    let guides = match plot.theme.legend_position {
        LegendPosition::None => Vec::new(),
        _ => plot.style_guides.iter().map(guide_keys).collect(),
    };
    Ok(PlotInspection {
        layers,
        panels,
        guides,
        warnings: plot.warnings.clone(),
    })
}

fn guide_keys(guide: &super::StyleGuide) -> GuideKeys {
    let first = |values: &StyleValues| match values {
        StyleValues::Colors(colors) => GuideValue::Text(color_text(colors.get(0))),
        StyleValues::Numbers(numbers) => GuideValue::Number(numbers.get(0)),
        StyleValues::Shapes(shapes) => GuideValue::Integer(i64::from(shapes.get(0))),
        StyleValues::Linetypes(linetypes) => GuideValue::Text(dash_text(linetypes.get_ref(0))),
        StyleValues::Texts(texts) => GuideValue::Text(texts.get_ref(0).clone()),
    };
    GuideKeys {
        aesthetic: guide.key.aesthetic,
        group: guide.key.group,
        title: guide.title.clone(),
        labels: guide
            .entries
            .iter()
            .map(|entry| label_text(&entry.label))
            .collect(),
        values: guide
            .entries
            .iter()
            .map(|entry| first(&entry.value))
            .collect(),
    }
}

/// `solid`, or the dash pattern (`4 4`).
fn dash_text(linetype: &LineType) -> String {
    match linetype.0.is_empty() {
        true => "solid".to_owned(),
        false => linetype
            .0
            .iter()
            .map(f64::to_string)
            .collect::<Vec<_>>()
            .join(" "),
    }
}

/// `#rrggbb`, with the alpha appended when the color is not opaque.
fn color_text(color: Color) -> String {
    match color.alpha {
        255 => color.to_string(),
        alpha => format!("{color}{alpha:02x}"),
    }
}

/// A screen axis's own breaks and its secondary axis's.
type ScreenAxis<'a> = (&'a AxisBreaks, Option<&'a AxisBreaks>);

fn axis_scale(scale: &PositionScale, (breaks, secondary): ScreenAxis) -> AxisScale {
    AxisScale {
        discrete: scale.is_discrete(),
        limits: scale.limited_domain(),
        levels: scale.levels(),
        domain: scale.expanded_domain(),
        breaks: breaks.values.clone(),
        labels: label_texts(breaks),
        secondary: secondary.map(|secondary| (secondary.values.clone(), label_texts(secondary))),
    }
}

/// Labels as drawn (`10^6` for a superscript).
fn label_texts(breaks: &AxisBreaks) -> Vec<String> {
    breaks.labels.iter().map(label_text).collect()
}

/// A label as drawn (`10^6` for a superscript).
fn label_text(label: &Label) -> String {
    let mut text = label.text.clone();
    if let Some(superscript) = &label.superscript {
        text.push('^');
        text.push_str(superscript);
    }
    if let Some(suffix) = &label.suffix {
        text.push_str(suffix);
    }
    text
}

/// One column of the output table, gathered over panels.
enum Gathered {
    Numbers(Vec<Option<f64>>),
    /// Codes into `levels`.
    Categories(Vec<Option<u32>>, Vec<String>),
}

/// The rows of one layer over every panel as one table. Stat variables are named
/// without dots (`count`); one whose name an aesthetic already uses keeps its dots.
fn layer_table(parts: &[(u32, &PreparedLayer)], path: &str) -> Result<RecordBatch> {
    let row_count: usize = parts.iter().map(|(_, layer)| layer.data.row_count()).sum();
    let mut panel = Vec::with_capacity(row_count);
    let mut group = Vec::with_capacity(row_count);
    let mut names: Vec<String> = Vec::new();
    let mut columns: Vec<Gathered> = Vec::new();
    let mut variables: Vec<Variable> = Vec::new();
    for (_, layer) in parts {
        for (variable, _) in layer.data.columns() {
            let variable = oriented(variable, layer.y_oriented);
            if variable == Variable::Aesthetic(Aesthetic::Group) || variables.contains(&variable) {
                continue;
            }
            variables.push(variable);
        }
    }
    variables.sort();
    // A geom's default mapping copies stat variables into aesthetics of the same
    // name (`quantile`, `violinwidth`): keep one column when they hold the same.
    let duplicates: Vec<Variable> = variables
        .iter()
        .filter_map(|variable| {
            let Variable::Computed(computed) = variable else {
                return None;
            };
            let twin = variables.iter().find(|other| {
                matches!(other, Variable::Aesthetic(aesthetic) if aesthetic.as_str() == computed.as_str())
            })?;
            parts
                .iter()
                .all(|(_, layer)| {
                    layer.data.get(oriented(*twin, layer.y_oriented)) == layer.data.get(*variable)
                })
                .then_some(*variable)
        })
        .collect();
    variables.retain(|variable| !duplicates.contains(variable));
    let aesthetic_names: BTreeSet<&str> = variables
        .iter()
        .filter_map(|variable| match variable {
            Variable::Aesthetic(aesthetic) => Some(aesthetic.as_str()),
            Variable::Computed(_) => None,
        })
        .chain(["panel", "group"])
        .collect();
    for variable in &variables {
        names.push(match variable {
            Variable::Aesthetic(aesthetic) => aesthetic.as_str().to_owned(),
            Variable::Computed(computed) if aesthetic_names.contains(computed.as_str()) => {
                variable.to_string()
            }
            Variable::Computed(computed) => computed.as_str().to_owned(),
        });
        columns.push(Gathered::Numbers(Vec::with_capacity(row_count)));
    }
    for (index, variable) in variables.iter().enumerate() {
        let first_discrete = parts.iter().find_map(|(_, layer)| {
            match layer.data.get(oriented(*variable, layer.y_oriented)) {
                Some(Column::Discrete(_)) => Some(true),
                Some(Column::Continuous(_)) => Some(false),
                None => None,
            }
        });
        if first_discrete == Some(true) {
            columns[index] = Gathered::Categories(Vec::with_capacity(row_count), Vec::new());
        }
    }
    for (panel_index, layer) in parts {
        let data = &layer.data;
        let rows = data.row_count();
        panel.extend(std::iter::repeat_n(*panel_index, rows));
        group.extend((0..rows).map(|row| data.group_of(row)));
        for (variable, gathered) in variables.iter().zip(columns.iter_mut()) {
            let column = data.get(oriented(*variable, layer.y_oriented));
            match (gathered, column) {
                (Gathered::Numbers(values), Some(Column::Continuous(column))) => {
                    values.extend((0..rows).map(|row| column.get(row)));
                }
                (Gathered::Categories(codes, levels), Some(Column::Discrete(column))) => {
                    let remap: Vec<u32> = column
                        .levels()
                        .iter()
                        .map(
                            |level| match levels.iter().position(|known| known == level) {
                                Some(position) => position as u32,
                                None => {
                                    levels.push(level.clone());
                                    (levels.len() - 1) as u32
                                }
                            },
                        )
                        .collect();
                    codes.extend((0..rows).map(|row| {
                        column
                            .code(row)
                            .and_then(|code| remap.get(code as usize).copied())
                    }));
                }
                (Gathered::Numbers(values), None) => {
                    values.extend(std::iter::repeat_n(None, rows));
                }
                (Gathered::Categories(codes, _), None) => {
                    codes.extend(std::iter::repeat_n(None, rows));
                }
                _ => {
                    return Err(Error::render(format!(
                        "{path}: variable '{variable}' is numeric in one panel and \
                         categorical in another; please report this"
                    )));
                }
            }
        }
    }
    let mut fields = vec![
        Field::new("panel", DataType::UInt32, false),
        Field::new("group", DataType::UInt32, false),
    ];
    let mut arrays: Vec<ArrayRef> = vec![
        Arc::new(UInt32Array::from(panel)),
        Arc::new(UInt32Array::from(group)),
    ];
    for (name, gathered) in names.into_iter().zip(columns) {
        let array: ArrayRef = match gathered {
            Gathered::Numbers(values) => Arc::new(Float64Array::from(values)),
            Gathered::Categories(codes, levels) => {
                let keys = UInt32Array::from(codes);
                let values = Arc::new(StringArray::from(levels));
                Arc::new(
                    DictionaryArray::<UInt32Type>::try_new(keys, values)
                        .map_err(|error| Error::render(format!("{path}: {error}")))?,
                )
            }
        };
        fields.push(Field::new(name, array.data_type().clone(), true));
        arrays.push(array);
    }
    RecordBatch::try_new(Arc::new(Schema::new(fields)), arrays)
        .map_err(|error| Error::render(format!("{path}: {error}")))
}

/// A variable as the user named it: a y-oriented layer keeps x and y swapped.
fn oriented(variable: Variable, y_oriented: bool) -> Variable {
    match (variable, y_oriented) {
        (Variable::Aesthetic(aesthetic), true) => Variable::Aesthetic(aesthetic.flip_orientation()),
        _ => variable,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::data::DataFrame;
    use arrow_array::Array;

    fn frame(x: Vec<f64>, label: Vec<&str>) -> DataFrame {
        let schema = Arc::new(Schema::new(vec![
            Field::new("x", DataType::Float64, false),
            Field::new("label", DataType::Utf8, false),
        ]));
        let batch = RecordBatch::try_new(
            schema.clone(),
            vec![
                Arc::new(Float64Array::from(x)),
                Arc::new(StringArray::from(label)),
            ],
        )
        .unwrap();
        DataFrame::new(schema, vec![batch]).unwrap()
    }

    fn inspect(spec: &str, data: DataFrame) -> PlotInspection {
        let crate::spec::Figure::Plot(spec) = crate::spec::Figure::from_json(spec).unwrap() else {
            panic!("not a plot");
        };
        let data = PlotData {
            plot: Some(data),
            layers: Vec::new(),
        };
        inspect_plot(&spec, &data).unwrap()
    }

    #[test]
    fn histogram_counts_and_breaks() {
        let inspection = inspect(
            r#"{"kind": "plot", "data": {"frame": 0}, "mapping": {"x": "x"},
                "layers": [{"geom": "histogram", "bins": 2}]}"#,
            frame(vec![0.0, 1.0, 2.0, 3.0], vec!["a", "a", "b", "b"]),
        );
        let table = &inspection.layers[0];
        assert_eq!(table.num_rows(), 2);
        let names: Vec<&str> = table
            .schema_ref()
            .fields()
            .iter()
            .map(|field| field.name().as_str())
            .collect();
        assert!(names.starts_with(&["panel", "group"]));
        assert!(names.contains(&"count"));
        let count = table
            .column_by_name("count")
            .unwrap()
            .as_any()
            .downcast_ref::<Float64Array>()
            .unwrap();
        assert_eq!(count.values().iter().sum::<f64>(), 4.0);
        assert_eq!(inspection.panels.len(), 1);
        let x = &inspection.panels[0].x;
        assert!(!x.discrete);
        assert_eq!(x.breaks.len(), x.labels.len());
        assert!(!x.breaks.is_empty());
    }

    #[test]
    fn guides_and_scale_limits() {
        let inspection = inspect(
            r#"{"kind": "plot", "data": {"frame": 0},
                "mapping": {"x": "label", "y": "x", "color": "label"},
                "layers": [{"geom": "point"}]}"#,
            frame(vec![0.0, 1.0, 2.0], vec!["a", "b", "b"]),
        );
        let guide = &inspection.guides[0];
        assert_eq!(guide.aesthetic, Aesthetic::Color);
        assert_eq!(guide.labels, ["a", "b"]);
        assert!(matches!(&guide.values[0], GuideValue::Text(text) if text.starts_with('#')));
        let panel = &inspection.panels[0];
        assert_eq!(panel.x.levels, ["a", "b"]);
        assert_eq!(panel.x.limits, (0.0, 1.0));
        assert_eq!(panel.y.limits, (0.0, 2.0));
    }

    #[test]
    fn facets_give_one_panel_each_and_categories() {
        let inspection = inspect(
            r#"{"kind": "plot", "data": {"frame": 0},
                "mapping": {"x": "x", "y": "x", "color": "label"},
                "facet": {"name": "wrap", "facets": "label"},
                "layers": [{"geom": "point"}]}"#,
            frame(vec![0.0, 1.0, 2.0], vec!["a", "b", "b"]),
        );
        assert_eq!(inspection.panels.len(), 2);
        let table = &inspection.layers[0];
        let panel = table
            .column_by_name("panel")
            .unwrap()
            .as_any()
            .downcast_ref::<UInt32Array>()
            .unwrap();
        assert_eq!(panel.values().to_vec(), vec![0, 1, 1]);
        let color = table.column_by_name("color").unwrap();
        assert!(matches!(color.data_type(), DataType::Dictionary(_, _)));
    }
}

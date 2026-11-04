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
use crate::layout::AxisBreaks;
use crate::scale::PositionScale;
use crate::scene::Rect;
use crate::spec::{Aesthetic, PlotSpec};
use crate::{Error, Result};

/// The computed data and scales of a plot.
#[derive(Debug, Clone)]
pub struct PlotInspection {
    /// One table per spec layer, in layer order: `panel`, `group`, then the
    /// layer's aesthetics and stat variables.
    pub layers: Vec<RecordBatch>,
    /// One entry per panel, in panel order (one for an unfaceted plot).
    pub panels: Vec<PanelScales>,
    /// Build warnings (as rendering would give).
    pub warnings: Vec<String>,
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
    /// The range the panel spans, after limits and expansion, in transformed units.
    pub domain: (f64, f64),
    /// Break positions in transformed units.
    pub breaks: Vec<f64>,
    /// Break labels as drawn (`10^6` for a superscript), one per break.
    pub labels: Vec<String>,
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
    let screen_breaks: Vec<(&AxisBreaks, &AxisBreaks)> = match &arrangement.layout {
        Arranged::Single(layout) => vec![(&layout.x_breaks, &layout.y_breaks)],
        Arranged::Facets(layout) => layout
            .tiles
            .iter()
            .map(|tile| (&tile.layout.x_breaks, &tile.layout.y_breaks))
            .collect(),
    };
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
    Ok(PlotInspection {
        layers,
        panels,
        warnings: plot.warnings.clone(),
    })
}

fn axis_scale(scale: &PositionScale, breaks: &AxisBreaks) -> AxisScale {
    AxisScale {
        discrete: scale.is_discrete(),
        domain: scale.expanded_domain(),
        breaks: breaks.values.clone(),
        labels: breaks
            .labels
            .iter()
            .map(|label| {
                let mut text = label.text.clone();
                if let Some(superscript) = &label.superscript {
                    text.push('^');
                    text.push_str(superscript);
                }
                if let Some(suffix) = &label.suffix {
                    text.push_str(suffix);
                }
                text
            })
            .collect(),
    }
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

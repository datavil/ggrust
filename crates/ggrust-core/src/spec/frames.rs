//! Where a figure's data lives and which of its columns the engine reads.
//!
//! The Python bridge replaces every `data` object with a frame reference
//! (`{"frame": i, "columns": [...]}`, deduplicated by identity) and sends the
//! frames as Arrow. `columns` names every column of the source data, exported or
//! not, so data vectors in mappings are named as lets-plot names them.
//! [`Figure::referenced_columns`] is the single list of columns worth exporting.
//!
//! What it does not do: resolve references to frames (see `build::PlotData`).

use std::collections::BTreeSet;

use serde::Deserialize;

use super::{DataMeta, Figure, LayerSpec, Mapping, MappingValue, PlotSpec, Value};

/// A `data` key of a plot or layer.
#[derive(Debug, Clone, PartialEq, Eq, Deserialize)]
#[serde(try_from = "Value")]
pub enum DataSource {
    /// `{"frame": i, "columns": [...]}`: the i-th Arrow frame sent next to the
    /// spec, and the names of all columns of its source (`columns` is optional;
    /// only the referenced columns are sent as Arrow).
    Frame {
        /// Index into the frames.
        index: usize,
        /// Every column name of the source data.
        columns: Vec<String>,
    },
    /// Inline data as lets_plot writes it (e.g. a frame's text in `as_dict()`).
    /// ggrust's Python API never sends it; it is skipped.
    Inline,
}

impl TryFrom<Value> for DataSource {
    type Error = String;

    /// A reference is an object with a number at `frame` and, optionally, a list of
    /// names at `columns`, and no other key. Anything else is inline data (an inline
    /// frame may well have a column named `frame`, but its values are a list).
    fn try_from(value: Value) -> std::result::Result<Self, Self::Error> {
        let reference = value.as_object().filter(|object| {
            object.get("frame").is_some_and(Value::is_number)
                && object.keys().all(|key| key == "frame" || key == "columns")
        });
        let Some(object) = reference else {
            return Ok(DataSource::Inline);
        };
        let frame = &object["frame"];
        let index = frame
            .as_u64()
            .and_then(|index| usize::try_from(index).ok())
            .ok_or_else(|| {
                format!("a frame reference needs a non-negative integer index, got {frame}")
            })?;
        let columns = match object.get("columns") {
            None => Vec::new(),
            Some(Value::Array(names)) => names
                .iter()
                .map(|name| name.as_str().map(str::to_owned))
                .collect::<Option<Vec<_>>>()
                .ok_or_else(|| {
                    format!(
                        "a frame reference's columns must be texts, got {}",
                        Value::Array(names.clone())
                    )
                })?,
            Some(other) => {
                return Err(format!(
                    "a frame reference's columns must be a list of texts, got {other}"
                ));
            }
        };
        Ok(DataSource::Frame { index, columns })
    }
}

impl Figure {
    /// Every column name the engine reads: mappings (plot and layers, group lists),
    /// `as_discrete(order_by=...)` and `facet_wrap` columns, nested figures
    /// included. Features that are parsed but not built yet (tooltips, `facet_grid`)
    /// add their columns when they land: exporting a column nothing reads can only
    /// fail the plot.
    pub fn referenced_columns(&self) -> BTreeSet<String> {
        let mut columns = BTreeSet::new();
        self.collect_columns(&mut columns);
        columns
    }

    fn collect_columns(&self, columns: &mut BTreeSet<String>) {
        match self {
            Figure::Plot(plot) => plot.collect_columns(columns),
            Figure::Subplots(subplots) => {
                for figure in subplots.figures.iter().flatten() {
                    figure.collect_columns(columns);
                }
            }
        }
    }
}

impl PlotSpec {
    fn collect_columns(&self, columns: &mut BTreeSet<String>) {
        add_mapping(&self.mapping, columns);
        add_meta(&self.data_meta, columns);
        if let Some(facet) = self.facet.as_ref().filter(|facet| facet.name == "wrap") {
            columns.extend(
                facet
                    .facets
                    .iter()
                    .flat_map(|facets| facets.as_slice())
                    .cloned(),
            );
        }
        for layer in &self.layers {
            add_layer(layer, columns);
        }
    }
}

fn add_layer(layer: &LayerSpec, columns: &mut BTreeSet<String>) {
    add_mapping(&layer.mapping, columns);
    add_meta(&layer.data_meta, columns);
}

fn add_mapping(mapping: &Mapping, columns: &mut BTreeSet<String>) {
    for (aesthetic, value) in mapping {
        if crate::data::inline::is_vector(*aesthetic, value) {
            // A data vector (`aes(x=[1, 2])`) names no column; the engine avoids
            // every name of the frame references' `columns` when naming it.
            continue;
        }
        match value {
            MappingValue::Column(name) => {
                columns.insert(name.clone());
            }
            MappingValue::Columns(names) => columns.extend(names.iter().cloned()),
            MappingValue::Other(_) => {}
        }
    }
}

fn add_meta(meta: &DataMeta, columns: &mut BTreeSet<String>) {
    for annotation in &meta.mapping_annotations {
        if let Some(order_by) = &annotation.parameters.order_by {
            columns.insert(order_by.clone());
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn columns(json: &str) -> Vec<String> {
        Figure::from_json(json)
            .unwrap()
            .referenced_columns()
            .into_iter()
            .collect()
    }

    #[test]
    fn frame_references_carry_column_names_and_vectors_reference_nothing() {
        let Figure::Plot(plot) = Figure::from_json(
            r#"{"kind": "plot", "data": {"frame": 0, "columns": ["a", "x"]},
                "mapping": {"x": [1, 2], "y": "a", "group": ["g", "h"]}}"#,
        )
        .unwrap() else {
            panic!("expected a plot");
        };
        assert_eq!(
            plot.data,
            Some(DataSource::Frame {
                index: 0,
                columns: vec!["a".into(), "x".into()]
            })
        );
        assert_eq!(
            columns(r#"{"kind": "plot", "mapping": {"x": ["p", "q"], "y": "a", "group": ["g"]}}"#),
            ["a", "g"]
        );
        let error = Figure::from_json(r#"{"kind": "plot", "data": {"frame": 0, "columns": [1]}}"#)
            .err()
            .unwrap()
            .to_string();
        assert!(error.contains("columns must be texts"), "{error}");
    }

    #[test]
    fn frame_references_parse_and_inline_data_is_skipped() {
        let Figure::Plot(plot) = Figure::from_json(
            r#"{"kind": "plot", "data": {"frame": 2},
                "layers": [{"geom": "point", "data": "shape: (4, 5) ..."}]}"#,
        )
        .unwrap() else {
            panic!("expected a plot");
        };
        assert_eq!(
            plot.data,
            Some(DataSource::Frame {
                index: 2,
                columns: Vec::new()
            })
        );
        assert_eq!(plot.layers[0].data, Some(DataSource::Inline));
        let error = Figure::from_json(r#"{"kind": "plot", "data": {"frame": -1}}"#)
            .unwrap_err()
            .to_string();
        assert!(
            error.contains("at 'data'") && error.contains("non-negative integer"),
            "{error}"
        );
        // lets_plot inline data with a column named `frame` is still inline data.
        for inline in [r#"{"frame": [0, 1], "x": [1, 2]}"#, r#"{"frame": [0, 1]}"#] {
            let Figure::Plot(plot) =
                Figure::from_json(&format!(r#"{{"kind": "plot", "data": {inline}}}"#)).unwrap()
            else {
                panic!("expected a plot");
            };
            assert_eq!(plot.data, Some(DataSource::Inline), "{inline}");
        }
    }

    #[test]
    fn columns_come_from_mappings_and_annotations_only() {
        let found = columns(
            r#"{"kind": "subplots", "layout": {"name": "grid", "ncol": 3, "nrow": 1},
                "figures": [
                  {"kind": "plot", "mapping": {"x": "UMAP1", "y": "UMAP2"},
                   "facet": {"name": "wrap", "facets": ["sample"]},
                   "layers": [{"geom": "point",
                               "mapping": {"color": "cluster", "group": ["batch", "donor"]},
                               "data_meta": {"mapping_annotations": [{"aes": "color",
                                   "annotation": "as_discrete",
                                   "parameters": {"order_by": "rank"}}]},
                               "tooltips": {"lines": ["@gene"]}}]},
                  null,
                  {"kind": "plot", "mapping": {"x": "other"}}
                ]}"#,
        );
        // Tooltips are not built yet, so their columns are not read.
        assert_eq!(
            found,
            [
                "UMAP1", "UMAP2", "batch", "cluster", "donor", "other", "rank", "sample"
            ]
        );
    }
}

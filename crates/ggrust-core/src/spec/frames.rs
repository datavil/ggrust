//! Where a figure's data lives and which of its columns the engine reads.
//!
//! The Python bridge replaces every `data` object with a frame reference
//! (`{"frame": i}`, deduplicated by identity) and sends the frames as Arrow.
//! [`Figure::referenced_columns`] is the single list of columns worth exporting.
//!
//! What it does not do: resolve references to frames (see `build::PlotData`).

use std::collections::BTreeSet;

use serde::Deserialize;

use super::{DataMeta, Figure, LayerSpec, Mapping, MappingValue, PlotSpec, Value};

/// A `data` key of a plot or layer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Deserialize)]
#[serde(try_from = "Value")]
pub enum DataSource {
    /// `{"frame": i}`: the i-th Arrow frame sent next to the spec.
    Frame(usize),
    /// Inline data as lets_plot writes it (e.g. a frame's text in `as_dict()`).
    /// ggrust's Python API never sends it; it is skipped.
    Inline,
}

impl TryFrom<Value> for DataSource {
    type Error = String;

    /// A reference is an object whose only key is `frame` with a number. Anything
    /// else is inline data (an inline frame may well have a column named `frame`,
    /// but its values are a list).
    fn try_from(value: Value) -> std::result::Result<Self, Self::Error> {
        let reference = value
            .as_object()
            .filter(|object| object.len() == 1)
            .and_then(|object| object.get("frame"))
            .filter(|frame| frame.is_number());
        let Some(frame) = reference else {
            return Ok(DataSource::Inline);
        };
        frame
            .as_u64()
            .and_then(|index| usize::try_from(index).ok())
            .map(DataSource::Frame)
            .ok_or_else(|| {
                format!("a frame reference needs a non-negative integer index, got {frame}")
            })
    }
}

impl Figure {
    /// Every column name the engine reads: mappings (plot and layers, group lists)
    /// and `as_discrete(order_by=...)`, nested figures included. Features that are
    /// parsed but not built yet (tooltips, facets) add their columns when they land:
    /// exporting a column nothing reads can only fail the plot.
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
    for value in mapping.values() {
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
    fn frame_references_parse_and_inline_data_is_skipped() {
        let Figure::Plot(plot) = Figure::from_json(
            r#"{"kind": "plot", "data": {"frame": 2},
                "layers": [{"geom": "point", "data": "shape: (4, 5) ..."}]}"#,
        )
        .unwrap() else {
            panic!("expected a plot");
        };
        assert_eq!(plot.data, Some(DataSource::Frame(2)));
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
        // Facets and tooltips are not built yet, so their columns are not read.
        assert_eq!(
            found,
            [
                "UMAP1", "UMAP2", "batch", "cluster", "donor", "other", "rank"
            ]
        );
    }
}

//! Where a figure's data lives and which of its columns the engine reads.
//!
//! The Python bridge replaces every `data` object with a frame reference
//! (`{"frame": i}`, deduplicated by identity) and sends the frames as Arrow.
//! [`Figure::referenced_columns`] is the single list of columns worth exporting.
//!
//! What it does not do: resolve references to frames (see `build::PlotData`).

use std::collections::BTreeSet;

use serde::Deserialize;

use super::{
    DataMeta, FacetSpec, Figure, LayerSpec, Mapping, MappingValue, PlotSpec, TooltipsSpec, Value,
};

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

    fn try_from(value: Value) -> std::result::Result<Self, Self::Error> {
        let Some(object) = value
            .as_object()
            .filter(|object| object.contains_key("frame"))
        else {
            return Ok(DataSource::Inline);
        };
        match (object.len(), object["frame"].as_u64()) {
            (1, Some(index)) => usize::try_from(index)
                .map(DataSource::Frame)
                .map_err(|_| format!("at 'frame': index {index} is too large")),
            _ => Err(format!(
                "expected a frame reference {{\"frame\": <non-negative integer>}}, got {value}"
            )),
        }
    }
}

impl Figure {
    /// Every column name the figure can read: mappings (plot and layers, group
    /// lists), `as_discrete(order_by=...)`, facet variables and tooltip fields.
    /// Nested figures are included.
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
        if let Some(facet) = &self.facet {
            add_facet(facet, columns);
        }
        for layer in &self.layers {
            add_layer(layer, columns);
        }
    }
}

fn add_layer(layer: &LayerSpec, columns: &mut BTreeSet<String>) {
    add_mapping(&layer.mapping, columns);
    add_meta(&layer.data_meta, columns);
    for tooltips in [&layer.tooltips, &layer.labels].into_iter().flatten() {
        let TooltipsSpec::Lines(lines) = tooltips else {
            continue;
        };
        let templates = lines.lines.iter().flatten().chain(&lines.title);
        for template in templates {
            columns.extend(template_fields(template));
        }
        for format in &lines.formats {
            columns.extend(template_fields(&format.field));
        }
        let variables = lines.variables.iter().flatten();
        columns.extend(variables.filter(|name| !is_stat_variable(name)).cloned());
    }
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

fn add_facet(facet: &FacetSpec, columns: &mut BTreeSet<String>) {
    columns.extend(
        facet
            .facets
            .iter()
            .flat_map(|facets| facets.as_slice())
            .cloned(),
    );
    columns.extend(facet.x.iter().chain(&facet.y).cloned());
}

/// `..count..`: a stat variable, not a data column.
fn is_stat_variable(name: &str) -> bool {
    name.len() > 4 && name.starts_with("..") && name.ends_with("..")
}

/// Column names referenced by a lets-plot tooltip template: `@name` (letters,
/// digits, `_`, `.`), `@{any name}`. `\@` is a literal `@`; `@..stat..` and
/// `^aesthetic` are not columns.
fn template_fields(template: &str) -> Vec<String> {
    let mut fields = Vec::new();
    let mut rest = template;
    while let Some(at) = rest.find('@') {
        let escaped = rest[..at].ends_with('\\');
        rest = &rest[at + 1..];
        if escaped {
            continue;
        }
        let name = if let Some(braced) = rest.strip_prefix('{') {
            let Some(end) = braced.find('}') else {
                break;
            };
            rest = &braced[end + 1..];
            &braced[..end]
        } else {
            let end = rest
                .find(|character: char| !(character.is_alphanumeric() || "_.".contains(character)))
                .unwrap_or(rest.len());
            let name = &rest[..end];
            rest = &rest[end..];
            name
        };
        if !name.is_empty() && !is_stat_variable(name) {
            fields.push(name.to_owned());
        }
    }
    fields
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
            error.contains("at 'data'") && error.contains("frame reference"),
            "{error}"
        );
    }

    #[test]
    fn columns_come_from_mappings_annotations_facets_and_tooltips() {
        let found = columns(
            r#"{"kind": "subplots", "layout": {"name": "grid", "ncol": 2, "nrow": 1},
                "figures": [
                  {"kind": "plot", "mapping": {"x": "UMAP1", "y": "UMAP2"},
                   "facet": {"name": "wrap", "facets": ["sample"]},
                   "layers": [{"geom": "point",
                               "mapping": {"color": "cluster", "group": ["batch", "donor"]},
                               "data_meta": {"mapping_annotations": [{"aes": "color",
                                   "annotation": "as_discrete",
                                   "parameters": {"order_by": "rank"}}]},
                               "tooltips": {"lines": ["@gene|@{cell type} \\@home @..count.."],
                                            "title": "@title_col", "variables": ["score", "..density.."],
                                            "formats": [{"field": "@pct", "format": ".1%"}]}}]},
                  null,
                  {"kind": "plot", "mapping": {"x": "other"}}
                ]}"#,
        );
        assert_eq!(
            found,
            [
                "UMAP1",
                "UMAP2",
                "batch",
                "cell type",
                "cluster",
                "donor",
                "gene",
                "other",
                "pct",
                "rank",
                "sample",
                "score",
                "title_col",
            ]
        );
    }
}

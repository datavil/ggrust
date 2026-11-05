//! Interactive legend (HTML only): which discrete legend key each layer row
//! belongs to, so clicking a key can hide its marks and tooltips.
//!
//! A row belongs to the key of a legend whose values (every aesthetic the
//! layer shows in that legend) equal the row's resolved styles; the first key
//! wins when two share values. Rows matching no key (values outside the
//! breaks) stay visible. Keys are named by legend title and label
//! ([`legend_key_id`]), as legends merge.
//!
//! Marks learn their key from a second, probe draw of the layer whose colors
//! name each row's key combination ([`Styles::probe`]): [`tag_marks`] walks the
//! real and probe nodes together and wraps marks in [`KEYED_ROLE`] groups (or
//! tags each marker of a mixed point batch). The real marks stay exactly as
//! drawn, in data order. A mark whose probe color is not a key (a color the geom
//! does not take from the styles) stays untagged and visible.
//!
//! What it does not do: continuous legends (colorbars) and static output.

use serde::Serialize;

use crate::scale::style::LegendEntry;
use crate::scene::{Color, Group, KEYED_ROLE, KeyedItems, Node, PerItem, Points, Rects};
use crate::spec::Aesthetic;
use crate::style::{Styles, probe_color};

/// Layers with more key combinations than this are not interactive: each
/// combination is a group of its own (and a canvas for dense points).
pub const MAX_COMBINATIONS: usize = 64;

/// The id of the key labelled `label` in the legend titled `title`, shared by
/// the legend key and the marks it stands for (interactive legend, HTML only).
/// Legends merge by title and keys by label, so the pair names one key.
pub fn legend_key_id(title: &str, label: &str) -> String {
    format!("{title}\u{1f}{label}")
}

/// [`Group::key`] for marks or a legend key belonging to the keys `ids`.
pub fn key_tag(ids: &[String]) -> String {
    serde_json::Value::from(ids.to_vec()).to_string()
}

/// The discrete legends one layer shows, before `override_aes`: per legend
/// title, the entries of each aesthetic in it.
pub type LayerLegends = Vec<(String, Vec<(Aesthetic, Vec<LegendEntry>)>)>;

/// A key id with, per aesthetic of its legend, the entry it matches.
type KeyValues<'a> = (String, Vec<(Aesthetic, &'a LegendEntry)>);

/// The legend keys of a layer's rows.
#[derive(Debug, Clone, PartialEq, Serialize)]
pub struct RowKeys {
    /// Distinct key combinations, each a list of key ids (one per legend the
    /// row has a key in), in order of first row.
    pub combinations: Vec<Vec<String>>,
    /// Combination of every row.
    pub rows: Vec<u32>,
}

/// The keys of `row_count` rows styled `styles`, or `None` when the layer shows
/// no discrete legend, no row has a key, or there are more than
/// [`MAX_COMBINATIONS`] combinations.
pub fn row_keys(styles: &Styles, row_count: usize, legends: &LayerLegends) -> Option<RowKeys> {
    if legends.is_empty() || row_count == 0 {
        return None;
    }
    // Per legend: its labels (distinct, in entry order) with, per aesthetic, the
    // value of the last entry with that label (as `assemble_legends` keys them).
    let keyed: Vec<Vec<KeyValues>> = legends
        .iter()
        .map(|(title, aesthetics)| {
            let mut labels: Vec<&crate::scale::format::Label> = Vec::new();
            for (_, entries) in aesthetics {
                for entry in entries {
                    if !labels.contains(&&entry.label) {
                        labels.push(&entry.label);
                    }
                }
            }
            labels
                .into_iter()
                .map(|label| {
                    let values = aesthetics
                        .iter()
                        .filter_map(|(aesthetic, entries)| {
                            let entry = entries.iter().rev().find(|entry| entry.label == *label)?;
                            Some((*aesthetic, entry))
                        })
                        .collect();
                    (legend_key_id(title, &label.to_string()), values)
                })
                .collect()
        })
        .collect();
    let mut combinations: Vec<Vec<String>> = Vec::new();
    let mut rows = Vec::with_capacity(row_count);
    let mut ids: Vec<String> = Vec::new();
    for row in 0..row_count {
        ids.clear();
        for keys in &keyed {
            let found = keys.iter().find(|(_, values)| {
                values
                    .iter()
                    .all(|(aesthetic, entry)| styles.row_equals(*aesthetic, row, &entry.value))
            });
            if let Some((id, _)) = found {
                ids.push(id.clone());
            }
        }
        let combination = match combinations.iter().position(|known| *known == ids) {
            Some(index) => index,
            None => {
                if combinations.len() == MAX_COMBINATIONS {
                    return None;
                }
                combinations.push(ids.clone());
                combinations.len() - 1
            }
        };
        rows.push(combination as u32);
    }
    if combinations.iter().all(Vec::is_empty) {
        return None;
    }
    Some(RowKeys { combinations, rows })
}

/// Whether two node trees have the same nodes (kind, count and nesting).
pub fn same_shape(left: &[Node], right: &[Node]) -> bool {
    left.len() == right.len()
        && left.iter().zip(right).all(|pair| match pair {
            (Node::Group(a), Node::Group(b)) => same_shape(&a.nodes, &b.nodes),
            (Node::Raster(a), Node::Raster(b)) => same_shape(&a.nodes, &b.nodes),
            (Node::Points(a), Node::Points(b)) => a.len() == b.len(),
            (Node::Rects(a), Node::Rects(b)) => a.rects.len() == b.rects.len(),
            (a, b) => std::mem::discriminant(a) == std::mem::discriminant(b),
        })
}

/// `nodes` (a layer as drawn) with every mark tagged by the key combination
/// its `probe` twin names (see the module doc). Unchanged where the two trees
/// differ in shape.
pub fn tag_marks(nodes: Vec<Node>, probe: &[Node], combinations: &[Vec<String>]) -> Vec<Node> {
    if nodes.len() != probe.len() {
        return nodes;
    }
    let tags: Vec<String> = combinations.iter().map(|ids| key_tag(ids)).collect();
    let decode = |color: Color| -> Option<u32> {
        let probe = probe_color(color.green as u32);
        let known = color.red == probe.red && color.blue == probe.blue;
        let index = color.green as usize;
        (known && combinations.get(index).is_some_and(|ids| !ids.is_empty()))
            .then_some(index as u32)
    };
    let mut tagged: Vec<(Node, Option<u32>)> = Vec::with_capacity(nodes.len());
    for (node, twin) in nodes.into_iter().zip(probe) {
        match (node, twin) {
            (Node::Group(mut group), Node::Group(twin)) => {
                group.nodes = tag_marks(group.nodes, &twin.nodes, combinations);
                tagged.push((Node::Group(group), None));
            }
            (Node::Raster(mut raster), Node::Raster(twin)) => {
                raster.nodes = tag_marks(raster.nodes, &twin.nodes, combinations);
                tagged.push((Node::Raster(raster), None));
            }
            (Node::Points(points), Node::Points(twin)) if twin.len() == points.len() => {
                let keys: Vec<Option<u32>> = (0..twin.len())
                    .map(|index| {
                        let fill = twin.fill.get(index).and_then(decode);
                        fill.or_else(|| twin.stroke.get(index).and_then(decode))
                    })
                    .collect();
                tagged.push(tag_points(points, &keys, &tags));
            }
            (Node::Rects(rects), Node::Rects(twin)) if twin.rects.len() == rects.rects.len() => {
                let stroke = twin.stroke.as_ref().and_then(|stroke| decode(stroke.color));
                let keys: Vec<Option<u32>> = (0..twin.rects.len())
                    .map(|index| twin.fill.get(index).and_then(decode).or(stroke))
                    .collect();
                tagged.extend(split_rects(rects, &keys));
            }
            (Node::Path(path), Node::Path(twin)) => {
                let fill = twin.fill.and_then(decode);
                let key = fill.or_else(|| twin.stroke.as_ref().and_then(|s| decode(s.color)));
                tagged.push((Node::Path(path), key));
            }
            (Node::Text(text), Node::Text(twin)) => {
                tagged.push((Node::Text(text), decode(twin.color)));
            }
            (node, _) => tagged.push((node, None)),
        }
    }
    // Consecutive marks of one key share a group (stacking order unchanged).
    let mut out: Vec<Node> = Vec::with_capacity(tagged.len());
    let mut open: Option<u32> = None;
    for (node, key) in tagged {
        match key {
            Some(key) if open == Some(key) => {
                if let Some(Node::Group(group)) = out.last_mut() {
                    group.nodes.push(node);
                }
            }
            Some(key) => {
                out.push(Node::Group(Group {
                    clip: None,
                    role: Some(KEYED_ROLE),
                    anchor: None,
                    key: Some(tags[key as usize].clone()),
                    nodes: vec![node],
                }));
                open = Some(key);
            }
            None => {
                out.push(node);
                open = None;
            }
        }
    }
    out
}

/// A point batch with its markers' keys: one key for the batch, or per marker.
fn tag_points(mut points: Points, keys: &[Option<u32>], tags: &[String]) -> (Node, Option<u32>) {
    match keys.first() {
        None => return (Node::Points(points), None),
        Some(first) if keys.iter().all(|key| key == first) => {
            return (Node::Points(points), *first);
        }
        _ => {}
    }
    // Item combination indices into `tags`, with a trailing "[]" for untagged.
    let mut combinations = tags.to_vec();
    combinations.push(key_tag(&[]));
    let untagged = (combinations.len() - 1) as u8;
    let items = keys
        .iter()
        .map(|key| key.map_or(untagged, |key| key as u8))
        .collect();
    points.keys = Some(KeyedItems {
        combinations,
        items,
    });
    (Node::Points(points), None)
}

/// A rectangle batch split into runs of one key (drawing order unchanged).
fn split_rects(rects: Rects, keys: &[Option<u32>]) -> Vec<(Node, Option<u32>)> {
    match keys.first() {
        Some(first) if !keys.iter().all(|key| key == first) => {}
        first => return vec![(Node::Rects(rects), first.copied().flatten())],
    }
    let mut runs = Vec::new();
    let mut start = 0;
    for end in 1..=keys.len() {
        if end == keys.len() || keys[end] != keys[start] {
            let fill = match &rects.fill {
                PerItem::Uniform(fill) => PerItem::Uniform(*fill),
                PerItem::Varying(fills) => PerItem::Varying(fills[start..end].to_vec()),
            };
            let run = Rects {
                rects: rects.rects[start..end].to_vec(),
                fill,
                stroke: rects.stroke.clone(),
            };
            runs.push((Node::Rects(run), keys[start]));
            start = end;
        }
    }
    runs
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scale::format::Label;
    use crate::scene::{Color, PerItem};
    use crate::style::StyleValues;

    fn color(red: u8) -> Color {
        Color::rgb(red, 0, 0)
    }

    fn entry(label: &str, red: u8) -> LegendEntry {
        LegendEntry {
            label: Label::plain(label),
            value: StyleValues::Colors(PerItem::Uniform(color(red))),
        }
    }

    fn styles(reds: &[u8]) -> Styles {
        let mut styles = Styles::default();
        styles
            .insert(
                Aesthetic::Color,
                StyleValues::Colors(PerItem::Varying(
                    reds.iter().map(|&red| color(red)).collect(),
                )),
            )
            .unwrap();
        styles
    }

    #[test]
    fn rows_take_the_key_their_style_matches() {
        let legends = vec![(
            "kind".to_string(),
            vec![(Aesthetic::Color, vec![entry("a", 1), entry("b", 2)])],
        )];
        let keys = row_keys(&styles(&[2, 1, 9, 2]), 4, &legends).unwrap();
        assert_eq!(
            keys.combinations,
            vec![
                vec![legend_key_id("kind", "b")],
                vec![legend_key_id("kind", "a")],
                vec![]
            ]
        );
        assert_eq!(keys.rows, [0, 1, 2, 0]);
    }

    #[test]
    fn no_legend_or_no_match_gives_none() {
        assert_eq!(row_keys(&styles(&[1]), 1, &Vec::new()), None);
        let legends = vec![(
            "kind".to_string(),
            vec![(Aesthetic::Color, vec![entry("a", 1)])],
        )];
        assert_eq!(row_keys(&styles(&[5, 6]), 2, &legends), None);
    }

    #[test]
    fn too_many_combinations_give_none() {
        let entries: Vec<LegendEntry> = (0..=MAX_COMBINATIONS as u8)
            .map(|red| entry(&red.to_string(), red))
            .collect();
        let reds: Vec<u8> = (0..=MAX_COMBINATIONS as u8).collect();
        let legends = vec![("kind".to_string(), vec![(Aesthetic::Color, entries)])];
        assert_eq!(row_keys(&styles(&reds), reds.len(), &legends), None);
    }
}

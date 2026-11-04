//! Hierarchical clustering and dendrogram geometry (ggdendro, legendry
//! `scale_*_dendro`).
//!
//! [`cluster_rows`] clusters the rows of a numeric matrix into a [`Tree`] with the
//! merges, heights and leaf order of R's `hclust`; [`Tree::from_spec`] reads a tree
//! back from a plot spec; [`Tree::segments`] draws it with leaves at 0, 1, 2, ...
//! (the positions of a discrete axis).
//!
//! What it does not do: draw anything itself, cut trees into clusters, or reorder
//! a tree (`reorder.dendrogram`).

mod distance;
mod hclust;

pub use distance::{Distance, dissimilarities};
pub use hclust::{Linkage, condensed_index, hclust};

use crate::data::{Column, DataFrame};
use crate::{Error, Result};

/// A clustering tree in R's `hclust` layout.
#[derive(Debug, Clone, PartialEq)]
pub struct Tree {
    /// Leaf names, in input order.
    pub labels: Vec<String>,
    /// One row per merge: `-k` is leaf `k` (1-based), `+s` the cluster made at
    /// step `s` (1-based), as R's `merge`.
    pub merge: Vec<[i64; 2]>,
    /// Height of each merge.
    pub height: Vec<f64>,
    /// Leaves (0-based) from left to right (R `order` minus one).
    pub order: Vec<usize>,
}

/// How a tree's links are drawn.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum DendrogramType {
    /// Elbows: down from each node, then across to its children (default).
    #[default]
    Rectangle,
    /// Straight lines from each node to its children.
    Triangle,
}

/// One drawn link: from (`position`, `height`) to (`position_end`, `height_end`),
/// `position` along the leaves (leaf `i` of the order at `i`).
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct Segment {
    /// Start along the leaf axis.
    pub position: f64,
    /// Start height.
    pub height: f64,
    /// End along the leaf axis.
    pub position_end: f64,
    /// End height.
    pub height_end: f64,
}

/// Cluster the `n` rows of a row-major `n x columns` matrix. `labels` name the
/// rows (one per row).
///
/// Errors (as data errors naming the row) on missing or infinite values, and on a
/// row without variance under correlation distance.
pub fn cluster_rows(
    values: &[f64],
    columns: usize,
    labels: Vec<String>,
    distance: Distance,
    linkage: Linkage,
) -> Result<Tree> {
    let n = labels.len();
    if values.len() != n * columns {
        return Err(Error::data(format!(
            "clustering needs {n} rows of {columns} values, got {} values",
            values.len()
        )));
    }
    let condensed = dissimilarities(values, columns, &labels, distance)?;
    let mut tree = hclust(condensed, n, linkage);
    tree.labels = labels;
    Ok(tree)
}

/// Cluster the rows of `frame` (or its numeric columns, `by_columns`). Every
/// column but `labels` must be numeric; leaves are named by the `labels` column
/// (rows), the column names (columns), or `"1"`, `"2"`, ... (rows without labels).
///
/// Errors (data errors) name the column or row at fault.
pub fn cluster_frame(
    frame: &DataFrame,
    labels: Option<&str>,
    by_columns: bool,
    distance: Distance,
    linkage: Linkage,
) -> Result<Tree> {
    let rows = frame.row_count();
    let mut names = Vec::new();
    let mut columns = Vec::new();
    for name in frame.column_names() {
        if Some(name) == labels {
            continue;
        }
        match frame.column(name)? {
            Column::Continuous(column) => {
                names.push(name.to_owned());
                columns.push(column);
            }
            Column::Discrete(_) => {
                return Err(Error::data(format!(
                    "cannot cluster: column '{name}' is not numeric; every column but the labels \
                     must hold numbers (name the label column with labels=, or drop the column)"
                )));
            }
        }
    }
    if columns.is_empty() {
        return Err(Error::data(
            "cannot cluster: the data has no numeric columns".to_owned(),
        ));
    }
    let value = |row: usize, column: usize| columns[column].get(row).unwrap_or(f64::NAN);
    let row_name = |row: usize| match labels.map(|name| frame.column(name)) {
        Some(Ok(Column::Discrete(column))) => column.code(row).map_or_else(
            || format!("row {row}"),
            |code| column.levels()[code as usize].clone(),
        ),
        // Rows without labels are leaves "1", "2", ...
        _ if labels.is_none() => (row + 1).to_string(),
        _ => format!("row {}", row + 1),
    };
    for (column, name) in names.iter().enumerate() {
        if let Some(row) = (0..rows).find(|row| !value(*row, column).is_finite()) {
            return Err(Error::data(format!(
                "cannot cluster: column '{name}' has a missing or infinite value at '{}'; fill or drop missing values first",
                row_name(row)
            )));
        }
    }
    if by_columns {
        let values: Vec<f64> = (0..columns.len())
            .flat_map(|column| (0..rows).map(move |row| (row, column)))
            .map(|(row, column)| value(row, column))
            .collect();
        return cluster_rows(&values, rows, names, distance, linkage);
    }
    let leaf_names = match labels {
        None => (1..=rows).map(|row| row.to_string()).collect(),
        Some(label_column) => match frame.column(label_column)? {
            Column::Discrete(column) => (0..rows)
                .map(|row| {
                    column
                        .code(row)
                        .map(|code| column.levels()[code as usize].clone())
                        .ok_or_else(|| {
                            Error::data(format!(
                                "cannot cluster: label column '{label_column}' is missing a value in row {row}; every leaf needs a name"
                            ))
                        })
                })
                .collect::<Result<Vec<_>>>()?,
            Column::Continuous(column) => (0..rows)
                .map(|row| {
                    column.get(row).map(number_label).ok_or_else(|| {
                        Error::data(format!(
                            "cannot cluster: label column '{label_column}' is missing a value in row {row}; every leaf needs a name"
                        ))
                    })
                })
                .collect::<Result<Vec<_>>>()?,
        },
    };
    let mut seen = std::collections::HashSet::new();
    if let Some(name) = leaf_names.iter().find(|name| !seen.insert(name.as_str())) {
        return Err(Error::data(format!(
            "cannot cluster: label '{name}' names two rows; leaves need distinct names (aggregate repeated rows first)"
        )));
    }
    let values: Vec<f64> = (0..rows)
        .flat_map(|row| (0..columns.len()).map(move |column| (row, column)))
        .map(|(row, column)| value(row, column))
        .collect();
    cluster_rows(&values, columns.len(), leaf_names, distance, linkage)
}

/// A number as a leaf name: integers without a decimal point.
fn number_label(value: f64) -> String {
    if value.fract() == 0.0 && value.abs() < 1e15 {
        format!("{}", value as i64)
    } else {
        value.to_string()
    }
}

impl Tree {
    /// Leaf labels from left to right.
    pub fn ordered_labels(&self) -> Vec<&str> {
        self.order
            .iter()
            .map(|leaf| self.labels[*leaf].as_str())
            .collect()
    }

    /// Read and check a tree given in a spec (`path` names it in errors): every
    /// leaf joined exactly once, each merge only refers to earlier ones, heights
    /// finite, `order` a permutation that keeps every cluster contiguous.
    pub fn from_spec(
        path: &str,
        labels: Vec<String>,
        merge: Vec<[i64; 2]>,
        height: Vec<f64>,
        order: Vec<usize>,
    ) -> Result<Tree> {
        let n = labels.len();
        if n == 0 {
            return Err(Error::spec(
                format!("{path}.labels"),
                "a dendrogram needs at least one leaf; cluster at least one item",
            ));
        }
        if merge.len() != n - 1 || height.len() != n - 1 || order.len() != n {
            return Err(Error::spec(
                path,
                format!(
                    "a dendrogram of {n} leaves needs {} merges, {} heights and an order of {n}; got {}, {} and {}; build it with ggrust.hclust()",
                    n - 1,
                    n - 1,
                    merge.len(),
                    height.len(),
                    order.len()
                ),
            ));
        }
        let mut seen = vec![false; n + merge.len()];
        for (step, row) in merge.iter().enumerate() {
            for side in row {
                let slot = match *side {
                    leaf if leaf < 0 && (-leaf as usize) <= n => (-leaf - 1) as usize,
                    cluster if cluster > 0 && (cluster as usize) <= step => {
                        n + cluster as usize - 1
                    }
                    other => {
                        return Err(Error::spec(
                            format!("{path}.merge[{step}]"),
                            format!(
                                "{other} is neither a leaf (-1..-{n}) nor an earlier merge (1..{step})"
                            ),
                        ));
                    }
                };
                if std::mem::replace(&mut seen[slot], true) {
                    return Err(Error::spec(
                        format!("{path}.merge[{step}]"),
                        format!("{side} is merged twice; every leaf and merge joins once"),
                    ));
                }
            }
        }
        if let Some(index) = height.iter().position(|value| !value.is_finite()) {
            return Err(Error::spec(
                format!("{path}.height[{index}]"),
                "merge heights must be finite numbers",
            ));
        }
        let tree = Tree {
            labels,
            merge,
            height,
            order,
        };
        if !tree.order_is_valid() {
            return Err(Error::spec(
                format!("{path}.order"),
                "the leaf order must list every leaf (0-based) once and keep each cluster together",
            ));
        }
        Ok(tree)
    }

    /// Whether `order` is a permutation in which every cluster's leaves are
    /// adjacent (so the tree can be drawn without crossings).
    fn order_is_valid(&self) -> bool {
        let n = self.labels.len();
        let mut rank = vec![usize::MAX; n];
        for (position, leaf) in self.order.iter().enumerate() {
            if *leaf >= n || rank[*leaf] != usize::MAX {
                return false;
            }
            rank[*leaf] = position;
        }
        // Each cluster's (lowest rank, highest rank, size): contiguous when the
        // span equals the size.
        let mut spans: Vec<(usize, usize, usize)> = Vec::with_capacity(self.merge.len());
        for row in &self.merge {
            let mut span = (usize::MAX, 0, 0);
            for side in row {
                let (low, high, size) = if *side < 0 {
                    let at = rank[(-side - 1) as usize];
                    (at, at, 1)
                } else {
                    spans[(*side - 1) as usize]
                };
                span = (span.0.min(low), span.1.max(high), span.2 + size);
            }
            if span.1 - span.0 + 1 != span.2 {
                return false;
            }
            spans.push(span);
        }
        true
    }

    /// The tree's links, leaf `i` of the order at position `i`, height 0 at the
    /// leaves; a node sits midway between its two children (R `plot.hclust`).
    pub fn segments(&self, kind: DendrogramType) -> Vec<Segment> {
        let mut leaf_position = vec![0.0; self.labels.len()];
        for (position, leaf) in self.order.iter().enumerate() {
            leaf_position[*leaf] = position as f64;
        }
        self.segments_at(&leaf_position, kind)
    }

    /// [`Self::segments`] with leaf `k` (input order) at `leaf_position[k]`, e.g.
    /// its category's place on a reversed axis.
    pub fn segments_at(&self, leaf_position: &[f64], kind: DendrogramType) -> Vec<Segment> {
        let mut node_position = Vec::with_capacity(self.merge.len());
        let mut segments = Vec::with_capacity(self.merge.len() * 4);
        for (row, height) in self.merge.iter().zip(&self.height) {
            let child = |side: i64, nodes: &[f64]| {
                if side < 0 {
                    (leaf_position[(-side - 1) as usize], 0.0)
                } else {
                    (nodes[(side - 1) as usize], self.height[(side - 1) as usize])
                }
            };
            let first = child(row[0], &node_position);
            let second = child(row[1], &node_position);
            let middle = (first.0 + second.0) / 2.0;
            for (position, child_height) in [first, second] {
                match kind {
                    DendrogramType::Rectangle => {
                        segments.push(Segment {
                            position,
                            height: *height,
                            position_end: position,
                            height_end: child_height,
                        });
                        segments.push(Segment {
                            position: middle,
                            height: *height,
                            position_end: position,
                            height_end: *height,
                        });
                    }
                    DendrogramType::Triangle => segments.push(Segment {
                        position: middle,
                        height: *height,
                        position_end: position,
                        height_end: child_height,
                    }),
                }
            }
            node_position.push(middle);
        }
        segments
    }

    /// The root height (0 for a single leaf).
    pub fn max_height(&self) -> f64 {
        self.height.iter().copied().fold(0.0, f64::max)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn small() -> Tree {
        Tree {
            labels: ["a", "b", "c"].map(str::to_owned).to_vec(),
            merge: vec![[-1, -3], [-2, 1]],
            height: vec![1.0, 4.0],
            order: vec![1, 0, 2],
        }
    }

    #[test]
    fn rectangle_segments_drop_from_each_node_to_its_children() {
        let segments = small().segments(DendrogramType::Rectangle);
        assert_eq!(segments.len(), 8);
        // Merge 1 joins leaves a (position 1) and c (position 2) at height 1.
        assert_eq!(
            segments[0],
            Segment {
                position: 1.0,
                height: 1.0,
                position_end: 1.0,
                height_end: 0.0
            }
        );
        assert_eq!(segments[1].position, 1.5);
        // The root sits midway between b (0) and merge 1 (1.5).
        assert_eq!(segments[5].position, 0.75);
        assert_eq!(segments[6].height_end, 1.0);
    }

    #[test]
    fn spec_trees_are_checked() {
        let tree = small();
        let ok = Tree::from_spec(
            "scales[0].dendrogram",
            tree.labels.clone(),
            tree.merge.clone(),
            tree.height.clone(),
            tree.order.clone(),
        );
        assert_eq!(ok.expect("valid"), tree);
        let twice = Tree::from_spec(
            "t",
            tree.labels.clone(),
            vec![[-1, -3], [-1, 1]],
            tree.height.clone(),
            tree.order.clone(),
        );
        assert!(twice.expect_err("bad").to_string().contains("merged twice"));
        let split = Tree::from_spec(
            "t",
            tree.labels.clone(),
            tree.merge.clone(),
            tree.height.clone(),
            vec![0, 1, 2],
        );
        let message = split.expect_err("bad").to_string();
        assert!(message.contains("t.order"), "{message}");
        let forward = Tree::from_spec(
            "t",
            tree.labels,
            vec![[-1, 2], [-2, -3]],
            tree.height,
            tree.order,
        );
        assert!(
            forward
                .expect_err("bad")
                .to_string()
                .contains("earlier merge")
        );
    }
}

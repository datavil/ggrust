//! Agglomerative hierarchical clustering with the results of R's `hclust`.
//!
//! The algorithm is the stored-dissimilarity approach with a nearest-neighbour list
//! and Lance-Williams updates (F. Murtagh, "Multidimensional Clustering
//! Algorithms", 1985), the one R's `hclust` uses. R's results depend on its exact
//! choices, which are kept here:
//!
//! - Ties go to the lowest index: the smallest pair `(i, j)` with `i < j` merges
//!   first, scanning the nearest-neighbour list from item 0.
//! - The merged cluster takes the lower index of the two; the higher one retires.
//! - `ward.D2` clusters squared distances and reports the square root as heights.
//! - Merges and leaf order follow R's convention (see [`super::Tree`]).
//!
//! What it does not do: `centroid`, `median`, `mcquitty` or `ward.D` linkage,
//! missing values (callers reject them first).

use super::Tree;

/// How the dissimilarity between two clusters is updated after a merge.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum Linkage {
    /// Largest pairwise dissimilarity (R `"complete"`).
    Complete,
    /// Mean pairwise dissimilarity (UPGMA, R `"average"`).
    Average,
    /// Smallest pairwise dissimilarity (R `"single"`).
    Single,
    /// Ward's minimum variance on squared distances (R `"ward.D2"`).
    WardD2,
}

impl Linkage {
    /// Every linkage with its R name, in documentation order.
    pub const ALL: [(&'static str, Linkage); 4] = [
        ("complete", Linkage::Complete),
        ("average", Linkage::Average),
        ("single", Linkage::Single),
        ("ward.D2", Linkage::WardD2),
    ];

    /// The linkage with this R name.
    pub fn from_name(name: &str) -> Option<Linkage> {
        Linkage::ALL
            .iter()
            .find(|(known, _)| *known == name)
            .map(|(_, linkage)| *linkage)
    }

    /// The R name (`"ward.D2"`, ...).
    pub fn name(self) -> &'static str {
        Linkage::ALL
            .iter()
            .find(|(_, linkage)| *linkage == self)
            .map_or("complete", |(name, _)| name)
    }
}

/// Index of the pair `first < second` in a condensed dissimilarity vector of `n`
/// items (R `dist` order: row-wise upper triangle).
pub fn condensed_index(n: usize, first: usize, second: usize) -> usize {
    debug_assert!(first < second && second < n);
    n * first - first * (first + 1) / 2 + second - first - 1
}

/// Cluster `n` items from their condensed dissimilarities (`n * (n - 1) / 2`
/// values in [`condensed_index`] order, all finite). Labels are `"1"`, `"2"`, ...;
/// callers replace them.
///
/// Fewer than 2 items give a tree without merges.
pub fn hclust(mut dissimilarities: Vec<f64>, n: usize, linkage: Linkage) -> Tree {
    debug_assert_eq!(dissimilarities.len(), n * n.saturating_sub(1) / 2);
    let labels = (1..=n).map(|index| index.to_string()).collect();
    if n < 2 {
        return Tree {
            labels,
            merge: Vec::new(),
            height: Vec::new(),
            order: (0..n).collect(),
        };
    }
    let ward = linkage == Linkage::WardD2;
    if ward {
        for value in &mut dissimilarities {
            *value *= *value;
        }
    }
    let dissimilarity = |values: &[f64], first: usize, second: usize| {
        let (low, high) = if first < second {
            (first, second)
        } else {
            (second, first)
        };
        values[condensed_index(n, low, high)]
    };

    let mut active = vec![true; n];
    let mut members = vec![1.0_f64; n];
    // Nearest neighbour (with a higher index) of every item but the last.
    let mut neighbour = vec![0_usize; n];
    let mut neighbour_distance = vec![f64::INFINITY; n];
    for first in 0..n - 1 {
        let (mut best, mut best_distance) = (first + 1, f64::INFINITY);
        for second in first + 1..n {
            let value = dissimilarities[condensed_index(n, first, second)];
            if value < best_distance {
                best_distance = value;
                best = second;
            }
        }
        neighbour[first] = best;
        neighbour_distance[first] = best_distance;
    }

    // Pairs merged at each step (item indices, low first) and their heights.
    let mut pairs = Vec::with_capacity(n - 1);
    let mut height = Vec::with_capacity(n - 1);
    for _ in 0..n - 1 {
        let (mut first, mut second, mut least) = (0, 0, f64::INFINITY);
        for item in 0..n - 1 {
            if active[item] && neighbour_distance[item] < least {
                least = neighbour_distance[item];
                first = item;
                second = neighbour[item];
            }
        }
        if !least.is_finite() {
            // Only reachable with infinite dissimilarities: merge the first two
            // active items so the tree stays complete.
            let mut remaining = (0..n).filter(|item| active[*item]);
            first = remaining.next().unwrap_or(0);
            second = remaining.next().unwrap_or(first);
            least = dissimilarity(&dissimilarities, first, second);
        }
        let (kept, retired) = (first.min(second), first.max(second));
        pairs.push((kept, retired));
        height.push(if ward { least.sqrt() } else { least });
        active[retired] = false;

        let between = dissimilarity(&dissimilarities, kept, retired);
        let (mut nearest, mut nearest_distance) = (neighbour[kept], f64::INFINITY);
        for other in 0..n {
            if !active[other] || other == kept {
                continue;
            }
            let to_kept = dissimilarity(&dissimilarities, kept, other);
            let to_retired = dissimilarity(&dissimilarities, retired, other);
            let updated = match linkage {
                Linkage::Complete => to_kept.max(to_retired),
                Linkage::Single => to_kept.min(to_retired),
                Linkage::Average => {
                    (members[kept] * to_kept + members[retired] * to_retired)
                        / (members[kept] + members[retired])
                }
                Linkage::WardD2 => {
                    ((members[kept] + members[other]) * to_kept
                        + (members[retired] + members[other]) * to_retired
                        - members[other] * between)
                        / (members[kept] + members[retired] + members[other])
                }
            };
            let (low, high) = (kept.min(other), kept.max(other));
            dissimilarities[condensed_index(n, low, high)] = updated;
            if kept < other {
                if updated < nearest_distance {
                    nearest_distance = updated;
                    nearest = other;
                }
            } else if updated < neighbour_distance[other] {
                neighbour_distance[other] = updated;
                neighbour[other] = kept;
            }
        }
        members[kept] += members[retired];
        neighbour[kept] = nearest;
        neighbour_distance[kept] = nearest_distance;

        // Items whose nearest neighbour was one of the merged two look again.
        for item in 0..n - 1 {
            if active[item] && (neighbour[item] == kept || neighbour[item] == retired) {
                let (mut best, mut best_distance) = (neighbour[item], f64::INFINITY);
                for other in item + 1..n {
                    if active[other] {
                        let value = dissimilarities[condensed_index(n, item, other)];
                        if value < best_distance {
                            best_distance = value;
                            best = other;
                        }
                    }
                }
                neighbour[item] = best;
                neighbour_distance[item] = best_distance;
            }
        }
    }

    let merge = merge_matrix(&pairs);
    let order = leaf_order(&merge);
    Tree {
        labels,
        merge,
        height,
        order,
    }
}

/// R's merge matrix from the merged item pairs: `-k` is item `k` (1-based), `+s`
/// the cluster made at step `s`; a singleton comes before a cluster, and two
/// clusters (or two singletons) go in increasing order.
fn merge_matrix(pairs: &[(usize, usize)]) -> Vec<[i64; 2]> {
    // The cluster an item index currently stands for (0: still a singleton).
    let mut cluster_of = vec![0_i64; pairs.len() + 1];
    let mut merge = Vec::with_capacity(pairs.len());
    for (step, &(kept, retired)) in pairs.iter().enumerate() {
        let side = |item: usize| match cluster_of[item] {
            0 => -(item as i64 + 1),
            cluster => cluster,
        };
        let (first, second) = (side(kept), side(retired));
        let row = match (first < 0, second < 0) {
            // A cluster and a singleton: the singleton first.
            (false, true) => [second, first],
            (true, false) => [first, second],
            // Two singletons: `-kept` first (kept < retired); two clusters: lower step first.
            (true, true) => [first, second],
            (false, false) => [first.min(second), first.max(second)],
        };
        merge.push(row);
        cluster_of[kept] = step as i64 + 1;
    }
    merge
}

/// Leaves (0-based items) from left to right: the last merge expanded depth
/// first, its first side before its second (R `hclust()$order`).
pub(super) fn leaf_order(merge: &[[i64; 2]]) -> Vec<usize> {
    let Some(root) = merge.len().checked_sub(1) else {
        return vec![0];
    };
    let mut order = Vec::with_capacity(merge.len() + 1);
    let mut stack = vec![root as i64 + 1];
    while let Some(node) = stack.pop() {
        if node < 0 {
            order.push((-node - 1) as usize);
        } else {
            let [first, second] = merge[(node - 1) as usize];
            stack.push(second);
            stack.push(first);
        }
    }
    order
}

#[cfg(test)]
mod tests {
    use super::*;

    fn condensed(matrix: &[&[f64]]) -> Vec<f64> {
        let n = matrix.len();
        let mut values = Vec::new();
        for (first, row) in matrix.iter().enumerate() {
            values.extend_from_slice(&row[first + 1..n]);
        }
        values
    }

    #[test]
    fn a_small_complete_tree_matches_r() {
        // R: hclust(as.dist(m), "complete") for this matrix.
        let matrix: &[&[f64]] = &[
            &[0.0, 2.0, 6.0, 10.0, 9.0],
            &[2.0, 0.0, 5.0, 9.0, 8.0],
            &[6.0, 5.0, 0.0, 4.0, 5.0],
            &[10.0, 9.0, 4.0, 0.0, 3.0],
            &[9.0, 8.0, 5.0, 3.0, 0.0],
        ];
        let tree = hclust(condensed(matrix), 5, Linkage::Complete);
        assert_eq!(tree.merge, vec![[-1, -2], [-4, -5], [-3, 2], [1, 3]]);
        assert_eq!(tree.height, vec![2.0, 3.0, 5.0, 10.0]);
        assert_eq!(tree.order, vec![0, 1, 2, 3, 4]);
    }

    #[test]
    fn one_or_zero_items_give_no_merges() {
        let tree = hclust(Vec::new(), 1, Linkage::Average);
        assert!(tree.merge.is_empty());
        assert_eq!(tree.order, vec![0]);
        assert!(hclust(Vec::new(), 0, Linkage::Average).order.is_empty());
    }

    #[test]
    fn linkage_names_round_trip() {
        for (name, linkage) in Linkage::ALL {
            assert_eq!(Linkage::from_name(name), Some(linkage));
            assert_eq!(linkage.name(), name);
        }
        assert_eq!(Linkage::from_name("ward.D"), None);
    }
}

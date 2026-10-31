//! Close-match suggestions for misspelt names ("did you mean 'point'?").
//!
//! Only names: column names, geoms, stats, positions, palettes, option values. It
//! does not decide whether a name is valid; callers check that first.

/// The candidate closest to `word` when it is close enough to be a likely typo:
/// about one edit per three letters (none for one- and two-letter words, where any
/// other name is one edit away). Case differences alone always count as a match.
/// No suggestion when the guess would be arbitrary: several candidates equally
/// close, or `word` the start of several names (`log` for `log2` and `log10`).
pub fn closest<'a, I>(word: &str, candidates: I) -> Option<&'a str>
where
    I: IntoIterator<Item = &'a str>,
{
    let lowered = word.to_lowercase();
    let limit = word.chars().count() / 3;
    let mut best: Option<(usize, &str)> = None;
    let mut tied = false;
    let mut extensions = 0;
    for candidate in candidates
        .into_iter()
        .filter(|candidate| *candidate != word)
    {
        let candidate_lowered = candidate.to_lowercase();
        if candidate_lowered.starts_with(&lowered) && candidate_lowered != lowered {
            extensions += 1;
        }
        let distance = distance(&lowered, &candidate_lowered);
        if distance > limit {
            continue;
        }
        match best {
            Some((best_distance, _)) if distance > best_distance => {}
            Some((best_distance, _)) if distance == best_distance => tied = true,
            _ => {
                best = Some((distance, candidate));
                tied = false;
            }
        }
    }
    if tied || extensions > 1 {
        return None;
    }
    best.map(|(_, candidate)| candidate)
}

/// `"; did you mean 'x'?"` for the closest candidate, or an empty string.
pub fn did_you_mean<'a, I>(word: &str, candidates: I) -> String
where
    I: IntoIterator<Item = &'a str>,
{
    closest(word, candidates)
        .map(|candidate| format!("; did you mean '{candidate}'?"))
        .unwrap_or_default()
}

/// The quoted names of a listing written for messages, e.g. `"'bw', 'grey' ('gray')"`
/// gives `bw`, `grey`, `gray`. Keeps one list for the message and the suggestion.
pub fn quoted(listing: &str) -> Vec<&str> {
    listing
        .split(['\'', '"'])
        .skip(1)
        .step_by(2)
        .filter(|name| !name.is_empty())
        .collect()
}

/// Optimal string alignment distance (Levenshtein plus adjacent transpositions).
fn distance(left: &str, right: &str) -> usize {
    let left: Vec<char> = left.chars().collect();
    let right: Vec<char> = right.chars().collect();
    let width = right.len() + 1;
    let mut rows = vec![0usize; (left.len() + 1) * width];
    for (index, cell) in rows.iter_mut().enumerate().take(width) {
        *cell = index;
    }
    for row in 1..=left.len() {
        rows[row * width] = row;
        for column in 1..=right.len() {
            let cost = usize::from(left[row - 1] != right[column - 1]);
            let mut best = (rows[(row - 1) * width + column] + 1)
                .min(rows[row * width + column - 1] + 1)
                .min(rows[(row - 1) * width + column - 1] + cost);
            if row > 1
                && column > 1
                && left[row - 1] == right[column - 2]
                && left[row - 2] == right[column - 1]
            {
                best = best.min(rows[(row - 2) * width + column - 2] + 1);
            }
            rows[row * width + column] = best;
        }
    }
    rows[left.len() * width + right.len()]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn typos_find_their_name() {
        let geoms = ["point", "line", "bar", "boxplot"];
        assert_eq!(closest("piont", geoms), Some("point"));
        assert_eq!(closest("pont", geoms), Some("point"));
        assert_eq!(closest("boxplto", geoms), Some("boxplot"));
        assert_eq!(closest("Line", geoms), Some("line"));
    }

    #[test]
    fn unrelated_words_get_no_suggestion() {
        let geoms = ["point", "line", "bar"];
        assert_eq!(closest("histogram", geoms), None);
        assert_eq!(closest("xy", geoms), None);
        assert_eq!(closest("x", ["y", "z"]), None);
        assert_eq!(closest("X", ["x", "y"]), Some("x"));
        assert_eq!(did_you_mean("histogram", geoms), "");
    }

    #[test]
    fn arbitrary_guesses_get_no_suggestion() {
        assert_eq!(closest("log", ["identity", "log10", "log2"]), None);
        assert_eq!(closest("genee", ["gene0", "gene1", "gene2"]), None);
        assert_eq!(closest("lgo10", ["log10", "log2"]), Some("log10"));
    }

    #[test]
    fn quoted_names_of_a_listing() {
        assert_eq!(
            quoted("'minimal2', 'grey' ('gray') or \"turbo\""),
            ["minimal2", "grey", "gray", "turbo"]
        );
    }

    #[test]
    fn suggestion_text() {
        assert_eq!(
            did_you_mean("sepal_lenght", ["sepal_length", "species"]),
            "; did you mean 'sepal_length'?"
        );
    }
}

//! Automatic tags across a grid: `gggrid(tag_levels=..., tag_level=...)`
//! (patchwork `plot_annotation(tag_levels=)` and `plot_layout(tag_level=)`).
//!
//! What it does: number every plot of a tagged grid in figure order (a plot, then
//! its insets, then the next figure), skipping empty cells and insets with
//! `ignore_tag`. Nested grids continue the numbering (`tag_level="keep"`) or take
//! one tag and number their plots at the next level (`"new"`: A1, A2), or take one
//! tag that the grid draws itself, leaving its plots untagged (`"unit"`: patchwork's
//! `wrap_elements(full=)` of a nested patchwork).
//!
//! What it does not do: draw tags (each plot draws its own, [`crate::layout::tag`])
//! or style them (the plots' themes, inherited from the grid).

use crate::spec::{Figure, InsetSpec, SubplotsSpec};
use crate::{Error, Result};

/// The tags of one figure.
#[derive(Debug, Clone, PartialEq)]
pub enum FigureTags {
    /// Not tagged: an empty cell, an ignored inset, or outside a tagged grid.
    Untagged,
    /// A plot's tag and its insets' tags.
    Plot(String, Vec<FigureTags>),
    /// A grid's figures' tags and its insets' tags.
    Grid(Vec<FigureTags>, Vec<FigureTags>),
    /// A grid tagged as one unit (`tag_level="unit"`): the grid draws this tag, its
    /// figures and insets are untagged.
    Unit(String),
}

impl FigureTags {
    /// Whether these are a nested grid's tags (numbered figures or one unit tag).
    pub fn is_grid(&self) -> bool {
        matches!(self, FigureTags::Grid(..) | FigureTags::Unit(_))
    }
}

/// How a grid nested in a tagged grid is tagged (`tag_level=`).
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum TagLevel {
    /// Its plots continue the containing grid's numbering (the default).
    Keep,
    /// It takes one tag and numbers its plots at the next level.
    New,
    /// It takes one tag, drawn by the grid; its plots are untagged.
    Unit,
}

/// One numbering of `tag_levels`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
enum Level {
    Upper,
    Lower,
    Number,
    RomanLower,
    RomanUpper,
}

impl Level {
    fn parse(name: &str) -> Result<Level> {
        Ok(match name {
            "A" => Level::Upper,
            "a" => Level::Lower,
            "1" => Level::Number,
            "i" => Level::RomanLower,
            "I" => Level::RomanUpper,
            other => {
                return Err(Error::spec(
                    "layout.tag_levels",
                    format!(
                        "unknown tag level '{other}'; expected 'A', 'a', '1', 'i' or 'I' \
                         (letters, lowercase letters, numbers, roman numerals)"
                    ),
                ));
            }
        })
    }

    /// The `index`th tag (0-based). Letters go on past Z as AA, AB, ... (a ggrust
    /// choice: patchwork gives NA past 26).
    fn format(self, index: usize) -> String {
        match self {
            Level::Upper => letters(index),
            Level::Lower => letters(index).to_lowercase(),
            Level::Number => (index + 1).to_string(),
            Level::RomanLower => roman(index + 1).to_lowercase(),
            Level::RomanUpper => roman(index + 1),
        }
    }
}

/// Spreadsheet column letters: 0 -> A, 25 -> Z, 26 -> AA.
fn letters(index: usize) -> String {
    let mut index = index + 1;
    let mut text = Vec::new();
    while index > 0 {
        let digit = (index - 1) % 26;
        text.push(b'A' + digit as u8);
        index = (index - 1) / 26;
    }
    text.reverse();
    String::from_utf8(text).unwrap_or_default()
}

/// Roman numerals (thousands as repeated M).
fn roman(number: usize) -> String {
    const NUMERALS: [(usize, &str); 13] = [
        (1000, "M"),
        (900, "CM"),
        (500, "D"),
        (400, "CD"),
        (100, "C"),
        (90, "XC"),
        (50, "L"),
        (40, "XL"),
        (10, "X"),
        (9, "IX"),
        (5, "V"),
        (4, "IV"),
        (1, "I"),
    ];
    let mut rest = number;
    let mut text = String::new();
    for (value, numeral) in NUMERALS {
        while rest >= value {
            text.push_str(numeral);
            rest -= value;
        }
    }
    text
}

/// A grid's `tag_level`; errors name the accepted values.
pub fn tag_level(spec: &SubplotsSpec) -> Result<TagLevel> {
    match spec.layout.tag_level.as_deref() {
        None | Some("keep") => Ok(TagLevel::Keep),
        Some("new") => Ok(TagLevel::New),
        Some("unit") => Ok(TagLevel::Unit),
        Some(other) => Err(Error::spec(
            "layout.tag_level",
            format!("expected 'keep', 'new' or 'unit', got '{other}'"),
        )),
    }
}

/// The tags of a grid that sets `tag_levels` (`None` when it sets none), with
/// warnings for nested `tag_levels` that are ignored.
pub fn grid_tags(spec: &SubplotsSpec, warnings: &mut Vec<String>) -> Result<Option<FigureTags>> {
    let Some(names) = &spec.layout.tag_levels else {
        return Ok(None);
    };
    if names.is_empty() {
        return Err(Error::spec(
            "layout.tag_levels",
            "is empty; expected 'A', 'a', '1', 'i' or 'I', or a list of them",
        ));
    }
    let levels = names
        .iter()
        .map(|name| Level::parse(name))
        .collect::<Result<Vec<_>>>()?;
    let mut tagger = Tagger {
        levels,
        warnings,
        path: String::new(),
    };
    let mut counter = 0;
    tagger.grid(spec, 0, "", &mut counter).map(Some)
}

struct Tagger<'w> {
    levels: Vec<Level>,
    warnings: &'w mut Vec<String>,
    /// Spec path of the figure being tagged, for messages.
    path: String,
}

impl Tagger<'_> {
    fn next(&self, depth: usize, prefix: &str, counter: &mut usize) -> String {
        let tag = format!("{prefix}{}", self.levels[depth].format(*counter));
        *counter += 1;
        tag
    }

    fn grid(
        &mut self,
        spec: &SubplotsSpec,
        depth: usize,
        prefix: &str,
        counter: &mut usize,
    ) -> Result<FigureTags> {
        let path = self.path.clone();
        let mut figures = Vec::with_capacity(spec.figures.len());
        for (index, figure) in spec.figures.iter().enumerate() {
            self.path = format!("{path}figures[{index}].");
            let tags = match figure {
                None => FigureTags::Untagged,
                Some(figure) => self.figure(figure, depth, prefix, counter)?,
            };
            figures.push(tags);
        }
        self.path = path;
        let insets = self.insets(&spec.insets, depth, prefix, counter)?;
        Ok(FigureTags::Grid(figures, insets))
    }

    fn figure(
        &mut self,
        figure: &Figure,
        depth: usize,
        prefix: &str,
        counter: &mut usize,
    ) -> Result<FigureTags> {
        match figure {
            Figure::Plot(plot) => {
                let tag = self.next(depth, prefix, counter);
                let insets = self.insets(&plot.insets, depth, prefix, counter)?;
                Ok(FigureTags::Plot(tag, insets))
            }
            Figure::Subplots(grid) => {
                if grid.layout.tag_levels.is_some() {
                    self.warnings.push(format!(
                        "at '{}layout.tag_levels': a grid inside a tagged grid is numbered \
                         by the outer grid's tag_levels; ignored (use tag_level='new' to \
                         number it at the next level)",
                        self.path
                    ));
                }
                let level = tag_level(grid).map_err(|error| error.nested(&self.path))?;
                if level == TagLevel::Unit {
                    return Ok(FigureTags::Unit(self.next(depth, prefix, counter)));
                }
                let new = level == TagLevel::New;
                if new && depth + 1 < self.levels.len() {
                    let prefix = self.next(depth, prefix, counter);
                    let mut inner = 0;
                    self.grid(grid, depth + 1, &prefix, &mut inner)
                } else {
                    if new {
                        self.warnings.push(format!(
                            "at '{}layout.tag_level': 'new' needs a level for it in the \
                             outer grid's tag_levels (it has {}); the numbering continues \
                             as with 'keep'",
                            self.path,
                            self.levels.len()
                        ));
                    }
                    self.grid(grid, depth, prefix, counter)
                }
            }
        }
    }

    fn insets(
        &mut self,
        insets: &[InsetSpec],
        depth: usize,
        prefix: &str,
        counter: &mut usize,
    ) -> Result<Vec<FigureTags>> {
        let path = self.path.clone();
        let mut tags = Vec::with_capacity(insets.len());
        for (index, inset) in insets.iter().enumerate() {
            self.path = format!("{path}insets[{index}].figure.");
            tags.push(if inset.ignore_tag.unwrap_or(false) {
                FigureTags::Untagged
            } else {
                self.figure(&inset.figure, depth, prefix, counter)?
            });
        }
        self.path = path;
        Ok(tags)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn numberings_follow_patchwork() {
        let first: Vec<String> = [0, 1, 25, 26, 27]
            .iter()
            .map(|index| Level::Upper.format(*index))
            .collect();
        assert_eq!(first, ["A", "B", "Z", "AA", "AB"]);
        assert_eq!(Level::Lower.format(2), "c");
        assert_eq!(Level::Number.format(9), "10");
        assert_eq!(Level::RomanLower.format(3), "iv");
        assert_eq!(Level::RomanUpper.format(13), "XIV");
        assert_eq!(roman(1994), "MCMXCIV");
    }

    #[test]
    fn unknown_levels_name_the_accepted_ones() {
        let error = Level::parse("B").expect_err("unknown");
        let message = error.to_string();
        assert!(message.contains("unknown tag level 'B'"), "{message}");
        assert!(message.contains("'A', 'a', '1', 'i' or 'I'"), "{message}");
    }

    fn grid(json: &str) -> SubplotsSpec {
        match Figure::from_json(json).expect("valid grid") {
            Figure::Subplots(grid) => *grid,
            Figure::Plot(_) => panic!("expected a grid"),
        }
    }

    #[test]
    fn unit_grids_take_one_tag_and_leave_their_plots_untagged() {
        let plot = r#"{"kind": "plot", "layers": []}"#;
        let unit = format!(
            r#"{{"kind": "subplots", "layout": {{"name": "grid", "ncol": 1, "nrow": 2,
                "tag_level": "unit"}}, "figures": [{plot}, {plot}]}}"#
        );
        let spec = grid(&format!(
            r#"{{"kind": "subplots", "layout": {{"name": "grid", "ncol": 3, "nrow": 1,
                "tag_levels": ["A"]}}, "figures": [{unit}, {plot}, {unit}]}}"#
        ));
        let mut warnings = Vec::new();
        let tags = grid_tags(&spec, &mut warnings).expect("tags");
        let expected = FigureTags::Grid(
            vec![
                FigureTags::Unit("A".into()),
                FigureTags::Plot("B".into(), Vec::new()),
                FigureTags::Unit("C".into()),
            ],
            Vec::new(),
        );
        assert_eq!(tags, Some(expected));
        assert!(warnings.is_empty(), "{warnings:?}");
    }

    #[test]
    fn unknown_tag_level_names_unit() {
        let spec = grid(
            r#"{"kind": "subplots", "layout": {"name": "grid", "ncol": 1, "nrow": 1,
                "tag_level": "one"}, "figures": [null]}"#,
        );
        let message = tag_level(&spec).expect_err("unknown").to_string();
        assert!(message.contains("'keep', 'new' or 'unit'"), "{message}");
    }
}

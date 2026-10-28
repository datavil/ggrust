//! The text of a layer's tooltips, per row (lets-plot `GeomInteractionUtil`,
//! `GeomInteractionBuilderUtil`, `LineSpecConfig`, `MappingField`,
//! `DataFrameField`, `TooltipFormatting`).
//!
//! Default lines: the aesthetics bound to a variable, except the axis ones (and
//! those bound to an axis' variable), hidden ones and discrete ones with fewer
//! than 5 levels, one line per variable, labelled by the scale name (a single
//! `x` or `y` line has no label). Values: positions in data units through the
//! scale (`format=`, else lets-plot's tooltip formatter: break labels for 100
//! breaks; category labels on discrete axes), other numbers `{,~g}`, categories
//! as they are. `layer_tooltips()` lines replace the default lines.

use std::collections::BTreeMap;

use serde::ser::{Serialize, SerializeStruct, Serializer};

use super::TooltipBehavior;
use super::template::{Field, Line, Part, format_field};
use crate::data::{Column, Computed, LayerData};
use crate::scale::PositionScale;
use crate::scale::format::{Label, format_with_step};
use crate::scale::label_pattern::{LabelPattern, PatternValue};
use crate::scene::Color;
use crate::spec::{Aesthetic, TooltipLines, TooltipsSpec};
use crate::style::Styles;

/// lets-plot `MIN_FACTORS_TO_SHOW_TOOLTIPS`: discrete variables with fewer levels
/// are left out of the default tooltip (the legend shows them).
const MIN_LEVELS_FOR_TOOLTIP: usize = 5;

/// lets-plot `TooltipFormatting.fromScale`: target break count of the tooltip
/// number formatter of continuous positions.
const TOOLTIP_BREAK_COUNT: usize = 100;

/// One text per layer row (`None`: nothing to show for that row), with an
/// optional label. Serialized dictionary-encoded: `values` (distinct texts) and
/// `codes` (index per row, -1 for none).
#[derive(Debug, Clone, Default, PartialEq)]
pub struct TextColumn {
    /// The line's label (bold, left), if any.
    pub label: Option<String>,
    /// The text of every row.
    pub values: Vec<Option<String>>,
}

impl Serialize for TextColumn {
    fn serialize<S: Serializer>(&self, serializer: S) -> Result<S::Ok, S::Error> {
        let mut distinct: Vec<&str> = Vec::new();
        let mut index_of: std::collections::HashMap<&str, i64> = Default::default();
        let codes: Vec<i64> = self
            .values
            .iter()
            .map(|value| match value {
                None => -1,
                Some(text) => *index_of.entry(text.as_str()).or_insert_with(|| {
                    distinct.push(text);
                    distinct.len() as i64 - 1
                }),
            })
            .collect();
        let mut state = serializer.serialize_struct("TextColumn", 3)?;
        state.serialize_field("label", &self.label)?;
        state.serialize_field("values", &distinct)?;
        state.serialize_field("codes", &codes)?;
        state.end()
    }
}

/// Everything the tooltips of one layer in one panel show.
#[derive(Debug, Clone, Default, PartialEq, serde::Serialize)]
pub struct LayerContent {
    /// Lines of the general tooltip, in order. A row's line with no text is left out.
    pub lines: Vec<TextColumn>,
    /// Title line (bold, centred).
    #[serde(skip_serializing_if = "Option::is_none")]
    pub title: Option<TextColumn>,
    /// Text of the horizontal (bottom) axis tooltip.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub axis_x: Option<TextColumn>,
    /// Text of the vertical (left) axis tooltip.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub axis_y: Option<TextColumn>,
    /// CSS color of the marker bar beside the lines.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub marker: Option<TextColumn>,
}

impl LayerContent {
    /// Whether no row has anything to show.
    pub fn is_empty(&self) -> bool {
        self.lines.is_empty()
            && self.title.is_none()
            && self.axis_x.is_none()
            && self.axis_y.is_none()
    }
}

/// What [`layer_content`] reads.
pub struct ContentSource<'a> {
    /// The layer's tooltips option.
    pub tooltips: Option<&'a TooltipsSpec>,
    /// The geom's behavior.
    pub behavior: TooltipBehavior,
    /// Layer rows before the position adjustment (values are shown unjittered,
    /// unstacked, as lets-plot).
    pub data: &'a LayerData,
    /// Resolved styles (marker colors).
    pub styles: &'a Styles,
    /// The panel's data scales (not swapped by `coord_flip`).
    pub x_scale: &'a PositionScale,
    /// See `x_scale`.
    pub y_scale: &'a PositionScale,
    /// Variable bound to each aesthetic: a column name or `..stat..`.
    pub bindings: &'a BTreeMap<Aesthetic, String>,
    /// Scale name of each bound aesthetic (the line label).
    pub labels: &'a BTreeMap<Aesthetic, String>,
    /// Unmapped columns the tooltips read, one value per layer row.
    pub columns: &'a [(String, Column)],
    /// `coord_flip`: data x is on the vertical axis.
    pub flipped: bool,
    /// Whether the screen's horizontal / vertical axis shows labels (lets-plot hides
    /// the axis tooltip otherwise).
    pub axis_labels: (bool, bool),
}

/// Where a field's values come from.
#[derive(Debug, Clone, Copy, PartialEq)]
enum Source {
    Aesthetic(Aesthetic),
    Computed(Computed),
    Column(usize),
}

/// The tooltip text of every row of a layer. Problems (unknown variables) are
/// pushed to `warnings` with `path`.
pub fn layer_content(
    source: &ContentSource,
    path: &str,
    warnings: &mut Vec<String>,
) -> LayerContent {
    let user = match source.tooltips {
        Some(TooltipsSpec::Disabled) => return LayerContent::default(),
        Some(TooltipsSpec::Lines(lines)) => Some(lines.as_ref()),
        None => None,
    };
    let has_user_lines = user.is_some_and(|user| user.lines.is_some() || user.variables.is_some());
    if source.behavior.needs_lines && !has_user_lines {
        return LayerContent::default();
    }
    let builder = Builder {
        source,
        formats: user
            .map(|user| formats(user, path, warnings))
            .unwrap_or_default(),
    };
    let mut content = LayerContent::default();
    if has_user_lines {
        let user = user.expect("user lines imply layer_tooltips()");
        for name in user.variables.iter().flatten() {
            let field = Field::Variable(name.clone());
            match builder.resolve(&field) {
                Some(found) => content.lines.push(TextColumn {
                    label: Some(builder.label_of(found, name)),
                    values: builder.values(found),
                }),
                None => warn_unknown(&field, path, warnings),
            }
        }
        for line in user.lines.iter().flatten() {
            if let Some(column) = builder.line(&Line::parse(line), path, warnings) {
                content.lines.push(column);
            }
        }
        // Side statistics not shown by the user's lines (lets-plot keeps them as
        // side tooltips unless `disable_splitting()`).
        if !user.disable_splitting.unwrap_or(false) {
            for &aesthetic in source.behavior.side {
                let used = user.lines.iter().flatten().any(|line| {
                    Line::parse(line)
                        .fields()
                        .any(|field| builder.resolve(field) == Some(Source::Aesthetic(aesthetic)))
                });
                if !used && source.data.contains(aesthetic) {
                    content.lines.push(builder.default_line(aesthetic, false));
                }
            }
        }
    } else {
        let aesthetics = builder.default_aesthetics();
        let general = aesthetics
            .iter()
            .filter(|aesthetic| !source.behavior.side.contains(aesthetic))
            .count();
        for &aesthetic in &aesthetics {
            let unlabelled = general == 1 && matches!(aesthetic, Aesthetic::X | Aesthetic::Y);
            content
                .lines
                .push(builder.default_line(aesthetic, unlabelled));
        }
    }
    if let Some(title) = user.and_then(|user| user.title.as_deref()) {
        content.title = builder.line(&Line::parse(title), path, warnings);
    }
    let (x_on_screen_x, axis_labels) = (!source.flipped, source.axis_labels);
    for (aesthetic, shown) in [
        (Aesthetic::X, source.behavior.axis.0),
        (Aesthetic::Y, source.behavior.axis.1),
    ] {
        if !shown || !source.data.contains(aesthetic) {
            continue;
        }
        let horizontal = (aesthetic == Aesthetic::X) == x_on_screen_x;
        let visible = if horizontal {
            axis_labels.0
        } else {
            axis_labels.1
        };
        if !visible {
            continue;
        }
        let column = TextColumn {
            label: None,
            values: builder.aesthetic_values(aesthetic, true),
        };
        if horizontal {
            content.axis_x = Some(column);
        } else {
            content.axis_y = Some(column);
        }
    }
    if !content.lines.is_empty() {
        content.marker = builder.marker();
    }
    content
}

fn warn_unknown(field: &Field, path: &str, warnings: &mut Vec<String>) {
    let name = match field {
        Field::Variable(name) => format!("@{name}"),
        Field::Aesthetic(name) => format!("^{name}"),
    };
    warnings.push(format!(
        "at '{path}.tooltips': '{name}' is not a variable of the layer's data; the tooltip \
         line was left out"
    ));
}

/// `format()` patterns by field. `^X` / `^Y` apply to every x / y position aesthetic.
fn formats(
    user: &TooltipLines,
    path: &str,
    warnings: &mut Vec<String>,
) -> Vec<(FormatKey, LabelPattern)> {
    let mut result = Vec::new();
    for (index, format) in user.formats.iter().enumerate() {
        let pattern = match LabelPattern::parse(&format.format) {
            Ok(pattern) => pattern,
            Err(message) => {
                warnings.push(format!(
                    "at '{path}.tooltips.formats[{index}].format': {message}; the default \
                     format is used"
                ));
                continue;
            }
        };
        let key = match format_field(&format.field) {
            Field::Aesthetic(name) if name == "X" => FormatKey::AllX,
            Field::Aesthetic(name) if name == "Y" => FormatKey::AllY,
            Field::Aesthetic(name) => match name.parse::<Aesthetic>().ok() {
                Some(aesthetic) => FormatKey::Aesthetic(aesthetic),
                None => {
                    warnings.push(format!(
                        "at '{path}.tooltips.formats[{index}].field': unknown aesthetic \
                         '^{name}'; the format was ignored"
                    ));
                    continue;
                }
            },
            Field::Variable(name) => FormatKey::Variable(name),
        };
        result.push((key, pattern));
    }
    result
}

#[derive(Debug, Clone, PartialEq)]
enum FormatKey {
    Aesthetic(Aesthetic),
    AllX,
    AllY,
    Variable(String),
}

struct Builder<'a> {
    source: &'a ContentSource<'a>,
    formats: Vec<(FormatKey, LabelPattern)>,
}

impl Builder<'_> {
    /// The aesthetics of the default tooltip (lets-plot `createTooltipAesList`),
    /// side statistics first (merged as `disable_splitting`).
    fn default_aesthetics(&self) -> Vec<Aesthetic> {
        let source = self.source;
        let behavior = source.behavior;
        let axis: Vec<Aesthetic> = [
            (Aesthetic::X, behavior.axis.0),
            (Aesthetic::Y, behavior.axis.1),
        ]
        .into_iter()
        .filter_map(|(aesthetic, shown)| shown.then_some(aesthetic))
        .collect();
        let axis_variables: Vec<&String> = axis
            .iter()
            .filter_map(|aesthetic| source.bindings.get(aesthetic))
            .collect();
        let mut shown: Vec<Aesthetic> = behavior
            .side
            .iter()
            .copied()
            .filter(|aesthetic| source.data.contains(*aesthetic))
            .collect();
        let mut seen_variables: Vec<&String> = Vec::new();
        for (&aesthetic, variable) in source.bindings {
            let candidate = !axis.contains(&aesthetic)
                && !axis_variables.contains(&variable)
                && !behavior.hidden.contains(&aesthetic)
                && !behavior.side.contains(&aesthetic)
                && aesthetic != Aesthetic::Group
                && source.data.contains(aesthetic)
                && self.enough_levels(aesthetic);
            if !candidate {
                continue;
            }
            if let Some(position) = seen_variables.iter().position(|seen| *seen == variable) {
                // The same variable twice: keep the continuous mapping.
                let kept = shown.len() - seen_variables.len() + position;
                if self.is_discrete(shown[kept]) && !self.is_discrete(aesthetic) {
                    shown[kept] = aesthetic;
                }
                continue;
            }
            seen_variables.push(variable);
            shown.push(aesthetic);
        }
        // Positions first, then the rest in a stable order (lets-plot: rendered order).
        let side_count = shown.len() - seen_variables.len();
        shown[side_count..].sort_by_key(|aesthetic| rendered_rank(*aesthetic));
        shown
    }

    fn is_discrete(&self, aesthetic: Aesthetic) -> bool {
        if aesthetic.is_x_position() {
            return self.source.x_scale.is_discrete();
        }
        if aesthetic.is_y_position() {
            return self.source.y_scale.is_discrete();
        }
        matches!(self.source.data.get(aesthetic), Some(Column::Discrete(_)))
    }

    fn enough_levels(&self, aesthetic: Aesthetic) -> bool {
        if !self.is_discrete(aesthetic) {
            return true;
        }
        let levels = if aesthetic.is_x_position() || aesthetic.is_y_position() {
            let scale = if aesthetic.is_x_position() {
                self.source.x_scale
            } else {
                self.source.y_scale
            };
            scale.breaks(0).positions.len()
        } else {
            match self.source.data.get(aesthetic) {
                Some(Column::Discrete(column)) => column.levels().len(),
                _ => 0,
            }
        };
        levels >= MIN_LEVELS_FOR_TOOLTIP
    }

    fn default_line(&self, aesthetic: Aesthetic, unlabelled: bool) -> TextColumn {
        TextColumn {
            label: Some(if unlabelled {
                String::new()
            } else {
                self.aesthetic_label(aesthetic)
            }),
            values: self.aesthetic_values(aesthetic, false),
        }
    }

    fn aesthetic_label(&self, aesthetic: Aesthetic) -> String {
        if let Some(label) = self.source.labels.get(&aesthetic) {
            return label.clone();
        }
        match aesthetic {
            Aesthetic::Ymax => "y max".into(),
            Aesthetic::Ymin => "y min".into(),
            Aesthetic::Xmax => "x max".into(),
            Aesthetic::Xmin => "x min".into(),
            other => match self.source.bindings.get(&other) {
                Some(variable) => Computed::from_mapping(variable)
                    .map_or_else(|| variable.clone(), |computed| computed.as_str().into()),
                None => other.as_str().into(),
            },
        }
    }

    /// The label a `variables` line gets: the scale name of a mapped variable,
    /// else the variable name.
    fn label_of(&self, found: Source, name: &str) -> String {
        match found {
            Source::Aesthetic(aesthetic) => self.aesthetic_label(aesthetic),
            Source::Computed(computed) => computed.as_str().into(),
            Source::Column(_) => name.into(),
        }
    }

    /// lets-plot `LineSpecConfig.getValueSource`: a variable bound to aesthetics
    /// reads the first of them (by name, preferring one with a format); stat
    /// variables otherwise read the stat's column; other variables the data.
    fn resolve(&self, field: &Field) -> Option<Source> {
        let source = self.source;
        match field {
            Field::Aesthetic(name) => {
                let aesthetic = name.parse::<Aesthetic>().ok()?;
                source
                    .data
                    .contains(aesthetic)
                    .then_some(Source::Aesthetic(aesthetic))
            }
            Field::Variable(name) => {
                let mut bound: Vec<Aesthetic> = source
                    .bindings
                    .iter()
                    .filter(|(aesthetic, variable)| {
                        *variable == name && source.data.contains(**aesthetic)
                    })
                    .map(|(aesthetic, _)| *aesthetic)
                    .collect();
                bound.sort_by_key(|aesthetic| aesthetic.as_str());
                let with_format = bound
                    .iter()
                    .find(|aesthetic| self.own_format(**aesthetic).is_some());
                if let Some(aesthetic) = with_format.or(bound.first()) {
                    return Some(Source::Aesthetic(*aesthetic));
                }
                if let Some(computed) = Computed::from_mapping(name) {
                    return source
                        .data
                        .contains(computed)
                        .then_some(Source::Computed(computed));
                }
                source
                    .columns
                    .iter()
                    .position(|(column, _)| column == name)
                    .map(Source::Column)
            }
        }
    }

    /// A template line: `None` when a `^aes` field is not mapped (lets-plot drops
    /// the line) or a variable is unknown (warned). Rows where a data field has no
    /// value get no line.
    fn line(&self, line: &Line, path: &str, warnings: &mut Vec<String>) -> Option<TextColumn> {
        let mut columns = Vec::new();
        for field in line.fields() {
            match self.resolve(field) {
                Some(found) => columns.push(self.values(found)),
                None => {
                    if let Field::Variable(_) = field {
                        warn_unknown(field, path, warnings);
                    }
                    return None;
                }
            }
        }
        let rows = self.source.data.row_count();
        let values = (0..rows)
            .map(|row| {
                let mut text = String::new();
                let mut fields = columns.iter();
                for part in &line.parts {
                    match part {
                        Part::Text(literal) => text.push_str(literal),
                        Part::Field(_) => {
                            let value = fields.next()?.get(row)?.as_deref()?;
                            text.push_str(value);
                        }
                    }
                }
                Some(text)
            })
            .collect();
        Some(TextColumn {
            label: line.label.clone(),
            values,
        })
    }

    fn values(&self, found: Source) -> Vec<Option<String>> {
        match found {
            Source::Aesthetic(aesthetic) => self.aesthetic_values(aesthetic, false),
            Source::Computed(computed) => {
                let format = match computed {
                    Computed::Proportion | Computed::SumProportion => "{.2f}",
                    Computed::ProportionPercent | Computed::SumPercent => "{.1f} %",
                    _ => "{,~g}",
                };
                let pattern = self
                    .variable_format(&format!("..{}..", computed.as_str()))
                    .cloned()
                    .or_else(|| LabelPattern::parse(format).ok());
                column_texts(self.source.data.get(computed), pattern.as_ref())
            }
            Source::Column(index) => {
                let (name, column) = &self.source.columns[index];
                let pattern = self
                    .variable_format(name)
                    .cloned()
                    .or_else(|| LabelPattern::parse("{,~g}").ok());
                column_texts(Some(column), pattern.as_ref())
            }
        }
    }

    fn own_format(&self, aesthetic: Aesthetic) -> Option<&LabelPattern> {
        let mut all = None;
        for (key, pattern) in &self.formats {
            match key {
                FormatKey::Aesthetic(candidate) if *candidate == aesthetic => {
                    return Some(pattern);
                }
                FormatKey::AllX if aesthetic.is_x_position() && all.is_none() => {
                    all = Some(pattern)
                }
                FormatKey::AllY if aesthetic.is_y_position() && all.is_none() => {
                    all = Some(pattern)
                }
                _ => {}
            }
        }
        all
    }

    fn variable_format(&self, name: &str) -> Option<&LabelPattern> {
        self.formats.iter().find_map(|(key, pattern)| match key {
            FormatKey::Variable(variable) if variable == name => Some(pattern),
            _ => None,
        })
    }

    /// Values of an aesthetic: its own format, else its variable's format, else
    /// the scale's tooltip formatter (positions) or `{,~g}` / the category.
    /// Axis tooltips ignore `format()` (lets-plot formats them by the scale).
    fn aesthetic_values(&self, aesthetic: Aesthetic, axis: bool) -> Vec<Option<String>> {
        let source = self.source;
        let format = if axis {
            None
        } else {
            self.own_format(aesthetic).or_else(|| {
                source
                    .bindings
                    .get(&aesthetic)
                    .and_then(|variable| self.variable_format(variable))
            })
        };
        let rows = source.data.row_count();
        let position_scale = if aesthetic.is_x_position() {
            Some(source.x_scale)
        } else if aesthetic.is_y_position() {
            Some(source.y_scale)
        } else {
            None
        };
        let Some(scale) = position_scale else {
            let default = LabelPattern::parse("{,~g}").ok();
            let mut values = column_texts(source.data.get(aesthetic), format.or(default.as_ref()));
            if !axis {
                for value in &mut values {
                    value.get_or_insert_with(|| "n/a".into());
                }
            }
            return values;
        };
        let Some(column) = source.data.continuous(aesthetic) else {
            return vec![None; rows];
        };
        let formatter = PositionFormatter::new(scale);
        (0..rows)
            .map(|row| {
                let text = column
                    .get(row)
                    .filter(|value| value.is_finite())
                    .and_then(|value| formatter.format(value, format));
                match (text, axis) {
                    (Some(text), _) => Some(text),
                    (None, true) => None,
                    (None, false) => Some("n/a".into()),
                }
            })
            .collect()
    }

    /// lets-plot `HintColorUtil.markerFactory` (simplified): the fill when `fill`
    /// is mapped, else the color when `color` is mapped; always the stroke color for
    /// lines and paths.
    fn marker(&self) -> Option<TextColumn> {
        let source = self.source;
        let mapped = |aesthetic: Aesthetic| source.data.contains(aesthetic);
        let aesthetic = if source.behavior.marker_is_stroke {
            Aesthetic::Color
        } else if mapped(Aesthetic::Fill) && source.styles.colors(Aesthetic::Fill).is_ok() {
            Aesthetic::Fill
        } else if mapped(Aesthetic::Color) {
            Aesthetic::Color
        } else {
            return None;
        };
        let colors = source.styles.colors(aesthetic).ok()?;
        let values = (0..source.data.row_count())
            .map(|row| {
                let color = colors.get(row);
                (color.alpha > 0).then(|| css_color(color))
            })
            .collect();
        Some(TextColumn {
            label: None,
            values,
        })
    }
}

/// Order of aesthetics in the default tooltip (lets-plot renders positions first).
fn rendered_rank(aesthetic: Aesthetic) -> (u8, &'static str) {
    let group = if aesthetic.is_x_position() {
        0
    } else if aesthetic.is_y_position() {
        1
    } else {
        2
    };
    (group, aesthetic.as_str())
}

/// Texts of a data column: categories as they are, numbers through `pattern`.
fn column_texts(column: Option<&Column>, pattern: Option<&LabelPattern>) -> Vec<Option<String>> {
    match column {
        None => Vec::new(),
        Some(Column::Discrete(column)) => (0..column.len())
            .map(|row| {
                column.label(row).map(|label| match pattern {
                    Some(pattern) => label_text(&pattern.format(PatternValue::Text(label))),
                    None => label.to_owned(),
                })
            })
            .collect(),
        Some(Column::Continuous(column)) => (0..column.len())
            .map(|row| {
                column
                    .get(row)
                    .filter(|value| !value.is_nan())
                    .map(|value| match pattern {
                        Some(pattern) => label_text(&pattern.format(PatternValue::Number(value))),
                        None => value.to_string(),
                    })
            })
            .collect(),
    }
}

/// Positions in axis units to text, as lets-plot's tooltip formatter of the scale.
struct PositionFormatter<'a> {
    scale: &'a PositionScale,
    /// Discrete axes: category label by position.
    categories: Vec<(f64, String)>,
    /// Continuous axes: the magnitude and step the default format is chosen for.
    reference_and_step: (f64, f64),
}

impl<'a> PositionFormatter<'a> {
    fn new(scale: &'a PositionScale) -> Self {
        match scale {
            PositionScale::Discrete(_) => {
                let breaks = scale.breaks(0);
                PositionFormatter {
                    scale,
                    categories: breaks
                        .positions
                        .iter()
                        .zip(&breaks.labels)
                        .map(|(position, label)| (*position, label_text(label)))
                        .collect(),
                    reference_and_step: (0.0, 0.0),
                }
            }
            PositionScale::Continuous(continuous) => {
                let (low, high) = continuous.limited_domain();
                let transform = continuous.transform;
                let (data_low, data_high) = (transform.inverse(low), transform.inverse(high));
                let reference = data_low.abs().max(data_high.abs());
                let step = crate::scale::breaks::nice_step(
                    (data_high - data_low).abs(),
                    TOOLTIP_BREAK_COUNT,
                );
                PositionFormatter {
                    scale,
                    categories: Vec::new(),
                    reference_and_step: (reference, step),
                }
            }
        }
    }

    fn format(&self, position: f64, pattern: Option<&LabelPattern>) -> Option<String> {
        match self.scale {
            PositionScale::Discrete(_) => {
                let label = self
                    .categories
                    .iter()
                    .find(|(at, _)| (at - position).abs() < 1e-9)
                    .map(|(_, label)| label.clone());
                match (label, pattern) {
                    (Some(label), Some(pattern)) => {
                        Some(label_text(&pattern.format(PatternValue::Text(&label))))
                    }
                    (Some(label), None) => Some(label),
                    // Numbers on a discrete axis (lets-plot `FALLBACK_NUMBER_FORMATTER`).
                    (None, _) => Some(format!("{position:.2}")),
                }
            }
            PositionScale::Continuous(continuous) => {
                let value = continuous.transform.inverse(position);
                if !value.is_finite() {
                    return None;
                }
                let pattern = pattern.or(continuous.format.as_ref());
                Some(match pattern {
                    Some(pattern) => label_text(&pattern.format(PatternValue::Number(value))),
                    None => {
                        let (reference, step) = self.reference_and_step;
                        label_text(&format_with_step(value, reference, step))
                    }
                })
            }
        }
    }
}

/// A label as plain text, the exponent in Unicode superscripts (`2·10⁶`).
fn label_text(label: &Label) -> String {
    let mut text = label.text.clone();
    if let Some(superscript) = &label.superscript {
        text.extend(superscript.chars().map(|character| match character {
            '0' => '⁰',
            '1' => '¹',
            '2' => '²',
            '3' => '³',
            '4' => '⁴',
            '5' => '⁵',
            '6' => '⁶',
            '7' => '⁷',
            '8' => '⁸',
            '9' => '⁹',
            '-' | '\u{2212}' => '⁻',
            other => other,
        }));
    }
    if let Some(suffix) = &label.suffix {
        text.push_str(suffix);
    }
    text
}

fn css_color(color: Color) -> String {
    if color.alpha == 255 {
        format!("#{:02x}{:02x}{:02x}", color.red, color.green, color.blue)
    } else {
        format!(
            "rgba({},{},{},{:.3})",
            color.red,
            color.green,
            color.blue,
            f64::from(color.alpha) / 255.0
        )
    }
}

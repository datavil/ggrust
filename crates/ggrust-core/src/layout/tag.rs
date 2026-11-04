//! Plot tags (`labs(tag=...)`, lets-plot `plot_tag*` theme options).
//!
//! What it does: measure a tag's box (its lines, one text band each, plus the
//! `plot_tag` margins), reserve bands of the plot margin for it
//! (`plot_tag_location="margin"`), and place and draw it in the plot, its panels
//! or the reserved band. Rules measured against lets-plot 4.11 `to_svg()`: the box
//! sits at the named side or corner of its area (`[x, y]`: centred there, kept
//! inside), and the text is placed in the box by `hjust`/`vjust`, so the margins
//! only widen the box, as lets-plot.
//!
//! What it does not do: rotate tags (lets-plot ignores `angle` too) or number
//! them across a grid (see [`crate::grid`]).

use crate::scene::{HorizontalAlign, Node, Rect, Text, VerticalAlign};
use crate::text::measure;
use crate::theme::{Margins, TagLocation, TagPosition, Theme};

use super::text_band;

/// The tag as drawn: prefix, text and suffix; `None` when blank or absent.
pub fn tag_text(theme: &Theme, tag: &Option<String>) -> Option<String> {
    // lets-plot draws the affixes around an empty tag ("()").
    let tag = tag.as_deref()?;
    theme.plot_tag?;
    let (prefix, suffix) = &theme.plot_tag_affixes;
    Some(format!("{prefix}{tag}{suffix}")).filter(|text| !text.is_empty())
}

/// The tag's box: the widest line by the lines' bands, plus the margins.
fn box_size(theme: &Theme, text: &str) -> Option<(f64, f64)> {
    let style = theme.plot_tag?;
    let margin = theme.plot_tag_margin;
    let lines: Vec<&str> = text.split('\n').collect();
    let width = lines
        .iter()
        .map(|line| measure(line, style.size, style.font).width)
        .fold(0.0, f64::max);
    let height = text_band(style) * lines.len() as f64;
    Some((
        width + margin.left + margin.right,
        height + margin.top + margin.bottom,
    ))
}

/// Bands of the plot (inside its margin) the tag reserves: with
/// `plot_tag_location="margin"` and a named position, its box's width on the left
/// or right and its height on the top or bottom, by the sides it touches
/// (`top-left` reserves both). Nothing otherwise.
pub fn tag_bands(theme: &Theme, tag: &Option<String>) -> Margins {
    let mut bands = Margins::uniform(0.0);
    let (TagLocation::Margin, TagPosition::Named(x, y)) =
        (theme.plot_tag_location, theme.plot_tag_position)
    else {
        return bands;
    };
    let Some((width, height)) = tag_text(theme, tag).and_then(|text| box_size(theme, &text)) else {
        return bands;
    };
    if x == 0.0 {
        bands.left = width;
    } else if x == 1.0 {
        bands.right = width;
    }
    if y == 1.0 {
        bands.top = height;
    } else if y == 0.0 {
        bands.bottom = height;
    }
    bands
}

/// The tag's nodes. `plot` is the plot inside its margin (the tag's area for the
/// `plot` and `margin` locations), `panel` the panels.
pub fn tag_nodes(theme: &Theme, tag: &Option<String>, plot: Rect, panel: Rect) -> Vec<Node> {
    let (Some(style), Some(text)) = (theme.plot_tag, tag_text(theme, tag)) else {
        return Vec::new();
    };
    let Some((width, height)) = box_size(theme, &text) else {
        return Vec::new();
    };
    let area = match theme.plot_tag_location {
        TagLocation::Panel => panel,
        TagLocation::Plot | TagLocation::Margin => plot,
    };
    let (left, top) = match theme.plot_tag_position {
        TagPosition::Named(x, y) => (
            area.x + x * (area.width - width),
            area.y + (1.0 - y) * (area.height - height),
        ),
        TagPosition::Point(x, y) => {
            let keep_in = |centre: f64, extent: f64, start: f64, span: f64| {
                (centre - extent / 2.0)
                    .min(start + span - extent)
                    .max(start)
            };
            (
                keep_in(area.x + x * area.width, width, area.x, area.width),
                keep_in(
                    area.y + (1.0 - y) * area.height,
                    height,
                    area.y,
                    area.height,
                ),
            )
        }
    };
    // The text block inside the box, by the element's justification.
    let band = text_band(style);
    let lines: Vec<&str> = text.split('\n').collect();
    let block_height = band * lines.len() as f64;
    let justify = |value: f64, default: f64| if value.is_finite() { value } else { default };
    let block_top = top + (1.0 - justify(style.vjust, 0.5)) * (height - block_height);
    let hjust = justify(style.hjust, 0.5);
    lines
        .iter()
        .enumerate()
        .map(|(index, line)| {
            let line_width = measure(line, style.size, style.font).width;
            Node::Text(Text {
                content: (*line).to_owned(),
                superscript: None,
                suffix: None,
                x: left + hjust * (width - line_width) + line_width / 2.0,
                y: block_top + band * (index as f64 + 0.5),
                size: style.size,
                font: style.font,
                color: style.color,
                horizontal: HorizontalAlign::Middle,
                vertical: VerticalAlign::Middle,
                angle: 0.0,
                halo: None,
            })
        })
        .collect()
}

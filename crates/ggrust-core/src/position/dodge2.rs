//! `position="dodge2"`: ggplot2's `position_dodge2`, the boxplot dodge. Elements
//! (boxes, bars) whose x spans overlap form one dodge group; inside it they are
//! laid side by side in x then group order (`reverse`: groups backwards), each
//! as wide as its span divided by the elements in its group (`preserve="total"`)
//! or by the most elements at any x (`preserve="single"`), the group centred on
//! its overall span. Where some group holds two or more elements every element
//! shrinks by `padding` (default 0.1) around its centre.
//!
//! An element is the rows sharing a span and a group (a box's outlier rows move
//! with it). Without `xmin`/`xmax` the span is `x` +- `width` / 2 times the x
//! resolution (`width` as `position_dodge`; a row's own `width` column first, as
//! boxplot outliers under `varwidth` carry), else `x` itself. Other x positions
//! move with `x`.
//!
//! What it does not do: vertical dodging.

use std::collections::HashMap;

use super::Position;
use super::shift::shift_positions;
use crate::data::{Column, ContinuousColumn, LayerData};
use crate::parameters::Parameters;
use crate::spec::Aesthetic;
use crate::{Error, Result};

/// ggplot2 `padding` default.
const DEFAULT_PADDING: f64 = 0.1;

/// Dodges overlapping elements, centred on their span.
pub struct Dodge2Position;

impl Position for Dodge2Position {
    fn name(&self) -> &'static str {
        "dodge2"
    }

    fn parameters(&self) -> &'static [&'static str] {
        &["width", "preserve", "padding", "reverse"]
    }

    fn adjust(&self, data: LayerData, parameters: &Parameters) -> Result<LayerData> {
        let padding = parameters.number("padding")?.unwrap_or(DEFAULT_PADDING);
        if !(0.0..=1.0).contains(&padding) {
            return Err(Error::spec(
                parameters.path_of("padding"),
                format!("expected a share between 0 and 1, got {padding}"),
            ));
        }
        let options = Dodge2 {
            width: parameters.number("width")?,
            single: preserve_single(parameters)?,
            padding,
            reverse: parameters.boolean("reverse")?.unwrap_or(false),
        };
        dodge2(data, &options)
    }
}

/// `preserve`: `"total"` (default) or `"single"`, shared with `position_dodge`.
pub(crate) fn preserve_single(parameters: &Parameters) -> Result<bool> {
    match parameters.string("preserve")? {
        None | Some("total") => Ok(false),
        Some("single") => Ok(true),
        Some(other) => Err(Error::spec(
            parameters.path_of("preserve"),
            format!(
                "unknown preserve '{other}'; expected 'total' or 'single'{}",
                crate::suggest::did_you_mean(other, ["total", "single"])
            ),
        )),
    }
}

struct Dodge2 {
    width: Option<f64>,
    single: bool,
    padding: f64,
    reverse: bool,
}

/// One dodged element: the rows sharing a span and a group.
struct Element {
    rows: Vec<usize>,
    group: u32,
    center: f64,
    low: f64,
    high: f64,
}

fn dodge2(data: LayerData, options: &Dodge2) -> Result<LayerData> {
    let Some(x) = data.continuous(Aesthetic::X).cloned() else {
        return Ok(data);
    };
    let xmin = data.continuous(Aesthetic::Xmin).cloned();
    let xmax = data.continuous(Aesthetic::Xmax).cloned();
    let resolution = data.resolution(Aesthetic::X);
    // A row's own `width` (boxplot outliers under `varwidth`), else the option.
    let row_width = data.continuous(Aesthetic::Width).cloned();
    let span = |row: usize| -> Option<(f64, f64, f64)> {
        let center = x.get(row).filter(|value| value.is_finite())?;
        let (low, high) = match (&xmin, &xmax) {
            (Some(low), Some(high)) => (low.get(row)?, high.get(row)?),
            _ => {
                let width = row_width
                    .as_ref()
                    .and_then(|column| column.get(row))
                    .filter(|width| width.is_finite())
                    .or(options.width)
                    .unwrap_or(0.0);
                let half = width * resolution / 2.0;
                (center - half, center + half)
            }
        };
        (low.is_finite() && high.is_finite()).then_some((center, low, high))
    };
    // Elements by (span, group), in first-row order.
    let mut index_by_key: HashMap<(u64, u64, u32), usize> = HashMap::new();
    let mut elements: Vec<Element> = Vec::new();
    for row in 0..data.row_count() {
        let Some((center, low, high)) = span(row) else {
            continue;
        };
        let group = data.group_of(row);
        let key = (low.to_bits(), high.to_bits(), group);
        let index = *index_by_key.entry(key).or_insert_with(|| {
            elements.push(Element {
                rows: Vec::new(),
                group,
                center,
                low,
                high,
            });
            elements.len() - 1
        });
        elements[index].rows.push(row);
    }
    if elements.is_empty() {
        return Ok(data);
    }
    // ggplot2 `collide2`: by x, then group.
    elements.sort_by(|left, right| {
        let group = if options.reverse {
            right.group.cmp(&left.group)
        } else {
            left.group.cmp(&right.group)
        };
        left.center.total_cmp(&right.center).then(group)
    });
    let dodge_groups = overlap_groups(&elements);
    let group_count = dodge_groups.iter().max().map_or(0, |max| max + 1);
    let mut members = vec![0_usize; group_count];
    let mut span_low = vec![f64::INFINITY; group_count];
    let mut span_high = vec![f64::NEG_INFINITY; group_count];
    for (element, &group) in elements.iter().zip(&dodge_groups) {
        members[group] += 1;
        span_low[group] = span_low[group].min(element.low);
        span_high[group] = span_high[group].max(element.high);
    }
    // ggplot2 `preserve="single"`: the most elements at one x.
    let most_at_one_x = if options.single {
        let mut by_x: HashMap<u64, usize> = HashMap::new();
        for element in &elements {
            *by_x.entry(element.center.to_bits()).or_insert(0) += 1;
        }
        by_x.values().copied().max()
    } else {
        None
    };
    let new_width: Vec<f64> = elements
        .iter()
        .zip(&dodge_groups)
        .map(|(element, &group)| {
            let n = most_at_one_x.unwrap_or(members[group]) as f64;
            (element.high - element.low) / n
        })
        .collect();
    let mut total_width = vec![0.0; group_count];
    for (width, &group) in new_width.iter().zip(&dodge_groups) {
        total_width[group] += width;
    }
    let mut cursor: Vec<f64> = (0..group_count)
        .map(|group| (span_low[group] + span_high[group]) / 2.0 - total_width[group] / 2.0)
        .collect();
    let padded = members.iter().any(|count| *count > 1);
    let rows = data.row_count();
    let mut new_x: Vec<Option<f64>> = (0..rows).map(|row| x.get(row)).collect();
    let mut new_low: Vec<Option<f64>> = (0..rows)
        .map(|row| xmin.as_ref().and_then(|column| column.get(row)))
        .collect();
    let mut new_high: Vec<Option<f64>> = (0..rows)
        .map(|row| xmax.as_ref().and_then(|column| column.get(row)))
        .collect();
    let mut delta = vec![0.0; rows];
    for ((element, &group), width) in elements.iter().zip(&dodge_groups).zip(&new_width) {
        let low = cursor[group];
        cursor[group] += width;
        let center = low + width / 2.0;
        let half_width = if padded {
            width * (1.0 - options.padding) / 2.0
        } else {
            width / 2.0
        };
        for &row in &element.rows {
            delta[row] = center - element.center;
            new_x[row] = Some(center);
            new_low[row] = Some(center - half_width);
            new_high[row] = Some(center + half_width);
        }
    }
    let had_span = xmin.is_some() && xmax.is_some();
    let mut data = shift_positions(data, other_x_position, |row, value| value + delta[row])?;
    data.insert(
        Aesthetic::X,
        Column::Continuous(ContinuousColumn::from_options(new_x)),
    )?;
    if had_span {
        data.insert(
            Aesthetic::Xmin,
            Column::Continuous(ContinuousColumn::from_options(new_low)),
        )?;
        data.insert(
            Aesthetic::Xmax,
            Column::Continuous(ContinuousColumn::from_options(new_high)),
        )?;
    }
    Ok(data)
}

/// x positions other than `x`, `xmin` and `xmax` (set directly).
fn other_x_position(aesthetic: Aesthetic) -> bool {
    aesthetic.is_x_position()
        && !matches!(aesthetic, Aesthetic::X | Aesthetic::Xmin | Aesthetic::Xmax)
}

/// ggplot2 `find_x_overlaps`: a dodge group index per element (sorted by x), a
/// new group where an element starts past every earlier end (or at it, for an
/// element with a width).
fn overlap_groups(elements: &[Element]) -> Vec<usize> {
    let mut groups = Vec::with_capacity(elements.len());
    let mut group = 0;
    let mut end = f64::NEG_INFINITY;
    for (index, element) in elements.iter().enumerate() {
        if index == 0 {
            end = element.high;
        } else {
            let nonzero = element.high != element.low;
            if element.low > end || (element.low == end && nonzero) {
                group += 1;
            }
            end = end.max(element.high);
        }
        groups.push(group);
    }
    groups
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::spec::Options;

    fn data(x: &[f64], half: f64, groups: &[u32]) -> LayerData {
        let column = |values: Vec<f64>| Column::Continuous(ContinuousColumn::from_values(values));
        let mut data = LayerData::new(x.len());
        data.insert(Aesthetic::X, column(x.to_vec())).unwrap();
        data.insert(
            Aesthetic::Xmin,
            column(x.iter().map(|x| x - half).collect()),
        )
        .unwrap();
        data.insert(
            Aesthetic::Xmax,
            column(x.iter().map(|x| x + half).collect()),
        )
        .unwrap();
        data.set_group_index(Some(groups.to_vec())).unwrap();
        data
    }

    fn adjust(data: LayerData, options: &str) -> LayerData {
        let options: Options = serde_json::from_str(options).unwrap();
        Dodge2Position
            .adjust(data, &Parameters::new(&options, "layers[0].position"))
            .unwrap()
    }

    fn values(data: &LayerData, aesthetic: Aesthetic) -> Vec<f64> {
        let column = data.continuous(aesthetic).unwrap();
        (0..column.len())
            .map(|row| column.get(row).unwrap())
            .collect()
    }

    fn close(left: &[f64], right: &[f64]) -> bool {
        left.len() == right.len() && left.iter().zip(right).all(|(a, b)| (a - b).abs() < 1e-12)
    }

    #[test]
    fn matches_ggplot2_boxes() {
        // ggplot2 4.0.1: boxes 0.9 wide, groups a b at x 1, a at x 2.
        let dodged = adjust(data(&[1.0, 1.0, 2.0], 0.45, &[0, 1, 0]), "{}");
        assert!(close(&values(&dodged, Aesthetic::X), &[0.775, 1.225, 2.0]));
        assert!(close(
            &values(&dodged, Aesthetic::Xmin),
            &[0.5725, 1.0225, 1.595]
        ));
        let single = adjust(
            data(&[1.0, 1.0, 2.0], 0.45, &[0, 1, 0]),
            r#"{"preserve": "single"}"#,
        );
        assert!(close(
            &values(&single, Aesthetic::Xmin),
            &[0.5725, 1.0225, 1.7975]
        ));
        let reversed = adjust(data(&[1.0, 1.0], 0.45, &[0, 1]), r#"{"reverse": true}"#);
        assert!(close(&values(&reversed, Aesthetic::X), &[1.225, 0.775]));
    }

    #[test]
    fn rows_of_one_element_move_together_and_errors_name_the_option() {
        let column = |values: Vec<f64>| Column::Continuous(ContinuousColumn::from_values(values));
        let mut points = LayerData::new(3);
        points
            .insert(Aesthetic::X, column(vec![1.0, 1.0, 1.0]))
            .unwrap();
        points.set_group_index(Some(vec![0, 0, 1])).unwrap();
        let dodged = adjust(points, r#"{"width": 0.9}"#);
        assert!(close(
            &values(&dodged, Aesthetic::X),
            &[0.775, 0.775, 1.225]
        ));
        let options: Options = serde_json::from_str(r#"{"preserve": "singel"}"#).unwrap();
        let error = Dodge2Position
            .adjust(
                data(&[1.0], 0.45, &[0]),
                &Parameters::new(&options, "layers[0].position"),
            )
            .unwrap_err()
            .to_string();
        assert!(error.contains("did you mean 'single'"), "{error}");
    }
}

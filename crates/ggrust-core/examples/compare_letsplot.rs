//! Render a plot with ggrust next to lets-plot's own SVG of the same spec.
//!
//! Input: a directory holding `<case>.json` (`{"spec": as_dict() without data,
//! "data": {column: [numbers or strings]}}`) and `<case>_letsplot.svg` (lets-plot `to_svg()`).
//! Output: `<output>/compare_<case>.png`, lets-plot on the left, ggrust on the right.
//!
//! `cargo run --release --example compare_letsplot -- <input dir> <output dir> <case>...`

use std::fs;
use std::path::Path;
use std::sync::Arc;

use arrow_array::{ArrayRef, Float64Array, RecordBatch, StringArray};
use ggrust_core::build::{PlotData, build_plot};
use ggrust_core::data::DataFrame;
use ggrust_core::render::{svg_to_png, to_png};
use ggrust_core::spec::Figure;
use resvg::tiny_skia::{Color, Pixmap, PixmapPaint, Transform};

fn main() -> Result<(), Box<dyn std::error::Error>> {
    let arguments: Vec<String> = std::env::args().skip(1).collect();
    let [input, output, cases @ ..] = arguments.as_slice() else {
        return Err("usage: compare_letsplot <input dir> <output dir> <case>...".into());
    };
    fs::create_dir_all(output)?;
    for case in cases {
        let case_json: serde_json::Value = serde_json::from_str(&fs::read_to_string(
            Path::new(input).join(format!("{case}.json")),
        )?)?;
        let Figure::Plot(spec) = Figure::from_json(&case_json["spec"].to_string())? else {
            return Err("expected a single plot".into());
        };
        let columns: Vec<(String, ArrayRef)> = case_json["data"]
            .as_object()
            .ok_or("data must be an object")?
            .iter()
            .map(|(name, values)| {
                let values = values.as_array().cloned().unwrap_or_default();
                // A column with any string is a string column; otherwise numbers.
                let array: ArrayRef = if values.iter().any(serde_json::Value::is_string) {
                    let strings: Vec<Option<&str>> =
                        values.iter().map(serde_json::Value::as_str).collect();
                    Arc::new(StringArray::from(strings))
                } else {
                    let numbers: Vec<Option<f64>> =
                        values.iter().map(serde_json::Value::as_f64).collect();
                    Arc::new(Float64Array::from(numbers))
                };
                (name.clone(), array)
            })
            .collect();
        let batch = RecordBatch::try_from_iter(columns)?;
        let frame = DataFrame::new(batch.schema(), vec![batch])?;
        let built = build_plot(
            &spec,
            &PlotData {
                plot: Some(frame),
                layers: vec![],
            },
        )?;
        for warning in &built.warnings {
            eprintln!("{case}: warning: {warning}");
        }
        let (width, height) = (built.scene.width, built.scene.height);
        fs::write(
            Path::new(output).join(format!("compare_{case}_ggrust.svg")),
            ggrust_core::render::to_svg(&built.scene),
        )?;
        let ours = Pixmap::decode_png(&to_png(&built.scene, 2.0)?)?;
        let svg = fs::read_to_string(Path::new(input).join(format!("{case}_letsplot.svg")))?;
        let theirs = Pixmap::decode_png(&svg_to_png(&svg, width, height, 2.0)?)?;
        let gap = 20;
        let mut combined =
            Pixmap::new(ours.width() * 2 + gap, ours.height()).ok_or("allocation")?;
        combined.fill(Color::from_rgba8(200, 200, 200, 255));
        let paint = PixmapPaint::default();
        combined.draw_pixmap(0, 0, theirs.as_ref(), &paint, Transform::identity(), None);
        combined.draw_pixmap(
            (ours.width() + gap) as i32,
            0,
            ours.as_ref(),
            &paint,
            Transform::identity(),
            None,
        );
        let path = Path::new(output).join(format!("compare_{case}.png"));
        combined.save_png(&path)?;
        println!("wrote {}", path.display());
    }
    Ok(())
}

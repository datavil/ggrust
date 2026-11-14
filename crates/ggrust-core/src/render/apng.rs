//! Animated PNG: truecolour RGBA frames (no quantisation) drawn as PNG output and
//! written with the `png` crate's APNG support. Repeated frames (pauses) are one
//! frame with a longer delay. Loops forever. Frames are drawn on the rayon pool.

use super::animation::{FrameSource, encode_runs, runs};
use super::png::{pixel_size, render_pixmap};
use crate::{Error, Result};

/// Frame delays in milliseconds (the APNG delay is a u16 fraction of a second).
const DELAY_UNITS: f64 = 1000.0;

/// The animation as APNG bytes; `scale` multiplies the pixel size, `dpi` goes in
/// the `pHYs` chunk.
///
/// # Errors
///
/// A render error for an empty animation, a bad `dpi` or a failed encoding; any
/// error of drawing a frame.
pub fn to_apng(source: &dyn FrameSource, scale: f64, dpi: f64) -> Result<Vec<u8>> {
    if !(dpi.is_finite() && dpi > 0.0) {
        return Err(Error::render(format!("dpi must be positive, got {dpi}")));
    }
    let runs = runs(source, DELAY_UNITS);
    let Some(first) = runs.first() else {
        return Err(Error::render("the animation has no frames"));
    };
    let first_scene = source.scene(first.frame)?;
    let (width, height) = pixel_size(first_scene.width, first_scene.height, scale);
    let (width, height) = (width.max(1.0) as u32, height.max(1.0) as u32);
    let metadata = first_scene.metadata;
    drop(first_scene);
    let failed =
        |error: ::png::EncodingError| Error::render(format!("APNG encoding failed: {error}"));
    let mut bytes = Vec::new();
    let mut encoder = ::png::Encoder::new(&mut bytes, width, height);
    encoder.set_color(::png::ColorType::Rgba);
    encoder.set_depth(::png::BitDepth::Eight);
    encoder.set_compression(::png::Compression::Fast);
    let per_meter = (dpi / 0.0254).round() as u32;
    encoder.set_pixel_dims(Some(::png::PixelDimensions {
        xppu: per_meter,
        yppu: per_meter,
        unit: ::png::Unit::Meter,
    }));
    if metadata {
        encoder
            .add_text_chunk("Software".into(), super::GENERATOR.into())
            .map_err(failed)?;
    }
    encoder.set_animated(runs.len() as u32, 0).map_err(failed)?;
    let mut writer = encoder.write_header().map_err(failed)?;
    encode_runs(
        source,
        &runs,
        |scene| {
            let pixmap = render_pixmap(scene, scale)?;
            if (pixmap.width(), pixmap.height()) != (width, height) {
                return Err(Error::render(
                    "every frame of an animation must have the same size",
                ));
            }
            Ok(pixmap
                .pixels()
                .iter()
                .flat_map(|pixel| {
                    let color = pixel.demultiply();
                    [color.red(), color.green(), color.blue(), color.alpha()]
                })
                .collect::<Vec<u8>>())
        },
        |run, data| {
            let delay = run.delay.min(u64::from(u16::MAX)) as u16;
            writer
                .set_frame_delay(delay, DELAY_UNITS as u16)
                .map_err(failed)?;
            writer.write_image_data(&data).map_err(failed)
        },
    )?;
    writer.finish().map_err(failed)?;
    Ok(bytes)
}

#[cfg(test)]
mod tests {
    use super::super::animation::tests::Squares;
    use super::*;

    #[test]
    fn frames_decode_with_their_delays() {
        let source = Squares {
            timeline: vec![0, 0, 0, 2],
            fps: 10.0,
        };
        let bytes = to_apng(&source, 1.0, 96.0).unwrap();
        let decoder = ::png::Decoder::new(std::io::Cursor::new(bytes));
        let mut reader = decoder.read_info().unwrap();
        let animation = reader.info().animation_control().copied().unwrap();
        assert_eq!(animation.num_frames, 2);
        assert_eq!(animation.num_plays, 0);
        let mut delays = Vec::new();
        let mut pixels = Vec::new();
        for _ in 0..2 {
            let mut buffer = vec![0; reader.output_buffer_size().unwrap()];
            reader.next_frame(&mut buffer).unwrap();
            let control = reader.info().frame_control().copied().unwrap();
            delays.push((control.delay_num, control.delay_den));
            pixels.push(buffer);
        }
        assert_eq!(delays, vec![(300, 1000), (100, 1000)]);
        let at = |pixels: &[u8], x: usize| pixels[(4 * 20 + x) * 4];
        // Frame 0 has its square at x 0..4, frame 2 at 4..8.
        assert!(at(&pixels[0], 1) > 150 && at(&pixels[0], 1) < 255);
        assert_eq!(at(&pixels[1], 1), 255);
        assert!(at(&pixels[1], 5) < 255);
    }
}

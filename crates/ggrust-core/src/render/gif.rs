//! Animated GIF: every distinct frame rasterised (as PNG output), flattened onto
//! white (GIF has no partial transparency), quantised to its own 256-colour
//! palette (NeuQuant, the `gif` crate's `color_quant`) and LZW-encoded on the
//! rayon pool. Repeated frames (pauses) are one GIF frame with a longer delay.
//! Loops forever, as gganimate's gifski output.

use super::animation::{FrameSource, encode_runs, runs};
use super::png::{pixel_size, render_pixmap};
use crate::{Error, Result};

/// NeuQuant sampling: 1 is the slowest and best, 30 the fastest. 10 is the
/// `gif` crate's own suggestion for good quality at reasonable speed.
const QUANTIZE_SPEED: i32 = 10;

/// The animation as GIF bytes; `scale` multiplies the pixel size.
///
/// # Errors
///
/// A render error for an empty animation, a frame that does not fit GIF (1 to
/// 65535 pixels per side) or a failed encoding; any error of drawing a frame.
pub fn to_gif(source: &dyn FrameSource, scale: f64) -> Result<Vec<u8>> {
    let runs = runs(source, 100.0);
    let Some(first) = runs.first() else {
        return Err(Error::render("the animation has no frames"));
    };
    let first_scene = source.scene(first.frame)?;
    let (pixel_width, pixel_height) = pixel_size(first_scene.width, first_scene.height, scale);
    drop(first_scene);
    let fits = |pixels: f64| (1.0..=f64::from(u16::MAX)).contains(&pixels);
    if !(fits(pixel_width) && fits(pixel_height)) {
        return Err(Error::render(format!(
            "a GIF frame of {pixel_width} x {pixel_height} pixels does not fit GIF's 1 to \
             65535 pixels per side; change the size or scale"
        )));
    }
    let (width, height) = (pixel_width as u16, pixel_height as u16);
    let failed = |error: gif::EncodingError| Error::render(format!("GIF encoding failed: {error}"));
    let mut bytes = Vec::new();
    {
        let mut encoder = gif::Encoder::new(&mut bytes, width, height, &[]).map_err(failed)?;
        encoder.set_repeat(gif::Repeat::Infinite).map_err(failed)?;
        encode_runs(
            source,
            &runs,
            |scene| {
                let pixmap = render_pixmap(scene, scale)?;
                if (pixmap.width(), pixmap.height()) != (u32::from(width), u32::from(height)) {
                    return Err(Error::render(
                        "every frame of an animation must have the same size",
                    ));
                }
                let mut rgba = flattened_rgba(pixmap.data());
                let mut frame =
                    gif::Frame::from_rgba_speed(width, height, &mut rgba, QUANTIZE_SPEED);
                frame.make_lzw_pre_encoded();
                Ok(frame)
            },
            |run, mut frame| {
                frame.delay = run.delay.min(u64::from(u16::MAX)) as u16;
                encoder.write_lzw_pre_encoded_frame(&frame).map_err(failed)
            },
        )?;
    }
    Ok(bytes)
}

/// Premultiplied RGBA pixels over white, as opaque RGBA.
fn flattened_rgba(premultiplied: &[u8]) -> Vec<u8> {
    let mut rgba = Vec::with_capacity(premultiplied.len());
    for pixel in premultiplied.chunks_exact(4) {
        let uncovered = 255 - pixel[3];
        // Premultiplied channels never exceed alpha, so this stays <= 255.
        rgba.extend_from_slice(&[
            pixel[0].saturating_add(uncovered),
            pixel[1].saturating_add(uncovered),
            pixel[2].saturating_add(uncovered),
            255,
        ]);
    }
    rgba
}

#[cfg(test)]
mod tests {
    use super::super::animation::tests::Squares;
    use super::*;

    /// Decode a GIF: per frame its delay and RGBA pixels.
    fn decode(bytes: &[u8]) -> Vec<(u16, Vec<u8>)> {
        let mut options = gif::DecodeOptions::new();
        options.set_color_output(gif::ColorOutput::RGBA);
        let mut decoder = options.read_info(bytes).unwrap();
        let mut frames = Vec::new();
        while let Some(frame) = decoder.read_next_frame().unwrap() {
            frames.push((frame.delay, frame.buffer.to_vec()));
        }
        frames
    }

    #[test]
    fn frames_decode_to_the_drawn_pictures() {
        let source = Squares {
            timeline: vec![0, 1, 1, 2],
            fps: 4.0,
        };
        let frames = decode(&to_gif(&source, 1.0).unwrap());
        // The repeated frame is one GIF frame shown twice as long.
        let delays: Vec<u16> = frames.iter().map(|(delay, _)| *delay).collect();
        assert_eq!(delays, vec![25, 50, 25]);
        // Pixel (3, 4) is inside the square of frame 0 only; (7, 4) of frames 1 and 2.
        let at = |pixels: &[u8], x: usize, y: usize| {
            let start = (y * 20 + x) * 4;
            pixels[start..start + 3].to_vec()
        };
        let red = |pixel: Vec<u8>| pixel[0] > 150 && pixel[1] < 80 && pixel[2] < 80;
        let white = |pixel: Vec<u8>| pixel.iter().all(|channel| *channel > 240);
        assert!(red(at(&frames[0].1, 3, 4)));
        assert!(white(at(&frames[0].1, 7, 4)));
        assert!(red(at(&frames[1].1, 3, 4)));
        assert!(red(at(&frames[2].1, 7, 4)));
        assert!(white(at(&frames[2].1, 3, 4)));
    }

    #[test]
    fn transparent_pixels_become_white() {
        assert_eq!(flattened_rgba(&[0, 0, 0, 0]), vec![255, 255, 255, 255]);
        assert_eq!(flattened_rgba(&[100, 0, 0, 128]), vec![227, 127, 127, 255]);
    }
}

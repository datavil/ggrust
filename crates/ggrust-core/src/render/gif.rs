//! Animated GIF: every distinct frame rasterised (as PNG output) and flattened
//! onto white (GIF has no partial transparency). Frames that come out
//! pixel-identical to the previous one (pauses, a state held still) are merged
//! into one GIF frame with a longer delay, as gifski does. Every other frame
//! after the first writes only the rectangle that changed, with the pixels
//! that did not change left transparent so the frame below shows through:
//! small frames, long LZW runs. Each frame gets its own palette: exact when it
//! holds at most 255 colours, NeuQuant (`color_quant`) otherwise, with one
//! index kept for transparency. Drawing and encoding run on the rayon pool a
//! few frames at a time. Loops forever, as gganimate's gifski output.

use rayon::prelude::*;
use std::borrow::Cow;
use std::collections::HashMap;

use super::animation::{FrameSource, runs};
use super::png::{pixel_size, render_pixmap};
use crate::{Error, Result};

/// NeuQuant sampling: 1 is the slowest and best, 30 the fastest. 10 is the
/// `gif` crate's own suggestion for good quality at reasonable speed.
const QUANTIZE_SPEED: i32 = 10;

/// Colours a frame may use besides the transparent index.
const OPAQUE_COLORS: usize = 255;

/// The longest delay one GIF frame holds, in 1/100 s.
const MAX_DELAY: u64 = u16::MAX as u64;

/// A rectangle of the frame, in pixels.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
struct Area {
    left: usize,
    top: usize,
    width: usize,
    height: usize,
}

/// A distinct picture (opaque RGBA, the full frame) and how long it shows.
struct Shown {
    rgba: Vec<u8>,
    delay: u64,
}

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
    let draw = |frame: usize| -> Result<Vec<u8>> {
        let pixmap = render_pixmap(&source.scene(frame)?, scale)?;
        if (pixmap.width(), pixmap.height()) != (u32::from(width), u32::from(height)) {
            return Err(Error::render(
                "every frame of an animation must have the same size",
            ));
        }
        Ok(flattened_rgba(pixmap.data()))
    };
    let frame_width = usize::from(width);

    let mut bytes = Vec::new();
    {
        let mut encoder = gif::Encoder::new(&mut bytes, width, height, &[]).map_err(failed)?;
        encoder.set_repeat(gif::Repeat::Infinite).map_err(failed)?;
        let mut write = |frame: gif::Frame<'static>, delay: u64| -> Result<()> {
            write_frame(&mut encoder, frame, delay).map_err(failed)
        };
        // The last picture written, and the distinct pictures not written yet. The
        // newest one waits until the next differs, since that may lengthen its delay.
        let mut written: Option<Vec<u8>> = None;
        let mut waiting: Vec<Shown> = Vec::new();
        let chunk = rayon::current_num_threads().max(1);
        for runs in runs.chunks(chunk) {
            let pictures: Vec<Result<Vec<u8>>> =
                runs.par_iter().map(|run| draw(run.frame)).collect();
            for (run, picture) in runs.iter().zip(pictures) {
                let picture = picture?;
                match waiting.last_mut() {
                    Some(last) if last.rgba == picture => last.delay += run.delay,
                    _ => waiting.push(Shown {
                        rgba: picture,
                        delay: run.delay,
                    }),
                }
            }
            let ready = waiting.len().saturating_sub(1);
            let frames: Vec<gif::Frame<'static>> = (0..ready)
                .into_par_iter()
                .map(|index| {
                    let previous = match index {
                        0 => written.as_deref(),
                        _ => Some(waiting[index - 1].rgba.as_slice()),
                    };
                    encode_frame(&waiting[index].rgba, previous, frame_width)
                })
                .collect();
            for (frame, shown) in frames.into_iter().zip(&waiting) {
                write(frame, shown.delay)?;
            }
            if ready > 0 {
                written = waiting.drain(..ready).next_back().map(|shown| shown.rgba);
            }
        }
        if let Some(last) = waiting.pop() {
            write(
                encode_frame(&last.rgba, written.as_deref(), frame_width),
                last.delay,
            )?;
        }
    }
    Ok(bytes)
}

/// Write `frame` shown for `delay` (1/100 s). A delay longer than one GIF frame
/// holds goes on in fully transparent 1 x 1 frames, which change nothing.
fn write_frame<W: std::io::Write>(
    encoder: &mut gif::Encoder<W>,
    mut frame: gif::Frame<'static>,
    delay: u64,
) -> std::result::Result<(), gif::EncodingError> {
    let mut rest = delay.max(1);
    frame.delay = rest.min(MAX_DELAY) as u16;
    rest -= u64::from(frame.delay);
    encoder.write_lzw_pre_encoded_frame(&frame)?;
    while rest > 0 {
        let mut hold = gif::Frame {
            width: 1,
            height: 1,
            palette: Some(vec![0, 0, 0]),
            transparent: Some(0),
            buffer: Cow::Owned(vec![0]),
            dispose: gif::DisposalMethod::Keep,
            ..gif::Frame::default()
        };
        hold.delay = rest.min(MAX_DELAY) as u16;
        rest -= u64::from(hold.delay);
        hold.make_lzw_pre_encoded();
        encoder.write_lzw_pre_encoded_frame(&hold)?;
    }
    Ok(())
}

/// The GIF frame that turns `previous` (the picture on screen, or nothing for the
/// first frame) into `rgba`: the changed rectangle, unchanged pixels transparent.
fn encode_frame(rgba: &[u8], previous: Option<&[u8]>, frame_width: usize) -> gif::Frame<'static> {
    let full = Area {
        left: 0,
        top: 0,
        width: frame_width,
        height: rgba.len() / 4 / frame_width,
    };
    let area = previous.map_or(Some(full), |previous| {
        changed_area(previous, rgba, frame_width)
    });
    // Identical pictures are merged before this; keep one pixel if one gets here.
    let area = area.unwrap_or(Area {
        width: 1,
        height: 1,
        ..full
    });
    let mut pixels: Vec<Option<[u8; 3]>> = Vec::with_capacity(area.width * area.height);
    for row in area.top..area.top + area.height {
        for column in area.left..area.left + area.width {
            let start = (row * frame_width + column) * 4;
            let pixel = [rgba[start], rgba[start + 1], rgba[start + 2]];
            let unchanged = previous.is_some_and(|previous| previous[start..start + 3] == pixel);
            pixels.push((!unchanged).then_some(pixel));
        }
    }
    let (palette, buffer, transparent) = indexed(&pixels);
    let mut frame = gif::Frame {
        left: area.left as u16,
        top: area.top as u16,
        width: area.width as u16,
        height: area.height as u16,
        palette: Some(palette),
        transparent,
        buffer: Cow::Owned(buffer),
        dispose: gif::DisposalMethod::Keep,
        ..gif::Frame::default()
    };
    frame.make_lzw_pre_encoded();
    frame
}

/// The smallest rectangle holding every pixel that differs, or `None` when the
/// pictures are the same.
fn changed_area(previous: &[u8], rgba: &[u8], frame_width: usize) -> Option<Area> {
    let row_bytes = frame_width * 4;
    let rows = previous
        .chunks_exact(row_bytes)
        .zip(rgba.chunks_exact(row_bytes));
    let changed_rows: Vec<usize> = rows
        .enumerate()
        .filter(|(_, (before, after))| before != after)
        .map(|(row, _)| row)
        .collect();
    let (&top, &bottom) = (changed_rows.first()?, changed_rows.last()?);
    let (mut left, mut right) = (frame_width, 0);
    for &row in &changed_rows {
        let start = row * row_bytes;
        let before = &previous[start..start + row_bytes];
        let after = &rgba[start..start + row_bytes];
        let differs = |column: &usize| {
            before[column * 4..column * 4 + 4] != after[column * 4..column * 4 + 4]
        };
        if let Some(first) = (0..left).find(differs) {
            left = first;
        }
        if let Some(last) = (right..frame_width).rev().find(differs) {
            right = last;
        }
    }
    Some(Area {
        left,
        top,
        width: right.max(left) - left + 1,
        height: bottom - top + 1,
    })
}

/// Palette (RGB), indices and the transparent index for pixels where `None` is
/// transparent. Exact colours when at most 255 are used, NeuQuant otherwise.
fn indexed(pixels: &[Option<[u8; 3]>]) -> (Vec<u8>, Vec<u8>, Option<u8>) {
    let any_transparent = pixels.iter().any(Option::is_none);
    let mut exact: HashMap<[u8; 3], u8> = HashMap::new();
    let mut palette = Vec::new();
    let mut fits = true;
    for pixel in pixels.iter().flatten() {
        if !exact.contains_key(pixel) {
            if exact.len() == OPAQUE_COLORS {
                fits = false;
                break;
            }
            exact.insert(*pixel, exact.len() as u8);
            palette.extend_from_slice(pixel);
        }
    }
    let transparent_index = if fits { exact.len() } else { OPAQUE_COLORS };
    let transparent = any_transparent.then_some(transparent_index as u8);
    if fits {
        if any_transparent {
            palette.extend_from_slice(&[0, 0, 0]);
        }
        if palette.is_empty() {
            palette.extend_from_slice(&[0, 0, 0]);
        }
        let buffer = pixels
            .iter()
            .map(|pixel| pixel.map_or(transparent_index as u8, |pixel| exact[&pixel]))
            .collect();
        return (palette, buffer, transparent);
    }
    let opaque: Vec<u8> = pixels
        .iter()
        .flatten()
        .flat_map(|[red, green, blue]| [*red, *green, *blue, 255])
        .collect();
    let quantizer = color_quant::NeuQuant::new(QUANTIZE_SPEED, OPAQUE_COLORS, &opaque);
    let mut palette = quantizer.color_map_rgb();
    palette.extend_from_slice(&[0, 0, 0]);
    let buffer = pixels
        .iter()
        .map(|pixel| match pixel {
            Some([red, green, blue]) => quantizer.index_of(&[*red, *green, *blue, 255]) as u8,
            None => OPAQUE_COLORS as u8,
        })
        .collect();
    (palette, buffer, transparent)
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

    /// Decode a GIF: per frame its delay and the RGBA picture on screen after it
    /// (the decoder composites partial frames onto the previous picture).
    fn decode(bytes: &[u8]) -> Vec<(u16, Vec<u8>)> {
        let mut options = gif::DecodeOptions::new();
        options.set_color_output(gif::ColorOutput::RGBA);
        let mut decoder = options.read_info(bytes).unwrap();
        let (width, height) = (usize::from(decoder.width()), usize::from(decoder.height()));
        let mut screen = vec![0u8; width * height * 4];
        let mut frames = Vec::new();
        while let Some(frame) = decoder.read_next_frame().unwrap() {
            for row in 0..usize::from(frame.height) {
                for column in 0..usize::from(frame.width) {
                    let source = (row * usize::from(frame.width) + column) * 4;
                    if frame.buffer[source + 3] == 0 {
                        continue;
                    }
                    let target =
                        ((row + usize::from(frame.top)) * width + column + usize::from(frame.left))
                            * 4;
                    screen[target..target + 4].copy_from_slice(&frame.buffer[source..source + 4]);
                }
            }
            frames.push((frame.delay, screen.clone()));
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
    fn later_frames_hold_only_the_changed_rectangle() {
        let source = Squares {
            timeline: vec![0, 1, 2],
            fps: 10.0,
        };
        let bytes = to_gif(&source, 1.0).unwrap();
        let mut options = gif::DecodeOptions::new();
        options.set_color_output(gif::ColorOutput::RGBA);
        let mut decoder = options.read_info(bytes.as_slice()).unwrap();
        let mut sizes = Vec::new();
        while let Some(frame) = decoder.read_next_frame().unwrap() {
            sizes.push((frame.left, frame.top, frame.width, frame.height));
        }
        // The first frame is full; each later one covers only the pixels the
        // moving square left or reached, a strip a few pixels wide.
        assert_eq!(sizes[0], (0, 0, 20, 10));
        assert!(sizes[1].2 < 20 && sizes[1].3 < 10, "{sizes:?}");
        assert!(sizes[2].2 < 20 && sizes[2].3 < 10, "{sizes:?}");
    }

    /// Every frame of `timeline` is drawn by `picture`; the scene index is ignored.
    struct Pictures {
        timeline: Vec<usize>,
    }

    impl FrameSource for Pictures {
        fn scene(&self, frame: usize) -> Result<crate::scene::Scene> {
            // Frames 0 and 1 draw the same square, as do 2 and 3.
            Squares {
                timeline: vec![],
                fps: 1.0,
            }
            .scene(usize::from(frame >= 2))
        }

        fn timeline(&self) -> &[usize] {
            &self.timeline
        }

        fn fps(&self) -> f64 {
            10.0
        }
    }

    #[test]
    fn pixel_identical_frames_become_one_with_the_summed_delay() {
        let source = Pictures {
            timeline: vec![0, 1, 2, 3],
        };
        let frames = decode(&to_gif(&source, 1.0).unwrap());
        let delays: Vec<u16> = frames.iter().map(|(delay, _)| *delay).collect();
        assert_eq!(delays, vec![20, 20]);
    }

    #[test]
    fn long_delays_continue_in_transparent_frames() {
        let mut bytes = Vec::new();
        {
            let mut encoder = gif::Encoder::new(&mut bytes, 2, 1, &[]).unwrap();
            let rgba = [10, 20, 30, 255, 40, 50, 60, 255];
            write_frame(&mut encoder, encode_frame(&rgba, None, 2), MAX_DELAY + 5).unwrap();
        }
        let frames = decode(&bytes);
        let delays: Vec<u16> = frames.iter().map(|(delay, _)| *delay).collect();
        assert_eq!(delays, vec![u16::MAX, 5]);
        assert_eq!(frames[1].1, vec![10, 20, 30, 255, 40, 50, 60, 255]);
    }

    #[test]
    fn many_colours_are_quantised_and_unchanged_pixels_stay_transparent() {
        // 300 distinct colours force NeuQuant; the unchanged pixel is transparent.
        let previous: Vec<u8> = (0..300u32)
            .flat_map(|index| [(index % 256) as u8, (index / 2) as u8, 7, 255])
            .collect();
        let mut rgba = previous.clone();
        // Every pixel but the first and the middle one changes.
        for (index, pixel) in rgba.chunks_exact_mut(4).enumerate() {
            if index != 0 && index != 150 {
                pixel[2] = 200;
            }
        }
        let frame = encode_frame(&rgba, Some(&previous), 300);
        assert_eq!((frame.left, frame.width), (1, 299));
        assert_eq!(frame.transparent, Some(OPAQUE_COLORS as u8));
        // Only the unchanged pixel takes the transparent index.
        let pixels: Vec<Option<[u8; 3]>> = rgba
            .chunks_exact(4)
            .zip(previous.chunks_exact(4))
            .map(|(after, before)| (after != before).then(|| [after[0], after[1], after[2]]))
            .collect();
        let (_, buffer, _) = indexed(&pixels[1..]);
        let transparent: Vec<usize> = (0..buffer.len())
            .filter(|index| buffer[*index] as usize == OPAQUE_COLORS)
            .collect();
        assert_eq!(transparent, vec![149]);
        assert_eq!(
            frame.palette.as_ref().unwrap().len(),
            (OPAQUE_COLORS + 1) * 3
        );
    }

    #[test]
    fn changed_area_is_tight() {
        let previous = vec![255u8; 5 * 4 * 4];
        let mut rgba = previous.clone();
        let set = |rgba: &mut Vec<u8>, x: usize, y: usize| rgba[(y * 5 + x) * 4] = 0;
        set(&mut rgba, 1, 2);
        set(&mut rgba, 3, 1);
        assert_eq!(
            changed_area(&previous, &rgba, 5),
            Some(Area {
                left: 1,
                top: 1,
                width: 3,
                height: 2
            })
        );
        assert_eq!(changed_area(&previous, &previous, 5), None);
    }

    #[test]
    fn transparent_pixels_become_white() {
        assert_eq!(flattened_rgba(&[0, 0, 0, 0]), vec![255, 255, 255, 255]);
        assert_eq!(flattened_rgba(&[100, 0, 0, 128]), vec![227, 127, 127, 255]);
    }
}

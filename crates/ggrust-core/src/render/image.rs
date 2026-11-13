//! JPEG and TIFF output (`ggsave("plot.jpg")`, `ggsave("plot.tiff")`).
//!
//! Both rasterise the scene exactly as PNG does (`png::render_pixmap`) and only
//! change the encoding. JPEG has no alpha, so pixels are composited over white, as
//! R's `jpeg()` device does. TIFF is baseline RGB (RGBA with unassociated alpha
//! when any pixel is translucent), LZW compressed in strips, with the resolution
//! in `XResolution` / `YResolution`. The TIFF writer is hand-written: a baseline
//! file is a header and one directory, smaller than any TIFF crate.
//! This module does not decide sizes or resolutions (the Python `ggsave` does).

use jpeg_encoder::{ColorType, Density, Encoder};
use resvg::tiny_skia::Pixmap;

use crate::render::png::render_pixmap;
use crate::scene::Scene;
use crate::{Error, Result};

/// R's `jpeg()` default quality (ggplot2's `ggsave` passes it through).
pub const DEFAULT_JPEG_QUALITY: u8 = 75;

/// Uncompressed bytes per TIFF strip (libtiff's suggested 8 KiB is too small to
/// compress well; 64 KiB keeps strips cheap to decode).
const TIFF_STRIP_BYTES: usize = 64 * 1024;

/// Render a scene to JPEG bytes. `scale` multiplies the pixel size, `dpi` goes
/// into the JFIF header and `quality` is 1..=100.
pub fn to_jpeg(scene: &Scene, scale: f64, dpi: f64, quality: u8) -> Result<Vec<u8>> {
    check_dpi(dpi)?;
    if !(1..=100).contains(&quality) {
        return Err(Error::render(format!(
            "JPEG quality must be between 1 and 100, got {quality}"
        )));
    }
    let pixmap = render_pixmap(scene, scale)?;
    encode_jpeg(&pixmap, dpi, quality, scene.metadata)
}

/// Render a scene to LZW-compressed TIFF bytes. `scale` multiplies the pixel
/// size and `dpi` is written as the resolution.
pub fn to_tiff(scene: &Scene, scale: f64, dpi: f64) -> Result<Vec<u8>> {
    check_dpi(dpi)?;
    let pixmap = render_pixmap(scene, scale)?;
    encode_tiff(&pixmap, dpi, scene.metadata)
}

fn check_dpi(dpi: f64) -> Result<()> {
    if dpi.is_finite() && dpi > 0.0 && dpi <= f64::from(u16::MAX) {
        Ok(())
    } else {
        Err(Error::render(format!(
            "dpi must be positive and at most {}, got {dpi}",
            u16::MAX
        )))
    }
}

/// JPEG bytes of `pixmap`; `metadata` adds a `COM` segment holding
/// [`GENERATOR`](super::GENERATOR).
fn encode_jpeg(pixmap: &Pixmap, dpi: f64, quality: u8, metadata: bool) -> Result<Vec<u8>> {
    // Premultiplied pixels over white: channel + (255 - alpha).
    let rgb: Vec<u8> = pixmap
        .pixels()
        .iter()
        .flat_map(|pixel| {
            let under = 255 - pixel.alpha();
            [
                pixel.red() + under,
                pixel.green() + under,
                pixel.blue() + under,
            ]
        })
        .collect();
    let too_large = || {
        Error::render(format!(
            "a JPEG is at most 65535 pixels per side, got {} x {}",
            pixmap.width(),
            pixmap.height()
        ))
    };
    let width = u16::try_from(pixmap.width()).map_err(|_| too_large())?;
    let height = u16::try_from(pixmap.height()).map_err(|_| too_large())?;
    let mut bytes = Vec::new();
    let mut encoder = Encoder::new(&mut bytes, quality);
    // JFIF density 0 means "aspect ratio only"; a sub-1 dpi rounds up to 1.
    let density = dpi.round().max(1.0) as u16;
    encoder.set_density(Density::Inch {
        x: density,
        y: density,
    });
    encoder
        .encode(&rgb, width, height, ColorType::Rgb)
        .map_err(|error| Error::render(format!("JPEG encoding failed: {error}")))?;
    if metadata {
        insert_jpeg_comment(&mut bytes, super::GENERATOR)?;
    }
    Ok(bytes)
}

/// Insert a `COM` segment holding `comment` after the JFIF `APP0` segment (which
/// must directly follow the start-of-image marker).
fn insert_jpeg_comment(bytes: &mut Vec<u8>, comment: &str) -> Result<()> {
    let jfif = bytes.len() >= 6 && bytes[..4] == [0xFF, 0xD8, 0xFF, 0xE0];
    if !jfif {
        return Err(Error::render(
            "JPEG encoding failed: the encoder did not start with a JFIF header",
        ));
    }
    let after_app0 = 4 + usize::from(u16::from_be_bytes([bytes[4], bytes[5]]));
    let length = u16::try_from(comment.len() + 2)
        .map_err(|_| Error::render("a JPEG comment is at most 65533 bytes"))?;
    let mut segment = vec![0xFF, 0xFE];
    segment.extend_from_slice(&length.to_be_bytes());
    segment.extend_from_slice(comment.as_bytes());
    bytes.splice(after_app0..after_app0, segment);
    Ok(())
}

/// TIFF field types used here.
const ASCII: u16 = 2;
const SHORT: u16 = 3;
const LONG: u16 = 4;
const RATIONAL: u16 = 5;

/// TIFF bytes of `pixmap`; `metadata` adds the `Software` tag (305) holding
/// [`GENERATOR`](super::GENERATOR).
fn encode_tiff(pixmap: &Pixmap, dpi: f64, metadata: bool) -> Result<Vec<u8>> {
    let opaque = pixmap.pixels().iter().all(|pixel| pixel.alpha() == 255);
    let samples: usize = if opaque { 3 } else { 4 };
    let (width, height) = (pixmap.width() as usize, pixmap.height() as usize);
    let row_bytes = width * samples;
    let rows_per_strip = (TIFF_STRIP_BYTES / row_bytes).clamp(1, height);
    let pixels = pixmap.pixels();

    // Header, then the strips, then the directory and its out-of-line values.
    let mut bytes = b"II*\0\0\0\0\0".to_vec();
    let mut strip_offsets = Vec::new();
    let mut strip_counts = Vec::new();
    let mut strip = Vec::with_capacity(rows_per_strip * row_bytes);
    for rows in pixels.chunks(rows_per_strip * width) {
        strip.clear();
        for pixel in rows {
            let color = pixel.demultiply();
            strip.extend_from_slice(&[color.red(), color.green(), color.blue()]);
            if !opaque {
                strip.push(color.alpha());
            }
        }
        let compressed = weezl::encode::Encoder::with_tiff_size_switch(weezl::BitOrder::Msb, 8)
            .encode(&strip)
            .map_err(|error| Error::render(format!("TIFF encoding failed: {error}")))?;
        strip_offsets.push(offset(bytes.len())?);
        strip_counts.push(offset(compressed.len())?);
        bytes.extend_from_slice(&compressed);
        if bytes.len() % 2 == 1 {
            bytes.push(0); // TIFF values start on word boundaries.
        }
    }

    let resolution = [(dpi * 100.0).round() as u32, 100];
    let mut entries: Vec<(u16, u16, Values)> = vec![
        (256, LONG, Values::Long(vec![offset(width)?])),
        (257, LONG, Values::Long(vec![offset(height)?])),
        (258, SHORT, Values::Short(vec![8; samples])),
        (259, SHORT, Values::Short(vec![5])), // LZW
        (262, SHORT, Values::Short(vec![2])), // RGB
        (273, LONG, Values::Long(strip_offsets)),
        (277, SHORT, Values::Short(vec![samples as u16])),
        (278, LONG, Values::Long(vec![offset(rows_per_strip)?])),
        (279, LONG, Values::Long(strip_counts)),
        (282, RATIONAL, Values::Long(resolution.to_vec())),
        (283, RATIONAL, Values::Long(resolution.to_vec())),
        (284, SHORT, Values::Short(vec![1])), // chunky
        (296, SHORT, Values::Short(vec![2])), // inch
    ];
    if metadata {
        // NUL-terminated ASCII; tags stay in ascending order (296 < 305 < 338).
        let mut software = super::GENERATOR.as_bytes().to_vec();
        software.push(0);
        entries.push((305, ASCII, Values::Ascii(software)));
    }
    if !opaque {
        entries.push((338, SHORT, Values::Short(vec![2]))); // unassociated alpha
    }

    let directory = bytes.len();
    bytes[4..8].copy_from_slice(&offset(directory)?.to_le_bytes());
    let mut overflow = directory + 2 + entries.len() * 12 + 4;
    let mut out_of_line = Vec::new();
    bytes.extend_from_slice(&(entries.len() as u16).to_le_bytes());
    for (tag, kind, values) in &entries {
        let (count, encoded) = values.encode();
        let count = if *kind == RATIONAL { count / 2 } else { count };
        bytes.extend_from_slice(&tag.to_le_bytes());
        bytes.extend_from_slice(&kind.to_le_bytes());
        bytes.extend_from_slice(&offset(count)?.to_le_bytes());
        if encoded.len() <= 4 {
            let mut inline = [0u8; 4];
            inline[..encoded.len()].copy_from_slice(&encoded);
            bytes.extend_from_slice(&inline);
        } else {
            bytes.extend_from_slice(&offset(overflow)?.to_le_bytes());
            overflow += encoded.len();
            out_of_line.extend_from_slice(&encoded);
        }
    }
    bytes.extend_from_slice(&0u32.to_le_bytes()); // no next directory
    bytes.extend_from_slice(&out_of_line);
    Ok(bytes)
}

/// A TIFF value array, little-endian.
enum Values {
    Ascii(Vec<u8>),
    Short(Vec<u16>),
    Long(Vec<u32>),
}

impl Values {
    fn encode(&self) -> (usize, Vec<u8>) {
        match self {
            // Out-of-line values start on word boundaries: pad odd text with a NUL.
            Values::Ascii(text) => {
                let mut encoded = text.clone();
                if encoded.len() % 2 == 1 {
                    encoded.push(0);
                }
                (text.len(), encoded)
            }
            Values::Short(values) => (
                values.len(),
                values
                    .iter()
                    .flat_map(|value| value.to_le_bytes())
                    .collect(),
            ),
            Values::Long(values) => (
                values.len(),
                values
                    .iter()
                    .flat_map(|value| value.to_le_bytes())
                    .collect(),
            ),
        }
    }
}

/// A file offset or count as a TIFF LONG; files past 4 GiB are refused.
fn offset(value: usize) -> Result<u32> {
    u32::try_from(value).map_err(|_| {
        Error::render("the TIFF would exceed 4 GiB; lower 'dpi' or the plot size".to_owned())
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scene::Color;

    fn sized(width: f64, height: f64, background: Option<Color>) -> Scene {
        Scene {
            width,
            height,
            metadata: false,
            background,
            nodes: Vec::new(),
        }
    }

    fn scene(background: Option<Color>) -> Scene {
        sized(30.0, 20.0, background)
    }

    fn u16_at(bytes: &[u8], at: usize) -> u16 {
        u16::from_le_bytes([bytes[at], bytes[at + 1]])
    }

    fn u32_at(bytes: &[u8], at: usize) -> u32 {
        u32::from_le_bytes([bytes[at], bytes[at + 1], bytes[at + 2], bytes[at + 3]])
    }

    /// Tag -> (type, count, value or offset) from the first directory.
    fn directory(bytes: &[u8]) -> Vec<(u16, u16, u32, u32)> {
        let start = u32_at(bytes, 4) as usize;
        let count = u16_at(bytes, start) as usize;
        (0..count)
            .map(|index| {
                let at = start + 2 + index * 12;
                (
                    u16_at(bytes, at),
                    u16_at(bytes, at + 2),
                    u32_at(bytes, at + 4),
                    u32_at(bytes, at + 8),
                )
            })
            .collect()
    }

    fn decode_tiff(bytes: &[u8]) -> (u32, u32, usize, Vec<u8>) {
        let entries = directory(bytes);
        let value = |tag: u16| entries.iter().find(|entry| entry.0 == tag).map(|e| e.3);
        let width = value(256).unwrap_or_default();
        let height = value(257).unwrap_or_default();
        let samples = value(277).unwrap_or_default() as usize;
        let strips = entries
            .iter()
            .find(|entry| entry.0 == 273)
            .map(|e| (e.2, e.3));
        let counts = entries
            .iter()
            .find(|entry| entry.0 == 279)
            .map(|e| (e.2, e.3));
        let (Some((strip_count, offsets)), Some((_, sizes))) = (strips, counts) else {
            panic!("strip tags missing");
        };
        let lookup = |field: u32, index: u32| {
            if strip_count == 1 {
                field
            } else {
                u32_at(bytes, (field + 4 * index) as usize)
            }
        };
        let mut pixels = Vec::new();
        for index in 0..strip_count {
            let start = lookup(offsets, index) as usize;
            let end = start + lookup(sizes, index) as usize;
            let mut decoder =
                weezl::decode::Decoder::with_tiff_size_switch(weezl::BitOrder::Msb, 8);
            pixels.extend(decoder.decode(&bytes[start..end]).expect("valid LZW"));
        }
        (width, height, samples, pixels)
    }

    #[test]
    fn tiff_is_rgb_lzw_with_resolution() {
        let red = Color::rgb(200, 10, 30);
        let bytes = to_tiff(&scene(Some(red)), 2.0, 300.0).expect("tiff");
        assert_eq!(&bytes[..4], b"II*\0");
        let entries = directory(&bytes);
        let tags: Vec<u16> = entries.iter().map(|entry| entry.0).collect();
        let mut sorted = tags.clone();
        sorted.sort_unstable();
        assert_eq!(tags, sorted, "directory entries must be sorted by tag");
        let compression = entries.iter().find(|entry| entry.0 == 259).map(|e| e.3);
        assert_eq!(compression, Some(5));
        let x_resolution = entries.iter().find(|entry| entry.0 == 282).map(|e| e.3);
        let at = x_resolution.expect("XResolution") as usize;
        assert_eq!((u32_at(&bytes, at), u32_at(&bytes, at + 4)), (30_000, 100));
        let (width, height, samples, pixels) = decode_tiff(&bytes);
        assert_eq!((width, height, samples), (60, 40, 3));
        assert_eq!(pixels.len(), 60 * 40 * 3);
        assert!(pixels.chunks(3).all(|pixel| pixel == [200, 10, 30]));
    }

    #[test]
    fn tiff_keeps_alpha_when_translucent() {
        let bytes = to_tiff(&scene(None), 1.0, 96.0).expect("tiff");
        let (_, _, samples, pixels) = decode_tiff(&bytes);
        assert_eq!(samples, 4);
        assert!(pixels.chunks(4).all(|pixel| pixel[3] == 0));
        let extra = directory(&bytes).into_iter().find(|entry| entry.0 == 338);
        assert_eq!(extra.map(|entry| entry.3), Some(2));
    }

    #[test]
    fn tiff_splits_tall_images_into_strips() {
        // 2000 px wide RGB rows: 10 rows per 64 KiB strip, 50 rows -> 5 strips.
        let tall = sized(2000.0, 50.0, Some(Color::rgb(0, 0, 255)));
        let bytes = to_tiff(&tall, 1.0, 96.0).expect("tiff");
        let strips = directory(&bytes).into_iter().find(|entry| entry.0 == 273);
        assert_eq!(strips.map(|entry| entry.2), Some(5));
        let (_, _, _, pixels) = decode_tiff(&bytes);
        assert_eq!(pixels.len(), 2000 * 50 * 3);
        assert!(pixels.chunks(3).all(|pixel| pixel == [0, 0, 255]));
    }

    #[test]
    fn jpeg_composites_over_white_and_writes_density() {
        let bytes = to_jpeg(&scene(None), 1.0, 300.0, 90).expect("jpeg");
        assert_eq!(&bytes[..2], &[0xFF, 0xD8]);
        // JFIF APP0: units 1 (inch), then x and y density, big-endian.
        let app0 = bytes
            .windows(5)
            .position(|window| window == b"JFIF\0")
            .expect("JFIF header");
        assert_eq!(bytes[app0 + 7], 1);
        assert_eq!(u16::from_be_bytes([bytes[app0 + 8], bytes[app0 + 9]]), 300);
        let tiny = to_jpeg(&scene(None), 1.0, 0.4, 90).expect("jpeg");
        assert_eq!(u16::from_be_bytes([tiny[app0 + 8], tiny[app0 + 9]]), 1);
        assert!(to_jpeg(&scene(None), 1.0, 96.0, 0).is_err());
        assert!(to_jpeg(&scene(None), 1.0, 96.0, 101).is_err());
        assert!(to_tiff(&scene(None), 1.0, 0.0).is_err());
    }
    fn tagged(background: Option<Color>) -> Scene {
        Scene {
            metadata: true,
            ..scene(background)
        }
    }

    #[test]
    fn tiff_software_tag_holds_the_generator() {
        for background in [Some(Color::WHITE), None] {
            let bytes = to_tiff(&tagged(background), 1.0, 96.0).expect("tiff");
            let entries = directory(&bytes);
            let tags: Vec<u16> = entries.iter().map(|entry| entry.0).collect();
            assert!(tags.windows(2).all(|pair| pair[0] < pair[1]), "{tags:?}");
            let &(_, kind, count, at) = entries
                .iter()
                .find(|entry| entry.0 == 305)
                .expect("Software tag");
            assert_eq!(kind, ASCII);
            let text = &bytes[at as usize..at as usize + count as usize];
            assert_eq!(text, format!("{}\0", crate::render::GENERATOR).as_bytes());
            // Same pixels as the untagged file.
            let plain = to_tiff(&scene(background), 1.0, 96.0).expect("tiff");
            assert_eq!(decode_tiff(&bytes), decode_tiff(&plain));
            assert!(directory(&plain).iter().all(|entry| entry.0 != 305));
        }
    }

    #[test]
    fn tiff_odd_length_text_keeps_later_values_word_aligned() {
        let (count, encoded) = Values::Ascii(b"abc\0".to_vec()).encode();
        assert_eq!((count, encoded.len()), (4, 4));
        let (count, encoded) = Values::Ascii(b"ab\0".to_vec()).encode();
        assert_eq!((count, encoded), (3, b"ab\0\0".to_vec()));
    }

    /// (marker, payload) of each JPEG segment before the scan.
    fn jpeg_segments(bytes: &[u8]) -> Vec<(u8, Vec<u8>)> {
        let mut segments = Vec::new();
        let mut at = 2;
        while bytes[at] == 0xFF && bytes[at + 1] != 0xDA {
            let length = usize::from(u16::from_be_bytes([bytes[at + 2], bytes[at + 3]]));
            segments.push((bytes[at + 1], bytes[at + 4..at + 2 + length].to_vec()));
            at += 2 + length;
        }
        segments
    }

    #[test]
    fn jpeg_comment_follows_the_jfif_header() {
        let bytes = to_jpeg(&tagged(None), 1.0, 96.0, 90).expect("jpeg");
        let segments = jpeg_segments(&bytes);
        assert_eq!(segments[0].0, 0xE0);
        assert!(segments[0].1.starts_with(b"JFIF\0"));
        assert_eq!(
            segments[1],
            (0xFE, crate::render::GENERATOR.as_bytes().to_vec())
        );
        let plain = to_jpeg(&scene(None), 1.0, 96.0, 90).expect("jpeg");
        assert!(
            jpeg_segments(&plain)
                .iter()
                .all(|segment| segment.0 != 0xFE)
        );
        // Only the comment segment differs.
        let comment = 4 + crate::render::GENERATOR.len();
        let after_app0 = 4 + usize::from(u16::from_be_bytes([plain[4], plain[5]]));
        assert_eq!(bytes[..after_app0], plain[..after_app0]);
        assert_eq!(bytes[after_app0 + comment..], plain[after_app0..]);
    }
}

//! PDF output: the PNG rasterisation on one page, as lets-plot's `ggsave(.pdf)`
//! (which converts its PNG with Pillow).
//!
//! The page is the image at `dpi` (pixels / dpi inches), drawn over white (PDF
//! images here have no alpha). The pixels travel as the zlib stream of an RGB PNG,
//! which PDF's `FlateDecode` reads with the PNG predictor (`/Predictor 15`), so no
//! extra compression code is needed.
//!
//! What it does not do: vector output, embedded fonts, metadata.

use std::fmt::Write as _;

use resvg::tiny_skia::Pixmap;

use super::png::render_pixmap;
use crate::scene::Scene;
use crate::{Error, Result};

/// PDF user space units (points) per inch.
const POINTS_PER_INCH: f64 = 72.0;

/// Render a scene to a one-page PDF: the PNG pixels (`scale`) at `dpi`.
pub fn to_pdf(scene: &Scene, scale: f64, dpi: f64) -> Result<Vec<u8>> {
    if !(dpi.is_finite() && dpi > 0.0) {
        return Err(Error::Render(format!("dpi must be positive, got {dpi}")));
    }
    let pixmap = render_pixmap(scene, scale)?;
    let (width, height) = (pixmap.width(), pixmap.height());
    let pixels = rgb_zlib_stream(&pixmap)?;
    let page_width = f64::from(width) * POINTS_PER_INCH / dpi;
    let page_height = f64::from(height) * POINTS_PER_INCH / dpi;
    let content = format!("q {page_width:.4} 0 0 {page_height:.4} 0 0 cm /Im0 Do Q");

    let mut document: Vec<u8> = b"%PDF-1.4\n%\xe2\xe3\xcf\xd3\n".to_vec();
    let mut offsets = Vec::new();
    let mut object = |document: &mut Vec<u8>, head: &str, stream: Option<&[u8]>| {
        offsets.push(document.len());
        let number = offsets.len();
        document.extend_from_slice(format!("{number} 0 obj\n{head}").as_bytes());
        if let Some(stream) = stream {
            document.extend_from_slice(b"\nstream\n");
            document.extend_from_slice(stream);
            document.extend_from_slice(b"\nendstream");
        }
        document.extend_from_slice(b"\nendobj\n");
    };
    object(&mut document, "<< /Type /Catalog /Pages 2 0 R >>", None);
    object(
        &mut document,
        "<< /Type /Pages /Kids [3 0 R] /Count 1 >>",
        None,
    );
    object(
        &mut document,
        &format!(
            "<< /Type /Page /Parent 2 0 R /MediaBox [0 0 {page_width:.4} {page_height:.4}] \
             /Resources << /XObject << /Im0 4 0 R >> >> /Contents 5 0 R >>"
        ),
        None,
    );
    object(
        &mut document,
        &format!(
            "<< /Type /XObject /Subtype /Image /Width {width} /Height {height} \
             /ColorSpace /DeviceRGB /BitsPerComponent 8 /Filter /FlateDecode \
             /DecodeParms << /Predictor 15 /Colors 3 /BitsPerComponent 8 /Columns {width} >> \
             /Length {} >>",
            pixels.len()
        ),
        Some(&pixels),
    );
    object(
        &mut document,
        &format!("<< /Length {} >>", content.len()),
        Some(content.as_bytes()),
    );

    let xref = document.len();
    let mut trailer = format!("xref\n0 {}\n0000000000 65535 f \n", offsets.len() + 1);
    for offset in &offsets {
        let _ = writeln!(trailer, "{offset:010} 00000 n ");
    }
    let _ = write!(
        trailer,
        "trailer\n<< /Size {} /Root 1 0 R >>\nstartxref\n{xref}\n%%EOF\n",
        offsets.len() + 1
    );
    document.extend_from_slice(trailer.as_bytes());
    Ok(document)
}

/// The pixels over white as an RGB PNG, and that PNG's zlib stream (its `IDAT`
/// chunks joined): PNG-filtered scanlines, as `/Predictor 15` expects.
fn rgb_zlib_stream(pixmap: &Pixmap) -> Result<Vec<u8>> {
    let failed = |error: png::EncodingError| Error::Render(format!("PDF encoding failed: {error}"));
    // Premultiplied channels over white: channel + (255 - alpha).
    let data: Vec<u8> = pixmap
        .pixels()
        .iter()
        .flat_map(|pixel| {
            let cover = 255 - pixel.alpha();
            [
                pixel.red().saturating_add(cover),
                pixel.green().saturating_add(cover),
                pixel.blue().saturating_add(cover),
            ]
        })
        .collect();
    let mut png_bytes = Vec::new();
    let mut encoder = png::Encoder::new(&mut png_bytes, pixmap.width(), pixmap.height());
    encoder.set_color(png::ColorType::Rgb);
    encoder.set_depth(png::BitDepth::Eight);
    let mut writer = encoder.write_header().map_err(failed)?;
    writer.write_image_data(&data).map_err(failed)?;
    writer.finish().map_err(failed)?;

    // Chunks after the 8-byte signature: length, type, data, CRC.
    let mut stream = Vec::new();
    let mut position = 8;
    while position + 8 <= png_bytes.len() {
        let length = u32::from_be_bytes([
            png_bytes[position],
            png_bytes[position + 1],
            png_bytes[position + 2],
            png_bytes[position + 3],
        ]) as usize;
        let kind = &png_bytes[position + 4..position + 8];
        let start = position + 8;
        let end = start + length;
        if end > png_bytes.len() {
            break;
        }
        if kind == b"IDAT" {
            stream.extend_from_slice(&png_bytes[start..end]);
        }
        position = end + 4;
    }
    Ok(stream)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::scene::{Color, Node, PerItem, Rect, Rects};

    fn scene() -> Scene {
        Scene {
            width: 40.0,
            height: 30.0,
            background: Some(Color::WHITE),
            nodes: vec![Node::Rects(Rects {
                rects: vec![Rect {
                    x: 0.0,
                    y: 0.0,
                    width: 20.0,
                    height: 30.0,
                }],
                fill: PerItem::Uniform(Some(Color::rgb(255, 0, 0))),
                stroke: None,
            })],
        }
    }

    #[test]
    fn page_is_the_image_at_its_dpi() {
        let pdf = to_pdf(&scene(), 2.0, 96.0).unwrap();
        let text = String::from_utf8_lossy(&pdf);
        assert!(text.starts_with("%PDF-1.4"));
        assert!(text.ends_with("%%EOF\n"));
        // 80 x 60 pixels at 96 dpi: 60 x 45 points.
        assert!(text.contains("/MediaBox [0 0 60.0000 45.0000]"), "{text}");
        assert!(text.contains("/Width 80 /Height 60"));
        // The xref offsets point at the objects (offsets are in bytes).
        let find = |needle: &[u8]| {
            pdf.windows(needle.len())
                .rposition(|window| window == needle)
                .unwrap()
        };
        let tail = String::from_utf8(pdf[find(b"startxref\n")..].to_vec()).unwrap();
        let start: usize = tail.lines().nth(1).unwrap().parse().unwrap();
        let table = String::from_utf8(pdf[start..].to_vec()).unwrap();
        assert!(table.starts_with("xref\n0 6\n"), "{table}");
        for (index, line) in table.lines().skip(3).take(5).enumerate() {
            let offset: usize = line[..10].parse().unwrap();
            assert!(pdf[offset..].starts_with(format!("{} 0 obj", index + 1).as_bytes()));
        }
    }

    #[test]
    fn rgb_stream_inflates_to_filtered_scanlines() {
        let pixmap = render_pixmap(&scene(), 1.0).unwrap();
        let stream = rgb_zlib_stream(&pixmap).unwrap();
        // zlib header (deflate, 32K window).
        assert_eq!(stream[0] & 0x0f, 8);
        assert_eq!((u16::from(stream[0]) << 8 | u16::from(stream[1])) % 31, 0);
        assert!(to_pdf(&scene(), 1.0, 0.0).is_err());
    }
}

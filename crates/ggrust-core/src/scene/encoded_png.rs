//! A PNG kept encoded (base64, as in a `data:image/png;base64,` URL) so the SVG
//! writer can embed it as given; only raster backends decode it.
//!
//! The base64 and the PNG chunk structure (every CRC, `IHDR` first, `IEND` last)
//! are checked up front, so a broken image fails the same way for every output;
//! only the pixel data is left compressed.
//!
//! What it does not do: other image formats.

use std::sync::Arc;

use super::Color;

/// The 8-byte PNG signature.
const PNG_SIGNATURE: &[u8] = b"\x89PNG\r\n\x1a\n";

/// A base64 PNG with its size, read from the `IHDR` header.
#[derive(Debug, Clone, PartialEq)]
pub struct EncodedPng {
    base64: Arc<str>,
    bytes: Arc<[u8]>,
    columns: u32,
    rows: u32,
}

impl EncodedPng {
    /// Check `base64` (standard alphabet, optional padding) and the PNG's chunks,
    /// and read the image size. Errors are messages for the caller to place.
    pub fn new(base64: Arc<str>) -> Result<Self, String> {
        let bytes = decode_base64(base64.trim_end_matches('='))?;
        let (columns, rows) = check_chunks(&bytes)?;
        Ok(EncodedPng {
            base64,
            bytes: bytes.into(),
            columns,
            rows,
        })
    }

    /// The base64 text, as given.
    pub fn base64(&self) -> &str {
        &self.base64
    }

    /// Pixels per row.
    pub fn columns(&self) -> u32 {
        self.columns
    }

    /// Number of rows.
    pub fn rows(&self) -> u32 {
        self.rows
    }

    /// Row-major straight-alpha colors, top row first.
    pub fn decode(&self) -> Result<Vec<Color>, String> {
        let mut decoder = png::Decoder::new(std::io::Cursor::new(&self.bytes[..]));
        decoder.set_transformations(png::Transformations::normalize_to_color8());
        let mut reader = decoder
            .read_info()
            .map_err(|error| format!("invalid PNG image: {error}"))?;
        let size = reader
            .output_buffer_size()
            .ok_or_else(|| "the PNG image is too large".to_owned())?;
        let mut buffer = vec![0; size];
        let info = reader
            .next_frame(&mut buffer)
            .map_err(|error| format!("invalid PNG image: {error}"))?;
        let channels = info.color_type.samples();
        let pixels: Vec<Color> = buffer[..info.buffer_size()]
            .chunks_exact(channels)
            .map(|sample| match (info.color_type, sample) {
                (png::ColorType::Rgba, &[red, green, blue, alpha]) => Color {
                    red,
                    green,
                    blue,
                    alpha,
                },
                (png::ColorType::Rgb, &[red, green, blue]) => Color::rgb(red, green, blue),
                (png::ColorType::GrayscaleAlpha, &[grey, alpha]) => Color {
                    red: grey,
                    green: grey,
                    blue: grey,
                    alpha,
                },
                (_, &[grey, ..]) => Color::rgb(grey, grey, grey),
                _ => Color::TRANSPARENT,
            })
            .collect();
        if (info.width, info.height) != (self.columns, self.rows)
            || pixels.len() as u64 != u64::from(self.columns) * u64::from(self.rows)
        {
            return Err(format!(
                "the PNG image has {} pixels, expected {} x {}",
                pixels.len(),
                self.columns,
                self.rows
            ));
        }
        Ok(pixels)
    }
}

/// The PNG's size, after checking its signature and chunk structure: `IHDR`
/// first, every chunk's CRC, `IEND` last.
fn check_chunks(bytes: &[u8]) -> Result<(u32, u32), String> {
    let invalid = |reason: &str| format!("invalid PNG image: {reason}");
    if !bytes.starts_with(PNG_SIGNATURE) {
        return Err(invalid("missing PNG signature"));
    }
    let number =
        |at: usize| u32::from_be_bytes([bytes[at], bytes[at + 1], bytes[at + 2], bytes[at + 3]]);
    let mut position = PNG_SIGNATURE.len();
    let mut size = None;
    loop {
        if position + 12 > bytes.len() {
            return Err(invalid("the data ends before the IEND chunk"));
        }
        let length = number(position) as usize;
        let kind = &bytes[position + 4..position + 8];
        let end = position + 8 + length;
        if end + 4 > bytes.len() {
            return Err(invalid("a chunk runs past the end of the data"));
        }
        if crc32fast::hash(&bytes[position + 4..end]) != number(end) {
            let name = String::from_utf8_lossy(kind);
            return Err(invalid(&format!("CRC mismatch in the {name} chunk")));
        }
        match (kind, size) {
            (b"IHDR", None) if length == 13 => {
                size = Some((number(position + 8), number(position + 12)))
            }
            (_, None) => return Err(invalid("the first chunk is not IHDR")),
            (b"IEND", Some(size)) => {
                return match size {
                    (0, _) | (_, 0) => Err(invalid(&format!("size {} x {}", size.0, size.1))),
                    size => Ok(size),
                };
            }
            _ => {}
        }
        position = end + 4;
    }
}

/// The 6-bit values of the base64 alphabet; 255 for other bytes.
const BASE64_VALUES: [u8; 256] = {
    let alphabet = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
    let mut table = [255u8; 256];
    let mut index = 0;
    while index < alphabet.len() {
        table[alphabet[index] as usize] = index as u8;
        index += 1;
    }
    table
};

/// Standard base64 without padding.
fn decode_base64(text: &str) -> Result<Vec<u8>, String> {
    let bytes = text.as_bytes();
    if bytes.len() % 4 == 1 {
        return Err(format!(
            "invalid base64 data (length {} is not a whole number of bytes)",
            bytes.len()
        ));
    }
    let value = |position: usize| match BASE64_VALUES[bytes[position] as usize] {
        255 => Err(format!(
            "invalid base64 data (unexpected character at position {position})"
        )),
        value => Ok(u32::from(value)),
    };
    let mut output = Vec::with_capacity(bytes.len() / 4 * 3 + 2);
    let whole = bytes.len() / 4 * 4;
    for start in (0..whole).step_by(4) {
        let group = value(start)? << 18
            | value(start + 1)? << 12
            | value(start + 2)? << 6
            | value(start + 3)?;
        output.extend_from_slice(&group.to_be_bytes()[1..]);
    }
    let rest = bytes.len() - whole;
    if rest > 0 {
        let mut group = 0;
        for offset in 0..rest {
            group |= value(whole + offset)? << (18 - 6 * offset);
        }
        output.extend_from_slice(&group.to_be_bytes()[1..rest]);
    }
    Ok(output)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn base64_round_trips() {
        for (text, bytes) in [
            ("", &b""[..]),
            ("TQ", b"M"),
            ("TWE", b"Ma"),
            ("TWFu", b"Man"),
            ("/+8A", &[0xff, 0xef, 0x00]),
        ] {
            assert_eq!(decode_base64(text).as_deref(), Ok(bytes));
        }
        assert!(decode_base64("ab$c").is_err());
        assert!(decode_base64("TWFuT").unwrap_err().contains("length 5"));
    }

    #[test]
    fn header_gives_the_size_and_bad_data_is_reported() {
        // A 3 x 2 RGB PNG.
        let mut bytes = Vec::new();
        let mut encoder = png::Encoder::new(&mut bytes, 3, 2);
        encoder.set_color(png::ColorType::Rgb);
        let mut writer = encoder.write_header().unwrap();
        writer.write_image_data(&[200; 18]).unwrap();
        writer.finish().unwrap();
        let text = base64_text(&bytes);
        let image = EncodedPng::new(text.as_str().into()).unwrap();
        assert_eq!((image.columns(), image.rows()), (3, 2));
        assert_eq!(image.decode().unwrap(), vec![Color::rgb(200, 200, 200); 6]);

        assert!(
            EncodedPng::new("ab$c".into())
                .unwrap_err()
                .contains("position 2")
        );
        assert!(
            EncodedPng::new("TWFu".into())
                .unwrap_err()
                .contains("signature")
        );
        // Truncated data, and a header edited without fixing its CRC, fail up front.
        let error = EncodedPng::new(text[..text.len() - 20].into()).unwrap_err();
        assert!(
            error.contains("past the end") || error.contains("IEND"),
            "{error}"
        );
        let mut edited = bytes.clone();
        edited[19] = 5; // width 3 -> 5
        let error = EncodedPng::new(base64_text(&edited).into()).unwrap_err();
        assert!(error.contains("CRC mismatch in the IHDR chunk"), "{error}");
    }

    fn base64_text(bytes: &[u8]) -> String {
        const ALPHABET: &[u8] = b"ABCDEFGHIJKLMNOPQRSTUVWXYZabcdefghijklmnopqrstuvwxyz0123456789+/";
        let mut text = String::new();
        for chunk in bytes.chunks(3) {
            let value = chunk
                .iter()
                .enumerate()
                .fold(0u32, |value, (index, &byte)| {
                    value | u32::from(byte) << (16 - 8 * index)
                });
            for index in 0..=chunk.len() {
                text.push(ALPHABET[(value >> (18 - 6 * index) & 63) as usize] as char);
            }
        }
        text
    }
}

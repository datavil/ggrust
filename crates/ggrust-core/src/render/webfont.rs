//! Web fonts for SVG and HTML: a subset of a registered face as a CSS `@font-face`.
//!
//! What it does: subsets the face to the characters a document draws (plus the
//! characters zoomed axis labels can add), then makes the PDF-oriented subset
//! from `subsetter` usable in browsers: it adds a `cmap` (format 4, and 12 for
//! characters beyond the BMP), the original `OS/2` table, and a `kern` table with
//! the pair adjustments the shaper applied to the document's text, so browsers
//! space the text as it was measured. Each rule carries a `unicode-range`, so
//! documents on one page that subset the same face compose into one font.
//!
//! What it does not do: keep `GSUB`/`GPOS`. Ligatures and other substitutions are
//! not available to the browser (a known gap; PNG and PDF are exact).

use std::collections::{BTreeMap, BTreeSet};

use harfrust::UnicodeBuffer;
use read_fonts::TableProvider;
use read_fonts::types::{GlyphId, Tag};
use subsetter::GlyphRemapper;

use super::svg::push_base64;
use crate::text::LoadedFont;

/// Characters subsets of zoomable plots include: what the HTML runtime writes
/// when zoom relabels axes (digits, signs, exponents, SI and date letters).
const ZOOM_CHARACTERS: &str = " !\"#$%&'()*+,-./0123456789:;<=>?@ABCDEFGHIJKLMNOPQRSTUVWXYZ\
     abcdefghijklmnopqrstuvwxyz\u{2212}\u{00D7}\u{00B7}\u{00B5}";

/// The `@font-face` rule for `face` covering `texts` (and [`ZOOM_CHARACTERS`]
/// when `zoomable`), or `None` when the face cannot be subset (the document
/// then falls back to the family name).
pub(super) fn font_face_rule(
    face: &LoadedFont,
    texts: &BTreeSet<String>,
    zoomable: bool,
) -> Option<String> {
    let font = web_font(face, texts, zoomable)?;
    let mut rule = String::with_capacity(font.bytes.len() * 4 / 3 + 256);
    rule.push_str("@font-face{font-family:'");
    rule.push_str(&face.css_name);
    rule.push_str("';src:url(data:font/");
    rule.push_str(if face.cff { "otf" } else { "ttf" });
    rule.push_str(";base64,");
    push_base64(&mut rule, &font.bytes);
    rule.push_str(") format('");
    rule.push_str(if face.cff { "opentype" } else { "truetype" });
    rule.push_str("');unicode-range:");
    rule.push_str(&unicode_range(&font.characters));
    rule.push('}');
    Some(rule)
}

struct WebFont {
    bytes: Vec<u8>,
    /// Characters the `cmap` maps, sorted.
    characters: Vec<char>,
}

fn web_font(face: &LoadedFont, texts: &BTreeSet<String>, zoomable: bool) -> Option<WebFont> {
    let cmap = face.font_ref.cmap().ok()?;
    let hmtx = face.font_ref.hmtx().ok()?;
    let mut remapper = GlyphRemapper::new();
    let mut nominal: BTreeMap<char, u16> = BTreeMap::new();
    let characters = texts
        .iter()
        .flat_map(|text| text.chars())
        .chain(ZOOM_CHARACTERS.chars().filter(|_| zoomable));
    for character in characters {
        if nominal.contains_key(&character) {
            continue;
        }
        let glyph = cmap
            .map_codepoint(character)
            .and_then(|glyph| u16::try_from(glyph.to_u32()).ok())
            .filter(|glyph| *glyph != 0);
        if let Some(glyph) = glyph {
            nominal.insert(character, remapper.remap(glyph));
        }
    }
    // Format 4 lengths are 16 bits: one 8-byte segment per character.
    if nominal.is_empty() || nominal.len() > 8000 {
        return None;
    }
    // Pair adjustments the shaper made between nominal glyphs (kerning).
    let mut pairs: BTreeMap<(u16, u16), i16> = BTreeMap::new();
    let shaper = face.shaper_data.shaper(&face.font_ref).build();
    for text in texts {
        let mut buffer = UnicodeBuffer::new();
        buffer.push_str(text);
        buffer.guess_segment_properties();
        let output = shaper.shape(buffer, harfrust::ShapeOptions::new());
        let infos = output.glyph_infos();
        let positions = output.glyph_positions();
        let glyph_of = |index: usize| -> Option<u16> {
            let info = infos.get(index)?;
            let character = text.get(info.cluster as usize..)?.chars().next()?;
            let new = *nominal.get(&character)?;
            // Only glyphs the browser gets from the cmap (no substitutions).
            let old = cmap.map_codepoint(character)?.to_u32();
            (old == info.glyph_id).then_some(new)
        };
        for index in 0..infos.len().saturating_sub(1) {
            let (Some(left), Some(right)) = (glyph_of(index), glyph_of(index + 1)) else {
                continue;
            };
            let advance = hmtx
                .advance(GlyphId::new(infos[index].glyph_id))
                .map_or(0, i32::from);
            let adjustment = positions[index].x_advance - advance;
            if adjustment != 0
                && let Ok(adjustment) = i16::try_from(adjustment)
            {
                pairs.entry((left, right)).or_insert(adjustment);
            }
        }
    }
    let subset = subsetter::subset(face.data, face.index, &remapper).ok()?;
    let mut tables = read_tables(&subset)?;
    tables.retain(|(tag, _)| ![*b"cmap", *b"OS/2", *b"kern"].contains(tag));
    tables.push((*b"cmap", cmap_table(&nominal)));
    if let Some(os2) = face.font_ref.table_data(Tag::new(b"OS/2")) {
        tables.push((*b"OS/2", os2.as_bytes().to_vec()));
    }
    if !pairs.is_empty() {
        tables.push((*b"kern", kern_table(&pairs)));
    }
    let flavor = subset.get(..4)?.try_into().ok()?;
    Some(WebFont {
        bytes: write_font(flavor, tables),
        characters: nominal.into_keys().collect(),
    })
}

/// The tables of an sfnt font file.
fn read_tables(font: &[u8]) -> Option<Vec<([u8; 4], Vec<u8>)>> {
    let u16_at = |at: usize| -> Option<u16> {
        Some(u16::from_be_bytes(font.get(at..at + 2)?.try_into().ok()?))
    };
    let u32_at = |at: usize| -> Option<usize> {
        Some(u32::from_be_bytes(font.get(at..at + 4)?.try_into().ok()?) as usize)
    };
    let count = usize::from(u16_at(4)?);
    (0..count)
        .map(|index| {
            let record = 12 + 16 * index;
            let tag: [u8; 4] = font.get(record..record + 4)?.try_into().ok()?;
            let offset = u32_at(record + 8)?;
            let length = u32_at(record + 12)?;
            Some((tag, font.get(offset..offset + length)?.to_vec()))
        })
        .collect()
}

/// An sfnt file from `tables`, with checksums and `head.checkSumAdjustment`.
fn write_font(flavor: [u8; 4], mut tables: Vec<([u8; 4], Vec<u8>)>) -> Vec<u8> {
    tables.sort_by_key(|(tag, _)| *tag);
    let count = tables.len() as u16;
    let entry_selector = 15 - count.max(1).leading_zeros() as u16;
    let search_range = (1u16 << entry_selector) * 16;
    let mut output = Vec::new();
    output.extend_from_slice(&flavor);
    for value in [
        count,
        search_range,
        entry_selector,
        count * 16 - search_range,
    ] {
        output.extend_from_slice(&value.to_be_bytes());
    }
    let mut offset = 12 + 16 * tables.len();
    let mut head_offset = None;
    for (tag, data) in &mut tables {
        if tag == b"head" && data.len() >= 12 {
            data[8..12].fill(0);
            head_offset = Some(offset);
        }
        output.extend_from_slice(tag);
        output.extend_from_slice(&checksum(data).to_be_bytes());
        output.extend_from_slice(&(offset as u32).to_be_bytes());
        output.extend_from_slice(&(data.len() as u32).to_be_bytes());
        offset += data.len().div_ceil(4) * 4;
    }
    for (_, data) in &tables {
        output.extend_from_slice(data);
        output.resize(output.len().div_ceil(4) * 4, 0);
    }
    if let Some(head) = head_offset {
        let adjustment = 0xB1B0_AFBA_u32.wrapping_sub(checksum(&output));
        output[head + 8..head + 12].copy_from_slice(&adjustment.to_be_bytes());
    }
    output
}

fn checksum(data: &[u8]) -> u32 {
    data.chunks(4).fold(0u32, |sum, chunk| {
        let mut word = [0; 4];
        word[..chunk.len()].copy_from_slice(chunk);
        sum.wrapping_add(u32::from_be_bytes(word))
    })
}

/// A `cmap` with a format 4 subtable (BMP) and, when needed, format 12 (all).
fn cmap_table(nominal: &BTreeMap<char, u16>) -> Vec<u8> {
    let bmp: Vec<(u16, u16)> = nominal
        .iter()
        .filter_map(|(character, glyph)| {
            u16::try_from(u32::from(*character))
                .ok()
                .filter(|code| *code != 0xFFFF)
                .map(|code| (code, *glyph))
        })
        .collect();
    // Format 4: one segment per character, then the closing 0xFFFF segment.
    let segments = bmp.len() + 1;
    let mut format4 = Vec::new();
    let length = 16 + 8 * segments;
    let entry_selector = 15 - (segments as u16).leading_zeros() as u16;
    let search_range = 2 * (1u16 << entry_selector);
    for value in [
        4,
        length as u16,
        0,
        (segments * 2) as u16,
        search_range,
        entry_selector,
        (segments * 2) as u16 - search_range,
    ] {
        format4.extend_from_slice(&value.to_be_bytes());
    }
    for (code, _) in &bmp {
        format4.extend_from_slice(&code.to_be_bytes());
    }
    format4.extend_from_slice(&0xFFFF_u16.to_be_bytes());
    format4.extend_from_slice(&0_u16.to_be_bytes());
    for (code, _) in &bmp {
        format4.extend_from_slice(&code.to_be_bytes());
    }
    format4.extend_from_slice(&0xFFFF_u16.to_be_bytes());
    for (code, glyph) in &bmp {
        format4.extend_from_slice(&glyph.wrapping_sub(*code).to_be_bytes());
    }
    format4.extend_from_slice(&1_u16.to_be_bytes());
    format4.extend(std::iter::repeat_n(0, segments * 2));

    let wide = nominal
        .keys()
        .any(|character| u32::from(*character) > 0xFFFF);
    let mut format12 = Vec::new();
    if wide {
        format12.extend_from_slice(&12_u16.to_be_bytes());
        format12.extend_from_slice(&0_u16.to_be_bytes());
        format12.extend_from_slice(&((16 + 12 * nominal.len()) as u32).to_be_bytes());
        format12.extend_from_slice(&0_u32.to_be_bytes());
        format12.extend_from_slice(&(nominal.len() as u32).to_be_bytes());
        for (character, glyph) in nominal {
            let code = u32::from(*character);
            format12.extend_from_slice(&code.to_be_bytes());
            format12.extend_from_slice(&code.to_be_bytes());
            format12.extend_from_slice(&u32::from(*glyph).to_be_bytes());
        }
    }
    let records: u16 = if wide { 2 } else { 1 };
    let mut table = Vec::new();
    table.extend_from_slice(&0_u16.to_be_bytes());
    table.extend_from_slice(&records.to_be_bytes());
    let first = 4 + 8 * u32::from(records);
    table.extend_from_slice(&3_u16.to_be_bytes());
    table.extend_from_slice(&1_u16.to_be_bytes());
    table.extend_from_slice(&first.to_be_bytes());
    if wide {
        table.extend_from_slice(&3_u16.to_be_bytes());
        table.extend_from_slice(&10_u16.to_be_bytes());
        table.extend_from_slice(&(first + format4.len() as u32).to_be_bytes());
    }
    table.extend_from_slice(&format4);
    table.extend_from_slice(&format12);
    table
}

/// A version 0 `kern` table with one format 0 subtable (horizontal pairs).
fn kern_table(pairs: &BTreeMap<(u16, u16), i16>) -> Vec<u8> {
    // The subtable length is 16 bits: at most 10920 pairs.
    let pairs: Vec<_> = pairs.iter().take(10_920).collect();
    let count = pairs.len() as u16;
    let entry_selector = 15 - count.max(1).leading_zeros() as u16;
    let search_range = (1u16 << entry_selector) * 6;
    let mut table = Vec::new();
    for value in [
        0,
        1,
        0,
        14 + 6 * count,
        0x0001,
        count,
        search_range,
        entry_selector,
        (count * 6).saturating_sub(search_range),
    ] {
        table.extend_from_slice(&value.to_be_bytes());
    }
    // BTreeMap order is (left, right): the order format 0 requires.
    for ((left, right), value) in pairs {
        table.extend_from_slice(&left.to_be_bytes());
        table.extend_from_slice(&right.to_be_bytes());
        table.extend_from_slice(&value.to_be_bytes());
    }
    table
}

/// `U+20-7E,U+2212`: CSS `unicode-range` of sorted characters.
fn unicode_range(characters: &[char]) -> String {
    let mut ranges: Vec<(u32, u32)> = Vec::new();
    for code in characters.iter().map(|character| u32::from(*character)) {
        match ranges.last_mut() {
            Some((_, end)) if *end + 1 == code => *end = code,
            _ => ranges.push((code, code)),
        }
    }
    ranges
        .iter()
        .map(|(start, end)| {
            if start == end {
                format!("U+{start:X}")
            } else {
                format!("U+{start:X}-{end:X}")
            }
        })
        .collect::<Vec<_>>()
        .join(",")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn ranges_merge_consecutive_characters() {
        assert_eq!(
            unicode_range(&['a', 'b', 'c', 'x', '\u{2212}']),
            "U+61-63,U+78,U+2212"
        );
    }

    #[test]
    fn subset_of_an_embedded_face_has_a_cmap_and_kerning() {
        let face = crate::text::loaded(crate::text::Font::default());
        let texts = BTreeSet::from(["AVATAR".to_owned(), "\u{4E00}".to_owned()]);
        let font = web_font(&face, &texts, false).expect("subsets");
        assert!(!font.characters.contains(&'z'));
        assert!(
            web_font(&face, &texts, true)
                .expect("subsets")
                .characters
                .contains(&'z')
        );
        let parsed = read_fonts::FontRef::new(&font.bytes).expect("parses");
        let cmap = parsed.cmap().expect("cmap");
        let a = cmap.map_codepoint('A').expect("A mapped");
        let v = cmap.map_codepoint('V').expect("V mapped");
        assert_ne!(a.to_u32(), 0);
        // The CJK character is not in Liberation Sans: not mapped, not in the range.
        assert!(cmap.map_codepoint('\u{4E00}').is_none());
        assert!(!font.characters.contains(&'\u{4E00}'));
        assert!(parsed.os2().is_ok());
        let kern = parsed.table_data(Tag::new(b"kern")).expect("kern table");
        // A kerned pair 'A' 'V' is present with a negative value.
        let bytes = kern.as_bytes();
        let count = usize::from(u16::from_be_bytes([bytes[10], bytes[11]]));
        let found = (0..count).any(|index| {
            let at = 18 + 6 * index;
            let left = u32::from(u16::from_be_bytes([bytes[at], bytes[at + 1]]));
            let right = u32::from(u16::from_be_bytes([bytes[at + 2], bytes[at + 3]]));
            let value = i16::from_be_bytes([bytes[at + 4], bytes[at + 5]]);
            left == a.to_u32() && right == v.to_u32() && value < 0
        });
        assert!(found);
        // Checksums: the whole file sums to the magic number.
        assert_eq!(checksum(&font.bytes), 0xB1B0_AFBA);
    }
}

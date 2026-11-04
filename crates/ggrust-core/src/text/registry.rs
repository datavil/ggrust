//! Fonts the user registers at run time (`register_font`), and the parsed form of
//! every font ggrust draws with (embedded or registered).
//!
//! What it does: reads `.ttf`, `.otf`, `.ttc` and `.otc` files, takes the family
//! name and the face (regular, bold, italic, bold italic) from each font's `name`
//! and `OS/2` tables, and keeps them in one process-wide, append-only registry.
//! Measuring and every renderer read faces from here, so a registered font is
//! measured and drawn with the same data everywhere.
//!
//! Why a process-wide registry: text is measured deep inside layout through free
//! functions ([`super::measure`]); threading font data through every caller would
//! touch every layout module. The registry holds only what the user registered
//! explicitly (no system font lookup), so output stays deterministic, as with
//! matplotlib's `font_manager.addfont`. Each process registers its own fonts.
//!
//! What it does not do: look up system fonts by name, synthesize missing faces, or
//! select variable-font instances (the default instance is used).

use std::path::{Path, PathBuf};
use std::sync::{Arc, RwLock};

use harfrust::{FontRef, ShaperData};
use read_fonts::types::NameId;
use read_fonts::{FileRef, TableProvider};

use super::FontFace;
use crate::{Error, Result};

/// A parsed font face, ready for shaping, subsetting and rasterising.
pub(crate) struct LoadedFont {
    /// The whole font file (a collection holds several faces).
    pub(crate) data: &'static [u8],
    /// Face index in `data` (0 unless a collection).
    pub(crate) index: u32,
    pub(crate) font_ref: FontRef<'static>,
    pub(crate) shaper_data: ShaperData,
    pub(crate) units_per_em: u16,
    pub(crate) ascender: i16,
    /// Positive distance below the baseline.
    pub(crate) descender: i16,
    /// PostScript name (`name` ID 6), for PDF.
    pub(crate) postscript_name: String,
    /// `post.isFixedPitch`.
    pub(crate) monospaced: bool,
    /// Whether the license (`OS/2.fsType`) allows embedding outlines in documents.
    pub(crate) embeddable: bool,
    /// Whether the outlines are CFF (an `.otf`), not TrueType `glyf`.
    pub(crate) cff: bool,
    /// Unique family name for SVG `@font-face` and the PNG font database:
    /// `ggrust-` and a hash of the face, so the same face gets the same name in
    /// every document.
    pub(crate) css_name: String,
}

impl LoadedFont {
    /// Parse face `index` of `data` (checked with [`check_face`] first);
    /// `file_hash` is [`content_hash`] of `data`.
    pub(crate) fn parse(
        data: &'static [u8],
        index: u32,
        file_hash: u64,
    ) -> std::result::Result<Self, String> {
        let font_ref = check_face(data, index)?;
        let head = font_ref
            .head()
            .map_err(|_| "it has no readable 'head' table".to_owned())?;
        let hhea = font_ref
            .hhea()
            .map_err(|_| "it has no readable 'hhea' table".to_owned())?;
        let cff = font_ref.cff().is_ok();
        let fs_type = font_ref.os2().map_or(0, |os2| os2.fs_type());
        // Bits 0-3 == 2: restricted license embedding; bit 8: no subsetting (ggrust
        // always embeds subsets); bit 9: bitmap embedding only.
        let embeddable = fs_type & 0x000F != 0x0002 && fs_type & 0x0300 == 0;
        let postscript_name = name_string(&font_ref, NameId::POSTSCRIPT_NAME)
            .map(|name| {
                name.chars()
                    .filter(|character| {
                        character.is_ascii_graphic() && !"[](){}<>/%".contains(*character)
                    })
                    .collect::<String>()
            })
            .filter(|name| !name.is_empty())
            .unwrap_or_else(|| format!("Font{index}"));
        let hash = (file_hash ^ u64::from(index)).wrapping_mul(0x0100_0000_01b3);
        Ok(LoadedFont {
            data,
            index,
            shaper_data: ShaperData::new(&font_ref),
            units_per_em: head.units_per_em(),
            ascender: hhea.ascender().to_i16(),
            descender: hhea.descender().to_i16().saturating_abs(),
            postscript_name,
            monospaced: font_ref.post().is_ok_and(|post| post.is_fixed_pitch() != 0),
            embeddable,
            cff,
            css_name: format!("ggrust-{:012x}", hash >> 16),
            font_ref,
        })
    }

    /// Family name (`name` ID 1) of the face, and its face slot from the `OS/2`
    /// selection flags (or `head.macStyle` without `OS/2`).
    fn family_and_face(&self) -> (Option<String>, FontFace) {
        let (bold, italic) = match self.font_ref.os2() {
            Ok(os2) => {
                let flags = os2.fs_selection();
                (
                    flags.contains(read_fonts::tables::os2::SelectionFlags::BOLD),
                    flags.contains(read_fonts::tables::os2::SelectionFlags::ITALIC),
                )
            }
            Err(_) => match self.font_ref.head() {
                Ok(head) => {
                    let style = head.mac_style();
                    (
                        style.contains(read_fonts::tables::head::MacStyle::BOLD),
                        style.contains(read_fonts::tables::head::MacStyle::ITALIC),
                    )
                }
                Err(_) => (false, false),
            },
        };
        let face = match (bold, italic) {
            (false, false) => FontFace::Plain,
            (true, false) => FontFace::Bold,
            (false, true) => FontFace::Italic,
            (true, true) => FontFace::BoldItalic,
        };
        (name_string(&self.font_ref, NameId::FAMILY_NAME), face)
    }
}

/// Face `index` of `data`, if it has the tables ggrust needs; the error is the reason.
fn check_face(data: &[u8], index: u32) -> std::result::Result<FontRef<'_>, String> {
    let font_ref = FontRef::from_index(data, index)
        .map_err(|error| format!("it could not be parsed as a font ({error})"))?;
    if font_ref.head().is_err() {
        return Err("it has no readable 'head' table".to_owned());
    }
    if font_ref.hhea().is_err() || font_ref.hmtx().is_err() || font_ref.cmap().is_err() {
        return Err("it has no readable 'hhea', 'hmtx' or 'cmap' table".to_owned());
    }
    if font_ref.cff().is_err() && font_ref.glyf().is_err() {
        return Err(
            "it has neither TrueType ('glyf') nor CFF outlines (bitmap-only, CFF2 \
             and Type 1 fonts are not supported)"
                .to_owned(),
        );
    }
    Ok(font_ref)
}

/// FNV-1a hash of a whole font file (stable across runs and platforms).
pub(crate) fn content_hash(data: &[u8]) -> u64 {
    data.iter().fold(0xcbf2_9ce4_8422_2325, |hash, byte| {
        (hash ^ u64::from(*byte)).wrapping_mul(0x0100_0000_01b3)
    })
}

/// A `name` table string: Windows English first, then any Unicode record, then any.
fn name_string(font: &FontRef<'_>, id: NameId) -> Option<String> {
    let name = font.name().ok()?;
    let records = name.name_record();
    let rank = |record: &&read_fonts::tables::name::NameRecord| match (
        record.platform_id(),
        record.language_id(),
    ) {
        (3, 0x0409) => 0,
        (0 | 3, _) => 1,
        _ => 2,
    };
    let mut candidates: Vec<_> = records
        .iter()
        .filter(|record| record.name_id() == id)
        .collect();
    candidates.sort_by_key(rank);
    candidates.into_iter().find_map(|record| {
        let text: String = record.string(name.string_data()).ok()?.chars().collect();
        let text = text.trim().to_owned();
        (!text.is_empty()).then_some(text)
    })
}

/// One registered family: up to four faces.
struct Family {
    name: String,
    faces: [Option<Arc<LoadedFont>>; 4],
}

struct Registry {
    families: Vec<Family>,
    /// Every face ever loaded, so registering the same file again reuses it.
    loaded: Vec<Arc<LoadedFont>>,
    /// Bumped on every change (the PNG font database is rebuilt when it moves).
    generation: u64,
}

static REGISTRY: RwLock<Registry> = RwLock::new(Registry {
    families: Vec::new(),
    loaded: Vec::new(),
    generation: 0,
});

fn read_registry() -> std::sync::RwLockReadGuard<'static, Registry> {
    // The registry is never left half-written: a poisoned lock still holds valid data.
    REGISTRY
        .read()
        .unwrap_or_else(std::sync::PoisonError::into_inner)
}

/// File extensions [`register_font_dir`] picks up (case-insensitive).
pub const FONT_EXTENSIONS: [&str; 4] = ["ttf", "otf", "ttc", "otc"];

/// Register every face in the font file at `path` and return the family names
/// registered, in file order. `family` overrides the family name read from the
/// file. A face replaces an earlier registration of the same family and face;
/// within one file, the first face for a slot wins.
pub fn register_font(path: &Path, family: Option<&str>) -> Result<Vec<String>> {
    if let Some(family) = family
        && family.trim().is_empty()
    {
        return Err(Error::invalid_font(
            path,
            "family must be a non-empty name; pass family=None to read it from the file",
        ));
    }
    let data = match std::fs::read(path) {
        Ok(data) => data,
        Err(error) if error.kind() == std::io::ErrorKind::NotFound => {
            return Err(Error::font_not_found(
                path,
                "no such file; check the path (relative paths start at the current \
                 working directory)",
            ));
        }
        Err(error) if path.is_dir() => {
            return Err(Error::invalid_font(
                path,
                format!(
                    "it is a directory, not a font file ({error}); use register_font_dir() \
                     to register every font in a directory"
                ),
            ));
        }
        Err(error) => {
            return Err(Error::invalid_font(
                path,
                format!("the file could not be read: {error}"),
            ));
        }
    };
    register_font_data(data, family).map_err(|reason| Error::invalid_font(path, reason))
}

/// What [`register_font_dir`] did.
#[derive(Debug, Clone, PartialEq, Eq, Default)]
pub struct DirectoryFonts {
    /// Families registered, each once, in order.
    pub families: Vec<String>,
    /// Files that could not be registered, with the reason (skipped).
    pub skipped: Vec<(PathBuf, String)>,
}

/// Register every font file (see [`FONT_EXTENSIONS`]) under the directory `path`,
/// searched recursively in sorted path order. Files that are not usable fonts are
/// skipped and listed (a font folder often holds a few); it fails only when no
/// file could be registered.
pub fn register_font_dir(path: &Path) -> Result<DirectoryFonts> {
    if !path.exists() {
        return Err(Error::font_not_found(
            path,
            "no such directory; check the path",
        ));
    }
    if !path.is_dir() {
        return Err(Error::invalid_font(
            path,
            "it is not a directory; use register_font() for a single font file",
        ));
    }
    let mut files = Vec::new();
    collect_font_files(path, &mut files).map_err(|error| {
        Error::invalid_font(path, format!("the directory could not be read: {error}"))
    })?;
    files.sort();
    if files.is_empty() {
        return Err(Error::font_not_found(
            path,
            format!(
                "the directory holds no font files (looked for .{} files, recursively)",
                FONT_EXTENSIONS.join(", .")
            ),
        ));
    }
    let count = files.len();
    let mut result = DirectoryFonts::default();
    for file in files {
        match register_font(&file, None) {
            Ok(names) => {
                for name in names {
                    if !result.families.contains(&name) {
                        result.families.push(name);
                    }
                }
            }
            Err(Error::FontNotFound { message, .. } | Error::InvalidFont { message, .. }) => {
                result.skipped.push((file, message));
            }
            Err(error) => result.skipped.push((file, error.reason())),
        }
    }
    if result.families.is_empty()
        && let Some((file, reason)) = result.skipped.first()
    {
        return Err(Error::invalid_font(
            path,
            format!(
                "none of its {count} font files could be registered; for example '{}': \
                 {reason}",
                file.display()
            ),
        ));
    }
    Ok(result)
}

fn collect_font_files(directory: &Path, files: &mut Vec<PathBuf>) -> std::io::Result<()> {
    for entry in std::fs::read_dir(directory)? {
        let path = entry?.path();
        if path.is_dir() {
            collect_font_files(&path, files)?;
        } else if path.extension().is_some_and(|extension| {
            FONT_EXTENSIONS
                .iter()
                .any(|known| extension.eq_ignore_ascii_case(known))
        }) {
            files.push(path);
        }
    }
    Ok(())
}

/// Register the faces of font `data` (see [`register_font`]); errors are reasons
/// without the path.
pub fn register_font_data(
    data: Vec<u8>,
    family: Option<&str>,
) -> std::result::Result<Vec<String>, String> {
    let count = match FileRef::new(&data) {
        Ok(FileRef::Font(_)) => 1,
        Ok(FileRef::Collection(collection)) => collection.len(),
        Err(error) => {
            return Err(format!(
                "it is not a font file ({error}); expected a TrueType or OpenType font \
                 (.ttf, .otf, .ttc, .otc)"
            ));
        }
    };
    if count == 0 {
        return Err("the font collection holds no fonts".to_owned());
    }
    // Check every face before anything is kept, so a bad file leaks nothing.
    for index in 0..count {
        check_face(&data, index).map_err(|reason| {
            if count > 1 {
                format!("face {index} of the collection: {reason}")
            } else {
                reason
            }
        })?;
    }
    let file_hash = content_hash(&data);
    let mut registry = REGISTRY
        .write()
        .unwrap_or_else(std::sync::PoisonError::into_inner);
    // The same bytes registered again reuse the faces already loaded (no new copy).
    let existing = registry
        .loaded
        .iter()
        .find(|loaded| loaded.data == data.as_slice())
        .map(|loaded| loaded.data);
    let data: &'static [u8] = existing.unwrap_or_else(|| Box::leak(data.into_boxed_slice()));
    let mut parsed = Vec::with_capacity(count as usize);
    for index in 0..count {
        let loaded = match registry
            .loaded
            .iter()
            .find(|loaded| std::ptr::eq(loaded.data, data) && loaded.index == index)
        {
            Some(loaded) => Arc::clone(loaded),
            None => {
                let loaded = LoadedFont::parse(data, index, file_hash).map_err(|reason| {
                    if count > 1 {
                        format!("face {index} of the collection: {reason}")
                    } else {
                        reason
                    }
                })?;
                Arc::new(loaded)
            }
        };
        let (name, face) = loaded.family_and_face();
        let name = match (family, name) {
            (Some(family), _) => family.trim().to_owned(),
            (None, Some(name)) => name,
            (None, None) => {
                return Err(
                    "it has no family name in its 'name' table; pass family= to name it".to_owned(),
                );
            }
        };
        parsed.push((name, face, loaded));
    }
    let mut names: Vec<String> = Vec::new();
    let mut filled: Vec<(String, FontFace)> = Vec::new();
    for (name, face, loaded) in parsed {
        if !registry
            .loaded
            .iter()
            .any(|known| Arc::ptr_eq(known, &loaded))
        {
            registry.loaded.push(Arc::clone(&loaded));
        }
        let key = name.to_lowercase();
        if filled.contains(&(key.clone(), face)) {
            continue;
        }
        filled.push((key, face));
        let index = match registry
            .families
            .iter()
            .position(|family| family.name.eq_ignore_ascii_case(&name))
        {
            Some(index) => index,
            None => {
                registry.families.push(Family {
                    name: name.clone(),
                    faces: [None, None, None, None],
                });
                registry.families.len() - 1
            }
        };
        registry.families[index].faces[face as usize] = Some(loaded);
        if !names.contains(&registry.families[index].name) {
            names.push(registry.families[index].name.clone());
        }
    }
    registry.generation += 1;
    Ok(names)
}

/// Every registered family with its faces, in registration order.
pub fn registered_fonts() -> Vec<(String, Vec<FontFace>)> {
    read_registry()
        .families
        .iter()
        .map(|family| {
            let faces = FontFace::ALL
                .into_iter()
                .filter(|face| family.faces[*face as usize].is_some())
                .collect();
            (family.name.clone(), faces)
        })
        .collect()
}

/// The registered family named `name` (case-insensitive), as an id.
pub(crate) fn lookup(name: &str) -> Option<u16> {
    read_registry()
        .families
        .iter()
        .position(|family| family.name.eq_ignore_ascii_case(name))
        .and_then(|index| u16::try_from(index).ok())
}

/// Name of registered family `id`.
pub(crate) fn family_name(id: u16) -> String {
    read_registry()
        .families
        .get(usize::from(id))
        .map_or_else(String::new, |family| family.name.clone())
}

/// Names of every registered family (for suggestions).
pub(crate) fn family_names() -> Vec<String> {
    read_registry()
        .families
        .iter()
        .map(|family| family.name.clone())
        .collect()
}

/// The face of family `id` drawn for `face`: `face` itself, else the nearest
/// registered one, keeping the slant first, as CSS font matching does.
pub(crate) fn face(id: u16, face: FontFace) -> Option<(Arc<LoadedFont>, FontFace)> {
    let registry = read_registry();
    let family = registry.families.get(usize::from(id))?;
    let order = match face {
        FontFace::Plain => [
            FontFace::Plain,
            FontFace::Bold,
            FontFace::Italic,
            FontFace::BoldItalic,
        ],
        FontFace::Bold => [
            FontFace::Bold,
            FontFace::Plain,
            FontFace::BoldItalic,
            FontFace::Italic,
        ],
        FontFace::Italic => [
            FontFace::Italic,
            FontFace::BoldItalic,
            FontFace::Plain,
            FontFace::Bold,
        ],
        FontFace::BoldItalic => [
            FontFace::BoldItalic,
            FontFace::Italic,
            FontFace::Bold,
            FontFace::Plain,
        ],
    };
    order.into_iter().find_map(|candidate| {
        family.faces[candidate as usize]
            .as_ref()
            .map(|loaded| (Arc::clone(loaded), candidate))
    })
}

/// Every registered face (each once), for the PNG font database.
pub(crate) fn all_faces() -> (u64, Vec<Arc<LoadedFont>>) {
    let registry = read_registry();
    let mut faces: Vec<Arc<LoadedFont>> = Vec::new();
    for family in &registry.families {
        for loaded in family.faces.iter().flatten() {
            if !faces.iter().any(|known| Arc::ptr_eq(known, loaded)) {
                faces.push(Arc::clone(loaded));
            }
        }
    }
    (registry.generation, faces)
}

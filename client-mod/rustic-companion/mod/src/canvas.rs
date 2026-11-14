//! The companion's canvas: everything Rustic draws in the game's GUI that is not a flat colour —
//! smooth text set in Inter (SIL OFL 1.1, `assets/`; Chinese, Japanese, Korean and Thai in the
//! system's own fonts, [`Face::Cjk`], [`Face::Thai`], and every other script Inter lacks in the
//! system's font for it, [`Face::Script`], shaped with rustybuzz and ordered by the Unicode
//! bidirectional algorithm where the script needs it), the round corners of its plates, its
//! pictures (the logo, icons, flags, server icons) — is rasterised here, at the screen's exact
//! pixel size, into the HUD atlas's mod slot (`horizon_client::hud::MOD_IMAGE_SPRITE`, the slot a
//! native mod may fill), and drawn from there one atlas pixel to one screen pixel.
//!
//! Pieces are packed on shelves as they are first drawn and uploaded with the frame's own batch
//! (`HudBatch::patches`, as the game uploads its glyphs), so a new glyph shows in the frame that
//! asks for it. When the slot is full, the frame draws what it has and the next starts over; a new
//! atlas (every resource reload) starts over too ([`Canvas::begin`], from the widget skin's
//! `begin`, Rustic's Horizon patch U15).
use ab_glyph::{Font, FontArc, FontRef, FontVec, GlyphId, PxScale, ScaleFont, point};
use horizon_client::font::Patch;
use horizon_client::hud::{HudAtlas, HudBatch, Rect};
use rusticmc_companion_core::paint::{Align, Paint, Picture};
use rusticmc_companion_core::title::{self, Glyph, Image, Rgba, Weight};
use std::collections::HashMap;
use std::path::PathBuf;
use std::sync::{Arc, LazyLock, Mutex, MutexGuard};

/// Inter Medium and SemiBold (`assets/make_fonts.py`: Latin, Cyrillic, Greek, Vietnamese).
pub(crate) const INTER_MEDIUM: &[u8] = include_bytes!("../assets/Inter-Medium.ttf");
pub(crate) const INTER_SEMIBOLD: &[u8] = include_bytes!("../assets/Inter-SemiBold.ttf");
static MEDIUM: LazyLock<Option<FontArc>> = LazyLock::new(|| inter(INTER_MEDIUM));
static SEMIBOLD: LazyLock<Option<FontArc>> = LazyLock::new(|| inter(INTER_SEMIBOLD));

fn inter(bytes: &'static [u8]) -> Option<FontArc> {
    FontRef::try_from_slice(bytes).ok().map(FontArc::new)
}

/// The face a character is drawn in: Inter, or for the Han, kana and Hangul that Inter lacks, a
/// system font of a region (Chinese, Japanese and Korean Han differ in shape), or for Thai a
/// system Thai font.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum Face {
    Inter(Weight),
    Cjk(Region),
    Thai,
    Script(Script),
}

/// A script Inter does not draw, drawn in the system's font for it.
#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum Script {
    Arabic,
    Hebrew,
    Armenian,
    Georgian,
    Devanagari,
    Bengali,
    Gurmukhi,
    Gujarati,
    Oriya,
    Tamil,
    Telugu,
    Kannada,
    Malayalam,
    Sinhala,
    Lao,
    Tibetan,
    Myanmar,
    Khmer,
    Ethiopic,
    /// Runic, Gothic, Old Italic: the game's historic languages.
    Historic,
    /// Anything else Inter lacks (symbols, other scripts).
    Symbol,
}

impl Script {
    /// The script of `c`, for one Inter does not cover.
    fn of(c: char) -> Option<Script> {
        use Script::*;
        Some(match u32::from(c) {
            0x0590..=0x05FF | 0xFB1D..=0xFB4F => Hebrew,
            0x0600..=0x06FF
            | 0x0750..=0x077F
            | 0x08A0..=0x08FF
            | 0xFB50..=0xFDFF
            | 0xFE70..=0xFEFF => Arabic,
            0x0530..=0x058F | 0xFB13..=0xFB17 => Armenian,
            0x10A0..=0x10FF | 0x1C90..=0x1CBF | 0x2D00..=0x2D2F => Georgian,
            0x0900..=0x097F | 0xA8E0..=0xA8FF => Devanagari,
            0x0980..=0x09FF => Bengali,
            0x0A00..=0x0A7F => Gurmukhi,
            0x0A80..=0x0AFF => Gujarati,
            0x0B00..=0x0B7F => Oriya,
            0x0B80..=0x0BFF => Tamil,
            0x0C00..=0x0C7F => Telugu,
            0x0C80..=0x0CFF => Kannada,
            0x0D00..=0x0D7F => Malayalam,
            0x0D80..=0x0DFF => Sinhala,
            0x0E80..=0x0EFF => Lao,
            0x0F00..=0x0FFF => Tibetan,
            0x1000..=0x109F => Myanmar,
            0x1780..=0x17FF => Khmer,
            0x1200..=0x139F | 0x2D80..=0x2DDF => Ethiopic,
            0x16A0..=0x16FF | 0x10300..=0x1034F => Historic,
            _ => return None,
        })
    }

    /// Its letters change shape, join or reorder: drawn through the shaper.
    fn shaped(self) -> bool {
        !matches!(
            self,
            Script::Armenian | Script::Georgian | Script::Ethiopic | Script::Historic
        )
    }

    /// The system fonts that draw it (file, face in a collection): Windows', macOS', Linux's Noto.
    fn files(self) -> &'static [(&'static str, u32)] {
        use Script::*;
        match self {
            Arabic => &[
                ("segoeui.ttf", 0),
                ("GeezaPro.ttc", 0),
                ("NotoSansArabic-Regular.ttf", 0),
                ("NotoNaskhArabic-Regular.ttf", 0),
            ],
            Hebrew => &[
                ("segoeui.ttf", 0),
                ("ArialHB.ttc", 0),
                ("NotoSansHebrew-Regular.ttf", 0),
            ],
            Armenian => &[
                ("segoeui.ttf", 0),
                ("Mshtakan.ttc", 0),
                ("NotoSansArmenian-Regular.ttf", 0),
            ],
            Georgian => &[
                ("segoeui.ttf", 0),
                ("Georgia.ttf", 0),
                ("NotoSansGeorgian-Regular.ttf", 0),
            ],
            Devanagari => &[
                ("Nirmala.ttc", 0),
                ("Nirmala.ttf", 0),
                ("Kohinoor.ttc", 0),
                ("DevanagariMT.ttc", 0),
                ("NotoSansDevanagari-Regular.ttf", 0),
            ],
            Bengali => &[
                ("Nirmala.ttc", 0),
                ("Nirmala.ttf", 0),
                ("KohinoorBangla.ttc", 0),
                ("NotoSansBengali-Regular.ttf", 0),
            ],
            Gurmukhi => &[
                ("Nirmala.ttc", 0),
                ("Nirmala.ttf", 0),
                ("GurmukhiMN.ttc", 0),
                ("NotoSansGurmukhi-Regular.ttf", 0),
            ],
            Gujarati => &[
                ("Nirmala.ttc", 0),
                ("Nirmala.ttf", 0),
                ("KohinoorGujarati.ttc", 0),
                ("NotoSansGujarati-Regular.ttf", 0),
            ],
            Oriya => &[
                ("Nirmala.ttc", 0),
                ("Nirmala.ttf", 0),
                ("Oriya Sangam MN.ttc", 0),
                ("NotoSansOriya-Regular.ttf", 0),
            ],
            Tamil => &[
                ("Nirmala.ttc", 0),
                ("Nirmala.ttf", 0),
                ("Tamil Sangam MN.ttc", 0),
                ("NotoSansTamil-Regular.ttf", 0),
            ],
            Telugu => &[
                ("Nirmala.ttc", 0),
                ("Nirmala.ttf", 0),
                ("KohinoorTelugu.ttc", 0),
                ("NotoSansTelugu-Regular.ttf", 0),
            ],
            Kannada => &[
                ("Nirmala.ttc", 0),
                ("Nirmala.ttf", 0),
                ("Kannada Sangam MN.ttc", 0),
                ("NotoSansKannada-Regular.ttf", 0),
            ],
            Malayalam => &[
                ("Nirmala.ttc", 0),
                ("Nirmala.ttf", 0),
                ("Malayalam Sangam MN.ttc", 0),
                ("NotoSansMalayalam-Regular.ttf", 0),
            ],
            Sinhala => &[
                ("Nirmala.ttc", 0),
                ("Nirmala.ttf", 0),
                ("Sinhala Sangam MN.ttc", 0),
                ("NotoSansSinhala-Regular.ttf", 0),
            ],
            Lao => &[
                ("LeelawUI.ttf", 0),
                ("leelawad.ttf", 0),
                ("Lao Sangam MN.ttf", 0),
                ("NotoSansLao-Regular.ttf", 0),
            ],
            Tibetan => &[
                ("himalaya.ttf", 0),
                ("Kailasa.ttc", 0),
                ("NotoSerifTibetan-Regular.ttf", 0),
            ],
            Myanmar => &[
                ("mmrtext.ttf", 0),
                ("MyanmarMN.ttc", 0),
                ("NotoSansMyanmar-Regular.ttf", 0),
            ],
            Khmer => &[
                ("KhmerUI.ttf", 0),
                ("Khmer Sangam MN.ttf", 0),
                ("NotoSansKhmer-Regular.ttf", 0),
            ],
            Ethiopic => &[
                ("ebrima.ttf", 0),
                ("Kefa.ttc", 0),
                ("NotoSansEthiopic-Regular.ttf", 0),
            ],
            Historic => &[
                ("seguihis.ttf", 0),
                ("seguisym.ttf", 0),
                ("Apple Symbols.ttf", 0),
                ("NotoSansRunic-Regular.ttf", 0),
                ("NotoSansGothic-Regular.ttf", 0),
            ],
            Symbol => &[
                // Segoe UI first: it has the phonetic letters (Upside Down English's).
                ("segoeui.ttf", 0),
                ("seguisym.ttf", 0),
                ("Apple Symbols.ttf", 0),
                ("NotoSansSymbols-Regular.ttf", 0),
                ("NotoSansSymbols2-Regular.ttf", 0),
                ("DejaVuSans.ttf", 0),
            ],
        }
    }
}

/// A system font kept for good: ab_glyph's face over its bytes, the bytes for the shaper.
struct SysFont {
    font: FontArc,
    bytes: &'static [u8],
    index: u32,
}

/// The system font of each script, looked for once, the first time one of its characters shows.
static SCRIPT_FONTS: LazyLock<Mutex<HashMap<Script, Option<&'static SysFont>>>> =
    LazyLock::new(|| Mutex::new(HashMap::new()));

fn script_font(script: Script) -> Option<&'static SysFont> {
    let mut fonts = SCRIPT_FONTS.lock().unwrap_or_else(|e| e.into_inner());
    *fonts.entry(script).or_insert_with(|| {
        let dirs = font_dirs();
        for (name, index) in script.files() {
            for dir in &dirs {
                let Ok(bytes) = std::fs::read(dir.join(name)) else {
                    continue;
                };
                // Kept for the game's life: a few fonts, read once.
                let bytes: &'static [u8] = Box::leak(bytes.into_boxed_slice());
                if let Ok(face) = FontRef::try_from_slice_and_index(bytes, *index) {
                    log::info!(
                        "Rustic text: {script:?} characters in {}",
                        dir.join(name).display()
                    );
                    return Some(&*Box::leak(Box::new(SysFont {
                        font: FontArc::new(face),
                        bytes,
                        index: *index,
                    })));
                }
            }
        }
        log::warn!("Rustic text: no system font for {script:?} characters");
        None
    })
}

#[derive(Clone, Copy, Debug, PartialEq, Eq, Hash)]
enum Region {
    Japanese,
    Korean,
    Simplified,
    Traditional,
}

static JAPANESE: LazyLock<Option<FontArc>> = LazyLock::new(|| system_font(Region::Japanese));
static KOREAN: LazyLock<Option<FontArc>> = LazyLock::new(|| system_font(Region::Korean));
static SIMPLIFIED: LazyLock<Option<FontArc>> = LazyLock::new(|| system_font(Region::Simplified));
static TRADITIONAL: LazyLock<Option<FontArc>> = LazyLock::new(|| system_font(Region::Traditional));
static THAI: LazyLock<Option<FontArc>> = LazyLock::new(|| load_system_font("Thai", THAI_FILES));

/// Thai fonts of Windows (Leelawadee UI since 8.1, Tahoma everywhere), macOS (Thonburi, Sukhumvit)
/// and Linux (Noto Sans Thai, TLWG's Loma / Garuda / Waree).
const THAI_FILES: &[(&str, u32)] = &[
    ("LeelawUI.ttf", 0),
    ("leelawad.ttf", 0),
    ("tahoma.ttf", 0),
    ("Thonburi.ttc", 0),
    ("Thonburi.ttf", 0),
    ("SukhumvitSet.ttc", 0),
    ("NotoSansThaiUI-Regular.ttf", 0),
    ("NotoSansThai-Regular.ttf", 0),
    ("NotoSansThai-Regular.otf", 0),
    ("Loma.ttf", 0),
    ("Loma.otf", 0),
    ("Garuda.ttf", 0),
    ("Garuda.otf", 0),
    ("Waree.ttf", 0),
    ("Waree.otf", 0),
];

impl Region {
    /// The regions to look in for `lang` (a game code), its own first.
    fn order(lang: &str) -> [Region; 4] {
        use Region::*;
        match lang {
            l if l.starts_with("ja") => [Japanese, Simplified, Traditional, Korean],
            l if l.starts_with("ko") => [Korean, Simplified, Japanese, Traditional],
            "zh_tw" | "zh_hk" | "lzh" => [Traditional, Simplified, Japanese, Korean],
            _ => [Simplified, Traditional, Japanese, Korean],
        }
    }

    fn font(self) -> Option<&'static FontArc> {
        match self {
            Region::Japanese => JAPANESE.as_ref(),
            Region::Korean => KOREAN.as_ref(),
            Region::Simplified => SIMPLIFIED.as_ref(),
            Region::Traditional => TRADITIONAL.as_ref(),
        }
    }

    /// Font files of the region and their face in a collection, the usual ones of Windows, macOS
    /// and Linux (Noto Sans CJK's collection holds JP, KR, SC, TC in that order).
    fn files(self) -> &'static [(&'static str, u32)] {
        match self {
            Region::Japanese => &[
                ("YuGothM.ttc", 0),
                ("meiryo.ttc", 0),
                ("msgothic.ttc", 0),
                ("ヒラギノ角ゴシック W4.ttc", 0),
                ("ヒラギノ角ゴシック W3.ttc", 0),
                ("NotoSansCJK-Regular.ttc", 0),
                ("NotoSansCJKjp-Regular.otf", 0),
            ],
            Region::Korean => &[
                ("malgun.ttf", 0),
                ("AppleSDGothicNeo.ttc", 0),
                ("NotoSansCJK-Regular.ttc", 1),
                ("NotoSansCJKkr-Regular.otf", 0),
            ],
            Region::Simplified => &[
                ("msyh.ttc", 0),
                ("simsun.ttc", 0),
                ("PingFang.ttc", 0),
                ("Hiragino Sans GB.ttc", 0),
                ("NotoSansCJK-Regular.ttc", 2),
                ("NotoSansCJKsc-Regular.otf", 0),
                ("wqy-microhei.ttc", 0),
                ("DroidSansFallbackFull.ttf", 0),
            ],
            Region::Traditional => &[
                ("msjh.ttc", 0),
                ("mingliu.ttc", 0),
                ("PingFang.ttc", 0),
                ("NotoSansCJK-Regular.ttc", 3),
                ("NotoSansCJKtc-Regular.otf", 0),
                ("DroidSansFallbackFull.ttf", 0),
            ],
        }
    }
}

/// The system's font folders.
fn font_dirs() -> Vec<PathBuf> {
    let mut dirs = Vec::new();
    if let Some(windir) = std::env::var_os("WINDIR") {
        dirs.push(PathBuf::from(windir).join("Fonts"));
    }
    for d in [
        "/System/Library/Fonts",
        "/System/Library/Fonts/Supplemental",
        "/Library/Fonts",
        "/usr/share/fonts/opentype/noto",
        "/usr/share/fonts/noto-cjk",
        "/usr/share/fonts/google-noto-cjk",
        "/usr/share/fonts/truetype/noto",
        "/usr/share/fonts/noto",
        "/usr/share/fonts/wenquanyi/wqy-microhei",
        "/usr/share/fonts/truetype/wqy",
        "/usr/share/fonts/truetype/droid",
        "/usr/share/fonts/google-noto",
        "/usr/share/fonts/truetype/tlwg",
        "/usr/share/fonts/opentype/tlwg",
        "/usr/share/fonts/tlwg",
    ] {
        dirs.push(PathBuf::from(d));
    }
    dirs
}

/// The region's font read from the computer, once, the first time one of its characters shows
/// (fonts of the system are used where they are, never shipped: no licence question). None when
/// the computer has none: the character shows Inter's missing-glyph box, as before.
fn system_font(region: Region) -> Option<FontArc> {
    load_system_font(&format!("{region:?}"), region.files())
}

/// The first of `files` (name, face in a collection) found in the system's font folders.
fn load_system_font(what: &str, files: &[(&str, u32)]) -> Option<FontArc> {
    let dirs = font_dirs();
    for (name, index) in files {
        for dir in &dirs {
            let Ok(bytes) = std::fs::read(dir.join(name)) else {
                continue;
            };
            if let Ok(font) = FontVec::try_from_vec_and_index(bytes, *index) {
                log::info!(
                    "Rustic text: {what} characters in {}",
                    dir.join(name).display()
                );
                return Some(FontArc::new(font));
            }
        }
    }
    log::warn!("Rustic text: no system font for {what} characters");
    None
}

/// Han, kana, Hangul and the full-width forms around them (only these look for a system font).
fn is_cjk(c: char) -> bool {
    matches!(u32::from(c),
        0x1100..=0x11FF
            | 0x2E80..=0x2FDF
            | 0x3000..=0x30FF
            | 0x3130..=0x318F
            | 0x31F0..=0x31FF
            | 0x3400..=0x4DBF
            | 0x4E00..=0x9FFF
            | 0xAC00..=0xD7AF
            | 0xF900..=0xFAFF
            | 0xFF00..=0xFFEF
            | 0x20000..=0x3134F)
}

/// The Thai block (consonants, vowels, tone marks, digits).
fn is_thai(c: char) -> bool {
    matches!(u32::from(c), 0x0E00..=0x0E7F)
}

/// The face `c` is drawn in: Inter when it has it, else the first region's system font that has
/// it (the game language's region first; `regions` is filled at the first such character), or the
/// system's Thai font.
fn face_of(c: char, weight: Weight, regions: &mut Option<[Region; 4]>) -> Face {
    let inter = Face::Inter(weight);
    if let Some(script) = Script::of(c) {
        // Inter's own where it has the letter and the script needs no shaping.
        if !script.shaped() && font(weight).is_some_and(|f| f.glyph_id(c).0 != 0) {
            return inter;
        }
        return match script_font(script) {
            Some(f) if f.font.glyph_id(c).0 != 0 => Face::Script(script),
            _ => inter,
        };
    }
    if is_thai(c) {
        return match THAI.as_ref() {
            Some(f) if f.glyph_id(c).0 != 0 => Face::Thai,
            _ => inter,
        };
    }
    if font(weight).is_some_and(|f| f.glyph_id(c).0 != 0) || c.is_whitespace() || c.is_control() {
        return inter;
    }
    if !is_cjk(c) {
        // A symbol or a script with no font of its own here: the system's symbol font.
        return match script_font(Script::Symbol) {
            Some(f) if f.font.glyph_id(c).0 != 0 => Face::Script(Script::Symbol),
            _ => inter,
        };
    }
    let order = *regions.get_or_insert_with(|| Region::order(&crate::title_menu::lang()));
    order
        .into_iter()
        .find(|r| r.font().is_some_and(|f| f.glyph_id(c).0 != 0))
        .map_or(inter, Face::Cjk)
}

fn face_font(face: Face) -> Option<&'static FontArc> {
    match face {
        Face::Inter(weight) => font(weight),
        Face::Cjk(region) => region.font(),
        Face::Thai => THAI.as_ref(),
        Face::Script(script) => script_font(script).map(|f| &f.font),
    }
}

/// The logo, the official banner and the icon strip (launcher/assets/brand, `assets/make_assets.py`).
pub(crate) static LOGO: LazyLock<Option<Rgba>> =
    LazyLock::new(|| decode_png(include_bytes!("../assets/rustic-wordmark.png")));
static BANNER: LazyLock<Option<Rgba>> =
    LazyLock::new(|| decode_png(include_bytes!("../assets/rustic-banner.png")));
static ICONS: LazyLock<Option<Rgba>> =
    LazyLock::new(|| decode_png(include_bytes!("../assets/rustic-icons.png")));

/// The banner's width over its height.
pub fn banner_aspect() -> f32 {
    BANNER
        .as_ref()
        .map_or(4.0, |l| l.width as f32 / l.height as f32)
}

pub(crate) fn decode_png(bytes: &[u8]) -> Option<Rgba> {
    let mut decoder = png::Decoder::new(std::io::Cursor::new(bytes));
    decoder.set_transformations(png::Transformations::EXPAND | png::Transformations::STRIP_16);
    let mut reader = decoder.read_info().ok()?;
    let mut buf = vec![0; reader.output_buffer_size()?];
    let info = reader.next_frame(&mut buf).ok()?;
    buf.truncate(info.buffer_size());
    let pixels = match info.color_type {
        png::ColorType::Rgba => buf,
        png::ColorType::Rgb => buf
            .as_chunks::<3>()
            .0
            .iter()
            .flat_map(|p| [p[0], p[1], p[2], 255])
            .collect(),
        png::ColorType::GrayscaleAlpha => buf
            .as_chunks::<2>()
            .0
            .iter()
            .flat_map(|p| [p[0], p[0], p[0], p[1]])
            .collect(),
        png::ColorType::Grayscale => buf.iter().flat_map(|&g| [g, g, g, 255]).collect(),
        png::ColorType::Indexed => return None,
    };
    Some(Rgba {
        width: info.width,
        height: info.height,
        pixels,
    })
}

fn font(weight: Weight) -> Option<&'static FontArc> {
    match weight {
        Weight::Medium => MEDIUM.as_ref(),
        Weight::SemiBold => SEMIBOLD.as_ref(),
    }
}

/// ab_glyph's scale is the line's height: Inter's em is that over its line height.
fn scale_of(px: f32) -> PxScale {
    PxScale::from(px * title::LINE)
}

/// The scale that gives `face` an em of `px` pixels: Inter's own, and for a system font the same
/// em (system fonts' line heights differ from Inter's: YaHei and Leelawadee UI are taller, Yu
/// Gothic shorter, so the line-height scale would draw them smaller or larger than Inter).
fn face_scale(face: Face, font: &FontArc, px: f32) -> PxScale {
    match face {
        Face::Inter(_) => scale_of(px),
        _ => match font.units_per_em() {
            Some(em) if em > 0.0 => PxScale::from(px * font.height_unscaled() / em),
            _ => scale_of(px),
        },
    }
}

/// A glyph of a laid-out line: its face and glyph, its pen position from the line's start, and
/// an offset for the Thai marks moved off what they would hit (see [`layout`]).
#[derive(Clone, Copy, Debug, PartialEq)]
struct LaidGlyph {
    face: Face,
    id: GlyphId,
    pen: f32,
    dx: f32,
    dy: f32,
}

/// Thai marks drawn above a consonant (vowels, nikhahit, maitaikhu) and the tone marks and
/// thanthakhat that stack over them.
fn thai_upper(c: char) -> bool {
    matches!(u32::from(c), 0x0E31 | 0x0E34..=0x0E37 | 0x0E47 | 0x0E4D)
}
fn thai_tone(c: char) -> bool {
    matches!(u32::from(c), 0x0E48..=0x0E4C)
}
fn thai_lower(c: char) -> bool {
    matches!(u32::from(c), 0x0E38..=0x0E3A)
}
/// Consonants with a tall stem on the right (ป ฝ ฟ ฬ): marks above them move left.
fn thai_tall(c: char) -> bool {
    matches!(c, '\u{0E1B}' | '\u{0E1D}' | '\u{0E1F}' | '\u{0E2C}')
}
/// Consonants with a descender (ฎ ฏ): a vowel below them moves down.
fn thai_deep(c: char) -> bool {
    matches!(c, '\u{0E0E}' | '\u{0E0F}')
}

/// Lays `text` out at an em of `px` screen pixels: each glyph's face and pen position, and the
/// line's width.
///
/// There is no shaping engine here (no OpenType mark positioning), so Thai marks would sit at
/// their default place, a tone mark over an upper vowel on top of it (ที่ read as ที). A few rules
/// stand in for the font's anchors, from the glyphs' own outlines: a tone mark over an upper vowel
/// (or before sara am, whose nikhahit is above) goes up above it; marks over a consonant with a
/// tall stem move left of the stem; a vowel below ฎ or ฏ goes under the descender.
fn layout(text: &str, px: f32, weight: Weight) -> (Vec<LaidGlyph>, f32) {
    if text
        .chars()
        .any(|c| Script::of(c).is_some_and(Script::shaped))
    {
        return shaped_layout(text, px, weight);
    }
    simple_layout(text, px, weight)
}

/// Laid-out lines of the shaped scripts, kept (they are shaped once, not every frame).
type Shaped = HashMap<(String, u32, Weight), (Vec<LaidGlyph>, f32)>;
static SHAPED: LazyLock<Mutex<Shaped>> = LazyLock::new(|| Mutex::new(HashMap::new()));

/// [`layout`] of a line with a script that needs shaping: the Unicode bidirectional algorithm
/// orders its runs (right-to-left ones drawn from the right), each run of a shaped script goes
/// through rustybuzz in its system font (joining forms, conjuncts, reordered vowels, marks on
/// their anchors), the rest as [`simple_layout`] does.
fn shaped_layout(text: &str, px: f32, weight: Weight) -> (Vec<LaidGlyph>, f32) {
    let key = (text.to_owned(), px.to_bits(), weight);
    if let Some(hit) = SHAPED.lock().unwrap_or_else(|e| e.into_inner()).get(&key) {
        return hit.clone();
    }
    let mut out = Vec::new();
    let mut pen = 0.0f32;
    let mut regions = None;
    let bidi = unicode_bidi::BidiInfo::new(text, None);
    for para in &bidi.paragraphs {
        let (levels, runs) = bidi.visual_runs(para, para.range.clone());
        for run in runs {
            let rtl = levels.get(run.start).is_some_and(|l| l.is_rtl());
            // The run cut where the face changes (a mark stays with its letter's face).
            let mut pieces: Vec<(Face, std::ops::Range<usize>)> = Vec::new();
            for (i, c) in text[run.clone()].char_indices() {
                let at = run.start + i;
                let face = face_of(c, weight, &mut regions);
                let joins = pieces.last().is_some_and(|(f, _)| {
                    *f == face || (c.is_whitespace() && matches!(f, Face::Script(_)))
                });
                match pieces.last_mut() {
                    Some((_, r)) if joins => r.end = at + c.len_utf8(),
                    _ => pieces.push((face, at..at + c.len_utf8())),
                }
            }
            if rtl {
                pieces.reverse();
            }
            for (face, range) in pieces {
                let piece = &text[range];
                match face {
                    Face::Script(script) if script.shaped() => {
                        if let Some(sf) = script_font(script) {
                            pen += shape_run(&mut out, sf, script, piece, rtl, px, pen);
                        }
                    }
                    _ => {
                        let piece: String = if rtl {
                            piece.chars().rev().collect()
                        } else {
                            piece.to_owned()
                        };
                        let (glyphs, width) = simple_layout(&piece, px, weight);
                        out.extend(glyphs.into_iter().map(|g| LaidGlyph {
                            pen: g.pen + pen,
                            ..g
                        }));
                        pen += width;
                    }
                }
            }
        }
    }
    let laid = (out, pen);
    let mut cache = SHAPED.lock().unwrap_or_else(|e| e.into_inner());
    if cache.len() > 2048 {
        cache.clear();
    }
    cache.insert(key, laid.clone());
    laid
}

/// `piece` shaped by rustybuzz in `sf`, its glyphs placed from `pen`: its advance.
fn shape_run(
    out: &mut Vec<LaidGlyph>,
    sf: &SysFont,
    script: Script,
    piece: &str,
    rtl: bool,
    px: f32,
    pen: f32,
) -> f32 {
    let Some(face) = rustybuzz::Face::from_slice(sf.bytes, sf.index) else {
        return 0.0;
    };
    let mut buffer = rustybuzz::UnicodeBuffer::new();
    buffer.push_str(piece);
    buffer.guess_segment_properties();
    buffer.set_direction(if rtl {
        rustybuzz::Direction::RightToLeft
    } else {
        rustybuzz::Direction::LeftToRight
    });
    let glyphs = rustybuzz::shape(&face, &[], buffer);
    let k = px / face.units_per_em().max(1) as f32;
    let mut x = 0.0;
    for (info, pos) in glyphs.glyph_infos().iter().zip(glyphs.glyph_positions()) {
        out.push(LaidGlyph {
            face: Face::Script(script),
            id: GlyphId(u16::try_from(info.glyph_id).unwrap_or(0)),
            pen: pen + x + pos.x_offset as f32 * k,
            dx: 0.0,
            dy: -(pos.y_offset as f32) * k,
        });
        x += pos.x_advance as f32 * k;
    }
    x
}

/// [`layout`] for text with no script that needs shaping: glyph after glyph with kerning, and
/// Thai's marks moved by the rules below.
fn simple_layout(text: &str, px: f32, weight: Weight) -> (Vec<LaidGlyph>, f32) {
    let mut out = Vec::new();
    let mut regions = None;
    let mut pen = 0.0;
    let mut prev: Option<(Face, GlyphId)> = None;
    let chars: Vec<char> = text.chars().collect();
    // The Thai cluster: its base consonant and the upper mark drawn on it so far.
    let mut base: Option<char> = None;
    let mut upper: Option<char> = None;
    for (i, &c) in chars.iter().enumerate() {
        let face = face_of(c, weight, &mut regions);
        let Some(f) = face_font(face) else {
            continue;
        };
        let sf = f.as_scaled(face_scale(face, f, px));
        let id = sf.glyph_id(c);
        if let Some((pf, p)) = prev
            && pf == face
        {
            pen += sf.kern(p, id);
        }
        prev = Some((face, id));
        let (mut dx, mut dy) = (0.0, 0.0);
        if face == Face::Thai {
            // Ink top and bottom in font units, y up (ab_glyph's outline bounds keep the top in
            // `min.y` and the bottom in `max.y`).
            let units = |c: char| {
                f.outline(f.glyph_id(c))
                    .map(|o| (o.bounds.min.y, o.bounds.max.y))
            };
            let gap = f.units_per_em().unwrap_or(1000.0) * 0.04;
            let mark = thai_upper(c) || thai_tone(c) || thai_lower(c);
            if !mark && c != '\u{0E33}' {
                base = Some(c);
                upper = None;
            }
            if thai_tone(c) {
                // Sara am after the tone mark puts its nikhahit under it.
                let under =
                    upper.or_else(|| (chars.get(i + 1) == Some(&'\u{0E33}')).then_some('\u{0E4D}'));
                if let (Some(u), Some(t)) = (under.and_then(units), units(c)) {
                    let lift = u.0 + gap - t.1;
                    if lift > 0.0 {
                        dy = -lift * sf.v_scale_factor();
                    }
                }
            }
            if thai_upper(c) {
                upper = Some(c);
            }
            if (thai_upper(c) || thai_tone(c) || c == '\u{0E33}') && base.is_some_and(thai_tall) {
                // Left of the stem: about its width and a gap.
                let shift = f.units_per_em().unwrap_or(1000.0) * 0.14;
                if c != '\u{0E33}' {
                    dx = -shift * sf.h_scale_factor();
                }
            }
            if thai_lower(c)
                && base.is_some_and(thai_deep)
                && let (Some(b), Some(l)) = (base.and_then(units), units(c))
            {
                let drop = l.0 - b.1 + gap;
                if drop > 0.0 {
                    dy = drop * sf.v_scale_factor();
                }
            }
        } else {
            base = None;
            upper = None;
        }
        out.push(LaidGlyph {
            face,
            id,
            pen,
            dx,
            dy,
        });
        pen += sf.h_advance(id);
    }
    (out, pen)
}

/// A line's width in screen pixels at an em of `px` screen pixels.
pub fn measure_px(text: &str, px: f32, weight: Weight) -> f32 {
    if font(weight).is_none() {
        return text.chars().count() as f32 * px * 0.55;
    }
    layout(text, px, weight).1
}

/// The ARGB colour as the HUD's vertices take it (ABGR).
pub fn abgr(argb: u32) -> u32 {
    let [a, r, g, b] = argb.to_be_bytes();
    u32::from_be_bytes([a, b, g, r])
}

/// Where the mod slot is in the atlas.
#[derive(Clone, Copy, Debug)]
struct Slot {
    /// The atlas's identity (a new one comes with every resource reload).
    id: usize,
    atlas: [u32; 2],
    origin: [u32; 2],
    size: [u32; 2],
    white: Rect,
}

/// A piece of the canvas.
#[derive(Clone, Debug, PartialEq, Eq, Hash)]
enum Key {
    Glyph(Face, u16, u16),
    /// A quarter disc (`false`) or a one-pixel quarter ring of this radius in pixels.
    Corner(u16, bool),
    Picture(Picture, u32, u32),
}

#[derive(Clone, Copy, Debug)]
struct Placed {
    x: u32,
    y: u32,
    w: u32,
    h: u32,
    /// A glyph's bitmap offset from the pen.
    left: f32,
    top: f32,
}

#[derive(Default)]
pub struct Canvas {
    slot: Option<Slot>,
    shelf: (u32, u32, u32),
    placed: HashMap<Key, Option<Placed>>,
    /// The slot filled up this frame: start over at the next.
    full: bool,
    /// Pictures downloaded from Rustic's CDN, decoded, by URL.
    pub remote: HashMap<String, Arc<Rgba>>,
    /// The slider track drawn last (its handle fills it up to itself).
    pub last_track: Option<[f32; 4]>,
    /// The open clip: the batch's vertex count at its start, and its rectangle.
    clip: Option<(usize, [f32; 4])>,
}

static CANVAS: LazyLock<Mutex<Canvas>> = LazyLock::new(|| Mutex::new(Canvas::default()));

/// The canvas (never held across a game call that may draw widgets).
pub fn canvas() -> MutexGuard<'static, Canvas> {
    CANVAS.lock().unwrap_or_else(|e| e.into_inner())
}

impl Canvas {
    /// Once a frame, with the atlas the frame draws from.
    pub fn begin(&mut self, atlas: &HudAtlas) {
        let slot = atlas
            .sprites
            .get(horizon_client::hud::MOD_IMAGE_SPRITE)
            .map(|r| Slot {
                id: std::ptr::from_ref(atlas) as usize,
                atlas: [atlas.width, atlas.height],
                origin: [
                    (r.u0 * atlas.width as f32).round() as u32,
                    (r.v0 * atlas.height as f32).round() as u32,
                ],
                size: [r.w, r.h],
                white: atlas.font.white(),
            });
        let key = |s: &Option<Slot>| s.map(|s| (s.id, s.atlas, s.origin, s.size));
        if key(&slot) != key(&self.slot) || self.full {
            self.placed.clear();
            self.shelf = (0, 0, 0);
            self.full = false;
        }
        self.slot = slot;
        self.last_track = None;
        self.clip = None;
    }

    /// A new atlas came with the resources: everything goes up again.
    pub fn forget(&mut self) {
        self.slot = None;
        self.placed.clear();
        self.shelf = (0, 0, 0);
    }

    /// Whether the canvas has a slot to draw from.
    pub fn ready(&self) -> bool {
        self.slot.is_some()
    }

    /// Room for `w` x `h` pixels in the slot, a pixel apart from the rest.
    fn room(&mut self, w: u32, h: u32) -> Option<[u32; 2]> {
        let slot = self.slot?;
        let (mut x, mut y, mut row) = self.shelf;
        if w > slot.size[0] || h > slot.size[1] {
            return None;
        }
        if x + w > slot.size[0] {
            x = 0;
            y += row + 1;
            row = 0;
        }
        if y + h > slot.size[1] {
            self.full = true;
            return None;
        }
        self.shelf = (x + w + 1, y, row.max(h));
        Some([x, y])
    }

    /// Puts `rgba` (`w` x `h`) in the slot, uploaded with `batch`.
    fn upload(&mut self, batch: &mut HudBatch, w: u32, h: u32, rgba: Vec<u8>) -> Option<[u32; 2]> {
        let at = self.room(w, h)?;
        let slot = self.slot?;
        batch.patches.push(Patch {
            x: slot.origin[0] + at[0],
            y: slot.origin[1] + at[1],
            width: w,
            height: h,
            rgba,
        });
        Some(at)
    }

    /// The atlas rectangle of a placed piece, its corners swapped to flip it.
    fn uv(&self, p: &Placed, flip_x: bool, flip_y: bool) -> Option<Rect> {
        let slot = self.slot?;
        let (aw, ah) = (slot.atlas[0] as f32, slot.atlas[1] as f32);
        let (x0, y0) = (
            (slot.origin[0] + p.x) as f32 / aw,
            (slot.origin[1] + p.y) as f32 / ah,
        );
        let (x1, y1) = (
            (slot.origin[0] + p.x + p.w) as f32 / aw,
            (slot.origin[1] + p.y + p.h) as f32 / ah,
        );
        let (u0, u1) = if flip_x { (x1, x0) } else { (x0, x1) };
        let (v0, v1) = if flip_y { (y1, y0) } else { (y0, y1) };
        Some(Rect {
            u0,
            v0,
            u1,
            v1,
            w: p.w,
            h: p.h,
        })
    }

    /// A piece, rasterised and uploaded the first time it is asked for.
    fn piece(&mut self, batch: &mut HudBatch, key: &Key) -> Option<Placed> {
        if let Some(p) = self.placed.get(key) {
            return *p;
        }
        if self.full {
            return None;
        }
        let made = match key {
            Key::Glyph(face, id, quarter) => {
                let font = face_font(*face)?;
                let px = f32::from(*quarter) / 4.0;
                let glyph = GlyphId(*id)
                    .with_scale_and_position(face_scale(*face, font, px), point(0.0, 0.0));
                match font.outline_glyph(glyph) {
                    Some(outline) => {
                        let b = outline.px_bounds();
                        let (w, h) = (b.width() as u32, b.height() as u32);
                        let mut alpha = vec![0f32; (w * h) as usize];
                        outline.draw(|x, y, c| {
                            if x < w && y < h {
                                alpha[(y * w + x) as usize] = c;
                            }
                        });
                        let rgba = alpha
                            .iter()
                            // Light text on dark plates reads thin: a gentle boost.
                            .flat_map(|a| [255, 255, 255, (a.powf(0.85) * 255.0).round() as u8])
                            .collect();
                        let at = self.upload(batch, w, h, rgba);
                        at.map(|[x, y]| Placed {
                            x,
                            y,
                            w,
                            h,
                            left: b.min.x,
                            top: b.min.y,
                        })
                    }
                    // A space: nothing to draw, never to ask again.
                    None => {
                        self.placed.insert(key.clone(), None);
                        return None;
                    }
                }
            }
            Key::Corner(r, ring) => {
                let n = u32::from(*r);
                let rgba = corner_mask(n, *ring);
                self.upload(batch, n, n, rgba).map(|[x, y]| Placed {
                    x,
                    y,
                    w: n,
                    h: n,
                    left: 0.0,
                    top: 0.0,
                })
            }
            Key::Picture(picture, w, h) => {
                let im = self.render_picture(picture, *w, *h)?;
                let (w, h) = (im.width, im.height);
                self.upload(batch, w, h, im.pixels).map(|[x, y]| Placed {
                    x,
                    y,
                    w,
                    h,
                    left: 0.0,
                    top: 0.0,
                })
            }
        };
        if made.is_some() {
            self.placed.insert(key.clone(), made);
        }
        made
    }

    fn render_picture(&self, picture: &Picture, w: u32, h: u32) -> Option<Rgba> {
        let side = Glyph::SIDE;
        let glyph = |g: Glyph| {
            ICONS
                .as_ref()
                .map(|s| s.crop([g.index() * side, 0, side, side]))
        };
        match picture {
            Picture::Title(Image::Logo) => BANNER.as_ref().map(|l| title::resample(l, w, h)),
            Picture::Title(Image::Icon(g, rgb)) => {
                glyph(*g).map(|im| title::resample(&im, w, h).tinted(*rgb))
            }
            Picture::Title(Image::Tile { hue, glyph: g }) => {
                glyph(*g).map(|im| title::tile(*hue, w, w as f32 * 0.14, &im))
            }
            Picture::Flag(code) => Some(rusticmc_companion_core::flags::render(
                code,
                w,
                h,
                (h as f32 * 0.14).max(1.0),
            )),
            Picture::Remote(url) => self.remote.get(url).map(|im| title::resample(im, w, h)),
        }
    }

    fn scale(batch: &HudBatch) -> f32 {
        batch.scale.max(1.0)
    }

    /// A rectangle in screen pixels, in `color` (ABGR).
    fn rect_px(&self, batch: &mut HudBatch, [x, y, w, h]: [i32; 4], color: u32) {
        let Some(slot) = self.slot else {
            return;
        };
        if w <= 0 || h <= 0 {
            return;
        }
        let s = Self::scale(batch);
        let mut r = slot.white;
        let (cu, cv) = ((r.u0 + r.u1) / 2.0, (r.v0 + r.v1) / 2.0);
        (r.u0, r.u1, r.v0, r.v1) = (cu, cu, cv, cv);
        batch.rect_uv(
            x as f32 / s,
            y as f32 / s,
            w as f32 / s,
            h as f32 / s,
            r,
            color,
        );
    }

    /// A corner mask `r` pixels square at `(x, y)`, flipped to the corner it is.
    #[allow(clippy::too_many_arguments)]
    fn corner_px(
        &mut self,
        batch: &mut HudBatch,
        x: i32,
        y: i32,
        r: i32,
        ring: bool,
        flip: (bool, bool),
        color: u32,
    ) {
        if r <= 0 {
            return;
        }
        let Some(p) = self.piece(batch, &Key::Corner(r as u16, ring)) else {
            return;
        };
        let Some(uv) = self.uv(&p, flip.0, flip.1) else {
            return;
        };
        let s = Self::scale(batch);
        batch.rect_uv(
            x as f32 / s,
            y as f32 / s,
            r as f32 / s,
            r as f32 / s,
            uv,
            color,
        );
    }

    /// A box with round corners of `r` pixels, filled in `fill`, `border` around it a pixel
    /// thick (ABGR; alpha 0 for none), all in screen pixels, no pixel covered twice.
    #[allow(clippy::too_many_arguments)]
    fn round_px(
        &mut self,
        batch: &mut HudBatch,
        [x, y, w, h]: [i32; 4],
        r: i32,
        fill: u32,
        border: u32,
    ) {
        if w <= 0 || h <= 0 {
            return;
        }
        let r = r.min(w / 2).min(h / 2).max(0);
        let has_border = border >> 24 != 0;
        // The fill's own box and radius.
        let (fx, fy, fw, fh, fr) = if has_border {
            (x + 1, y + 1, w - 2, h - 2, (r - 1).max(0))
        } else {
            (x, y, w, h, r)
        };
        if fill >> 24 != 0 && fw > 0 && fh > 0 {
            if fr == 0 {
                self.rect_px(batch, [fx, fy, fw, fh], fill);
            } else {
                self.rect_px(batch, [fx + fr, fy, fw - 2 * fr, fh], fill);
                self.rect_px(batch, [fx, fy + fr, fr, fh - 2 * fr], fill);
                self.rect_px(batch, [fx + fw - fr, fy + fr, fr, fh - 2 * fr], fill);
                for (cx, cy, flip) in [
                    (fx, fy, (false, false)),
                    (fx + fw - fr, fy, (true, false)),
                    (fx, fy + fh - fr, (false, true)),
                    (fx + fw - fr, fy + fh - fr, (true, true)),
                ] {
                    self.corner_px(batch, cx, cy, fr, false, flip, fill);
                }
            }
        }
        if has_border {
            if r <= 1 {
                self.rect_px(batch, [x, y, w, 1], border);
                self.rect_px(batch, [x, y + h - 1, w, 1], border);
                self.rect_px(batch, [x, y + 1, 1, h - 2], border);
                self.rect_px(batch, [x + w - 1, y + 1, 1, h - 2], border);
            } else {
                self.rect_px(batch, [x + r, y, w - 2 * r, 1], border);
                self.rect_px(batch, [x + r, y + h - 1, w - 2 * r, 1], border);
                self.rect_px(batch, [x, y + r, 1, h - 2 * r], border);
                self.rect_px(batch, [x + w - 1, y + r, 1, h - 2 * r], border);
                for (cx, cy, flip) in [
                    (x, y, (false, false)),
                    (x + w - r, y, (true, false)),
                    (x, y + h - r, (false, true)),
                    (x + w - r, y + h - r, (true, true)),
                ] {
                    self.corner_px(batch, cx, cy, r, true, flip, border);
                }
            }
        }
    }

    /// GUI pixels to whole screen pixels.
    fn px_rect(batch: &HudBatch, [x, y, w, h]: [f32; 4]) -> [i32; 4] {
        let s = Self::scale(batch);
        let (x0, y0) = ((x * s).round() as i32, (y * s).round() as i32);
        let (x1, y1) = (((x + w) * s).round() as i32, ((y + h) * s).round() as i32);
        [x0, y0, x1 - x0, y1 - y0]
    }

    /// Whether `r` (GUI pixels) shows inside the open clip.
    fn visible(&self, r: [f32; 4]) -> bool {
        match self.clip {
            Some((_, c)) => {
                r[0] < c[0] + c[2] && r[0] + r[2] > c[0] && r[1] < c[1] + c[3] && r[1] + r[3] > c[1]
            }
            None => true,
        }
    }

    pub fn fill(&mut self, batch: &mut HudBatch, rect: [f32; 4], argb: u32) {
        if !self.visible(rect) {
            return;
        }
        let r = Self::px_rect(batch, rect);
        self.rect_px(batch, r, abgr(argb));
    }

    pub fn round(
        &mut self,
        batch: &mut HudBatch,
        rect: [f32; 4],
        radius: f32,
        fill: u32,
        border: u32,
    ) {
        if !self.visible(rect) {
            return;
        }
        let r = Self::px_rect(batch, rect);
        let radius = (radius * Self::scale(batch)).round() as i32;
        self.round_px(batch, r, radius, abgr(fill), abgr(border));
    }

    /// A rectangle shading from `left` to `right` (ARGB), left to right.
    pub fn gradient(&mut self, batch: &mut HudBatch, rect: [f32; 4], left: u32, right: u32) {
        if !self.visible(rect) {
            return;
        }
        let [x, y, w, h] = Self::px_rect(batch, rect);
        let start = batch.vertices.len();
        self.rect_px(batch, [x, y, w, h], abgr(left));
        // Two triangles of corners 0 1 2 and 0 2 3 (top left, top right, bottom right, bottom
        // left): the right corners are the 2nd, 3rd and 5th vertices.
        if batch.vertices.len() == start + 6 {
            for i in [1, 2, 4] {
                batch.vertices[start + i].color = abgr(right);
            }
        }
    }

    pub fn quad(&mut self, batch: &mut HudBatch, points: [[f32; 2]; 4], argb: u32) {
        let Some(slot) = self.slot else {
            return;
        };
        let w = slot.white;
        let uv = [(w.u0 + w.u1) / 2.0, (w.v0 + w.v1) / 2.0];
        batch.quad(points, [uv; 4], abgr(argb));
    }

    /// A line of text whose em is `px` screen pixels, its baseline at `baseline` (screen pixels
    /// from the top) and its pen starting at `x` (screen pixels); the pen's end.
    #[allow(clippy::too_many_arguments)]
    pub fn text_px(
        &mut self,
        batch: &mut HudBatch,
        x: f32,
        baseline: f32,
        text: &str,
        px: f32,
        weight: Weight,
        color: u32,
    ) -> f32 {
        if font(weight).is_none() {
            return x;
        }
        let quarter = (px * 4.0).round().clamp(1.0, 4000.0) as u16;
        let s = Self::scale(batch);
        let baseline = baseline.round();
        let (glyphs, width) = layout(text, px, weight);
        for g in glyphs {
            let key = Key::Glyph(g.face, g.id.0, quarter);
            if let Some(p) = self.piece(batch, &key)
                && let Some(uv) = self.uv(&p, false, false)
            {
                let gx = (x + g.pen + g.dx).round() + p.left;
                let gy = (baseline + g.dy).round() + p.top;
                batch.rect_uv(gx / s, gy / s, p.w as f32 / s, p.h as f32 / s, uv, color);
            }
        }
        x + width
    }

    /// The ascent of a line of `px` screen pixels (its baseline from its box's top).
    pub fn ascent_px(px: f32, weight: Weight) -> f32 {
        font(weight).map_or(px * title::ASCENT, |f| f.as_scaled(scale_of(px)).ascent())
    }

    /// `picture` over `rect` (GUI pixels), multiplied by `tint` (ARGB).
    pub fn picture(&mut self, batch: &mut HudBatch, picture: &Picture, rect: [f32; 4], tint: u32) {
        if !self.visible(rect) {
            return;
        }
        let [x, y, w, h] = Self::px_rect(batch, rect);
        if w <= 0 || h <= 0 {
            return;
        }
        // Big pictures (a server's banner) are kept at most this large and stretched.
        let (max_w, max_h) = (640, 256);
        let k = (w as f32 / max_w as f32)
            .max(h as f32 / max_h as f32)
            .max(1.0);
        let (pw, ph) = (
            ((w as f32 / k).round() as u32).max(1),
            ((h as f32 / k).round() as u32).max(1),
        );
        let key = Key::Picture(picture.clone(), pw, ph);
        let Some(p) = self.piece(batch, &key) else {
            return;
        };
        let Some(uv) = self.uv(&p, false, false) else {
            return;
        };
        let s = Self::scale(batch);
        batch.rect_uv(
            x as f32 / s,
            y as f32 / s,
            w as f32 / s,
            h as f32 / s,
            uv,
            abgr(tint),
        );
    }

    /// A line of text, its box's top at `y`, its em `size` (GUI pixels); its width (GUI).
    #[allow(clippy::too_many_arguments)]
    pub fn text(
        &mut self,
        batch: &mut HudBatch,
        x: f32,
        y: f32,
        text: &str,
        size: f32,
        weight: Weight,
        argb: u32,
        align: Align,
    ) -> f32 {
        let s = Self::scale(batch);
        let px = (size * s).round().max(4.0);
        let width = measure_px(text, px, weight);
        let line_h = px * title::LINE;
        if !self.visible([x - width / s, y, width * 2.0 / s + 1.0, line_h / s]) {
            return width / s;
        }
        let x_px = match align {
            Align::Left => x * s,
            Align::Center => x * s - width / 2.0,
            Align::Right => x * s - width,
        };
        let baseline = y * s + Self::ascent_px(px, weight);
        self.text_px(batch, x_px.round(), baseline, text, px, weight, abgr(argb));
        width / s
    }

    /// Paints `paints` (a screen's or a widget's).
    pub fn paint_all(&mut self, batch: &mut HudBatch, paints: &[Paint]) {
        for p in paints {
            self.paint(batch, p);
        }
        // An unclosed clip ends with the paints.
        self.clip_end(batch);
    }

    pub fn paint(&mut self, batch: &mut HudBatch, p: &Paint) {
        match p {
            Paint::Fill { rect, argb } => self.fill(batch, *rect, *argb),
            Paint::Round {
                rect,
                radius,
                fill,
                border,
            } => self.round(batch, *rect, *radius, *fill, *border),
            Paint::Quad { points, argb } => self.quad(batch, *points, *argb),
            Paint::Gradient { rect, left, right } => self.gradient(batch, *rect, *left, *right),
            Paint::Text {
                x,
                y,
                text,
                size,
                weight,
                argb,
                align,
            } => {
                self.text(batch, *x, *y, text, *size, *weight, *argb, *align);
            }
            Paint::Picture {
                picture,
                rect,
                tint,
            } => self.picture(batch, picture, *rect, *tint),
            Paint::ClipBegin(r) => {
                self.clip_end(batch);
                self.clip = Some((batch.vertices.len(), *r));
            }
            Paint::ClipEnd => self.clip_end(batch),
        }
    }

    fn clip_end(&mut self, batch: &mut HudBatch) {
        if let Some((start, [x, y, w, h])) = self.clip.take()
            && batch.vertices.len() > start
        {
            batch.clip_since(start, x, y, x + w, y + h);
        }
    }
}

/// The top-left quarter of a circle of radius `n` centred on `(n, n)`, 4x4 supersampled, white:
/// the disc, or with `ring` the one-pixel ring at its edge.
fn corner_mask(n: u32, ring: bool) -> Vec<u8> {
    let r = n as f32;
    let cover = |x: u32, y: u32, radius: f32| -> f32 {
        if radius <= 0.0 {
            return 0.0;
        }
        let mut inside = 0;
        for sy in 0..4 {
            for sx in 0..4 {
                let px = x as f32 + (sx as f32 + 0.5) / 4.0;
                let py = y as f32 + (sy as f32 + 0.5) / 4.0;
                let (dx, dy) = (r - px, r - py);
                if dx * dx + dy * dy <= radius * radius {
                    inside += 1;
                }
            }
        }
        inside as f32 / 16.0
    };
    let mut out = Vec::with_capacity((n * n * 4) as usize);
    for y in 0..n {
        for x in 0..n {
            let a = if ring {
                cover(x, y, r) - cover(x, y, r - 1.0)
            } else {
                cover(x, y, r)
            };
            out.extend_from_slice(&[255, 255, 255, (a.clamp(0.0, 1.0) * 255.0).round() as u8]);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The game's language names in the scripts Inter lacks: every glyph is the font's own (no
    /// missing-glyph box) where the computer has a font for the script; a script with none here
    /// is skipped (said on the output).
    #[test]
    fn every_script_of_the_language_names_has_its_glyphs() {
        let names = [
            "Հայերեն",
            "ქართული",
            "עברית",
            "ייִדיש",
            "العربية",
            "فارسی",
            "हिन्दी",
            "বাংলা",
            "தமிழ்",
            "తెలుగు",
            "ಕನ್ನಡ",
            "ລາວ",
            "بهاس ملايو",
            "ᚦᛖ ᛖᚾᚷᛚᛁᛋᚳ",
            "ไทย",
            "日本語",
            "한국어",
            "Русский",
            "Ελληνικά",
            "Tiếng Việt",
        ];
        for name in names {
            let mut missing = false;
            for c in name.chars().filter(|c| !c.is_whitespace()) {
                let face = face_of(c, Weight::Medium, &mut None);
                let Some(f) = face_font(face) else {
                    missing = true;
                    continue;
                };
                if f.glyph_id(c).0 == 0 {
                    missing = true;
                }
            }
            if missing {
                let found = name
                    .chars()
                    .find_map(Script::of)
                    .and_then(script_font)
                    .is_some();
                assert!(
                    !found,
                    "{name}: a glyph falls back to the missing-glyph box"
                );
                println!("no system font for {name} here: skipped");
                continue;
            }
            // Shaped and laid out: every glyph a real one, the line some width.
            let (glyphs, width) = layout(name, 24.0, Weight::Medium);
            assert!(width > 0.0, "{name}");
            assert!(
                glyphs.iter().all(|g| g.id.0 != 0),
                "{name}: .notdef after shaping"
            );
        }
    }

    #[test]
    fn right_to_left_names_are_drawn_from_the_right() {
        if script_font(Script::Hebrew).is_none() {
            return;
        }
        // In visual order, the first letter (alef is not here: ע) is drawn last, at the right.
        let (glyphs, _) = layout("עב", 24.0, Weight::Medium);
        assert_eq!(glyphs.len(), 2);
        let f = face_font(glyphs[0].face).expect("font");
        assert_eq!(glyphs[1].id, f.glyph_id('ע'));
        assert!(glyphs[1].pen > glyphs[0].pen);
    }

    #[test]
    fn the_bundled_images_and_fonts_load() {
        let logo = LOGO.as_ref().expect("the wordmark decodes");
        assert_eq!(logo.height, 256);
        let banner = BANNER.as_ref().expect("the banner decodes");
        assert!(banner.width > banner.height * 3);
        let icons = ICONS.as_ref().expect("the icons decode");
        assert_eq!(
            (icons.width, icons.height),
            (Glyph::SIDE * Glyph::COUNT, Glyph::SIDE)
        );
        assert!(MEDIUM.is_some() && SEMIBOLD.is_some());
        let w = measure_px("abcdefghij", 20.0, Weight::Medium);
        assert!((90.0..130.0).contains(&w), "{w}");
        // Cyrillic and Turkish are Inter's own; Han, kana and Hangul look for a system font.
        let cyrillic = "Привет, Україна";
        assert!(
            cyrillic
                .chars()
                .all(|c| MEDIUM.as_ref().unwrap().glyph_id(c).0 != 0 || c == ' ')
        );
        assert!("ıİşğ".chars().all(|c| !is_cjk(c)));
        assert!("設定サーバー설정设置".chars().all(is_cjk));
    }

    #[test]
    fn each_language_looks_in_its_own_region_first() {
        assert_eq!(Region::order("ja_jp")[0], Region::Japanese);
        assert_eq!(Region::order("ko_kr")[0], Region::Korean);
        assert_eq!(Region::order("zh_hk")[0], Region::Traditional);
        assert_eq!(Region::order("zh_cn")[0], Region::Simplified);
        assert_eq!(Region::order("en_us")[0], Region::Simplified);
    }

    #[test]
    fn a_system_font_draws_what_inter_lacks_where_the_computer_has_one() {
        // Windows always ships these fonts; elsewhere the test only runs when one is installed.
        let windows = std::env::var_os("WINDIR").is_some();
        for (region, text) in [
            (Region::Simplified, "设置服务器"),
            (Region::Traditional, "設定伺服器"),
            (Region::Japanese, "設定サーバー"),
            (Region::Korean, "설정 서버"),
        ] {
            let Some(f) = region.font() else {
                assert!(!windows, "{region:?}: no system font on Windows");
                continue;
            };
            for c in text.chars().filter(|c| *c != ' ') {
                assert_ne!(f.glyph_id(c).0, 0, "{region:?} {c}");
            }
        }
        // Mixed text: Inter for the Latin letters, the system font for the rest.
        let mut regions = Some(Region::order("ja_jp"));
        assert_eq!(
            face_of('R', Weight::Medium, &mut regions),
            Face::Inter(Weight::Medium)
        );
        if windows {
            assert_eq!(
                face_of('設', Weight::Medium, &mut regions),
                Face::Cjk(Region::Japanese)
            );
            assert!(
                measure_px("Rustic 設定", 20.0, Weight::Medium)
                    > measure_px("Rustic", 20.0, Weight::Medium) + 30.0
            );
        }
    }

    /// A line laid out and rasterised as the canvas draws it (white on black), for the tests and
    /// to look at (`RUSTIC_TEXT_PNG=<file>` writes it).
    fn render_line(text: &str, px: f32) -> Rgba {
        let (glyphs, width) = layout(text, px, Weight::Medium);
        let (w, h) = (width.ceil() as u32 + 8, (px * 2.4).ceil() as u32);
        let baseline = (px * 1.5).round();
        let mut pixels: Vec<u8> = [0, 0, 0, 255].repeat((w * h) as usize);
        for g in glyphs {
            let Some(f) = face_font(g.face) else { continue };
            let glyph =
                g.id.with_scale_and_position(face_scale(g.face, f, px), point(0.0, 0.0));
            let Some(outline) = f.outline_glyph(glyph) else {
                continue;
            };
            let b = outline.px_bounds();
            let ox = (4.0 + g.pen + g.dx).round() + b.min.x;
            let oy = (baseline + g.dy).round() + b.min.y;
            outline.draw(|x, y, c| {
                let (px_, py) = (ox as i64 + i64::from(x), oy as i64 + i64::from(y));
                if px_ >= 0 && py >= 0 && (px_ as u32) < w && (py as u32) < h {
                    let i = ((py as u32 * w + px_ as u32) * 4) as usize;
                    let v = pixels[i].saturating_add((c * 255.0) as u8);
                    pixels[i..i + 3].copy_from_slice(&[v, v, v]);
                }
            });
        }
        Rgba {
            width: w,
            height: h,
            pixels,
        }
    }

    #[test]
    fn thai_reads_a_system_font_with_its_marks_off_each_other() {
        assert!("ภาษาไทย ที่ น้ำ".chars().filter(|c| *c != ' ').all(is_thai));
        assert!(!"Rustic 設定".chars().any(is_thai));
        let windows = std::env::var_os("WINDIR").is_some();
        let Some(f) = THAI.as_ref() else {
            assert!(!windows, "no Thai system font on Windows");
            return;
        };
        // Every character of the in-game Thai table has a glyph.
        let table = include_str!("../../core/i18n/th.json");
        for c in table.chars().filter(|c| is_thai(*c)) {
            assert_ne!(f.glyph_id(c).0, 0, "{c}");
        }
        let mut regions = None;
        assert_eq!(face_of('ท', Weight::Medium, &mut regions), Face::Thai);
        assert_eq!(
            face_of('R', Weight::Medium, &mut regions),
            Face::Inter(Weight::Medium)
        );
        let marks = |text: &str| -> Vec<(f32, f32)> {
            layout(text, 24.0, Weight::Medium)
                .0
                .iter()
                .map(|g| (g.dx, g.dy))
                .collect()
        };
        // A tone mark alone stays; over an upper vowel or before sara am it goes up.
        assert_eq!(marks("ก่")[1], (0.0, 0.0));
        assert!(marks("ที่")[2].1 < -2.0, "{:?}", marks("ที่"));
        assert!(marks("น้ำ")[1].1 < -2.0, "{:?}", marks("น้ำ"));
        // Over a tall stem the marks move left; under ฎ a lower vowel moves down.
        assert!(marks("ปั้น")[1].0 < -1.0 && marks("ปั้น")[2].0 < -1.0);
        assert_eq!(marks("กั้น")[1].0, 0.0);
        assert!(marks("ฎุ")[1].1 > 1.0);
        // Thai text has the width of its letters (marks take none) and the em of Inter's.
        let w = measure_px("ภาษาไทย", 20.0, Weight::Medium);
        assert!((40.0..140.0).contains(&w), "{w}");
        let line = render_line("กำลังโหลดเซิร์ฟเวอร์… ที่นี่ ปั้น ฝั่ง น้ำ ฎุ เพิ่มลงในรายการ Rustic", 40.0);
        assert!(line.pixels.chunks(4).any(|p| p[0] > 200));
        if let Some(path) = std::env::var_os("RUSTIC_TEXT_PNG") {
            let file = std::fs::File::create(path).expect("png file");
            let mut enc = png::Encoder::new(file, line.width, line.height);
            enc.set_color(png::ColorType::Rgba);
            enc.set_depth(png::BitDepth::Eight);
            let mut w = enc.write_header().expect("png header");
            w.write_image_data(&line.pixels).expect("png data");
        }
    }

    #[test]
    fn corner_masks_are_full_inside_and_rings_their_edge() {
        let disc = corner_mask(8, false);
        let ring = corner_mask(8, true);
        let a = |m: &[u8], x: u32, y: u32| m[((y * 8 + x) * 4 + 3) as usize];
        // The bottom right of the top-left quarter is inside the disc, the top left outside.
        assert_eq!(a(&disc, 7, 7), 255);
        assert_eq!(a(&disc, 0, 0), 0);
        assert_eq!(a(&ring, 7, 7), 0);
        assert!(a(&ring, 7, 0) > 200);
        // The ring is the disc's edge only: never more than the disc.
        for i in (3..disc.len()).step_by(4) {
            assert!(ring[i] <= disc[i]);
        }
    }

    #[test]
    fn colours_turn_into_the_huds_order() {
        assert_eq!(abgr(0xFF11_2233), 0xFF33_2211);
        assert_eq!(abgr(0x8000_00FF), 0x80FF_0000);
    }
}

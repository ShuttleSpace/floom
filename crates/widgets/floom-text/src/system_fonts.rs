// SPDX-License-Identifier: MIT
//! Fonts the platform ships where the font backend does not look.
//!
//! fontique's Apple backend builds its collection from
//! `NSSearchPathForDirectoriesInDomains(LibraryDirectory)` — `/Library/Fonts` and
//! `~/Library/Fonts` — and never from `/System/Library/Fonts`, which is where the
//! system faces actually live. On a Mac that usually goes unnoticed: a
//! developer machine tends to have a CJK face in `/Library/Fonts` (this one has
//! `Arial Unicode.ttf`). An iPhone or iPad has nothing of the sort: its Han
//! coverage is `PingFang.ttc` / `Songti.ttc` under `/System/Library/Fonts`, so
//! the default stack in [`crate::layout`] finds no face for `你好世界` and the
//! shaper emits `.notdef` boxes while Latin text looks perfectly fine.
//!
//! So we read those files ourselves and register them. Only the CJK-capable
//! stems are read: the full system directory is hundreds of megabytes, and the
//! default stack by name only ever asks for a handful of families.

use parley::FontContext;
#[cfg(target_vendor = "apple")]
use parley::fontique::Blob;

/// System font directories the backend skips, most specific first.
#[cfg(target_vendor = "apple")]
const SYSTEM_DIRECTORIES: &[&str] = &[
    "/System/Library/Fonts",
    "/System/Library/Fonts/Supplemental",
];

/// File stems (without extension) that carry Han / Kana / Hangul coverage.
///
/// Matched case-insensitively as a *prefix* of the file name: the shipped names
/// carry weights and styles after the family (`STHeiti Medium.ttc`).
#[cfg(target_vendor = "apple")]
const CJK_STEMS: &[&str] = &[
    "PingFang",
    "Hiragino Sans",
    "Hiragino Mincho",
    "STHeiti",
    "STSong",
    "STKaiti",
    "STFangsong",
    "Songti",
    "Kaiti",
    "Yuanti",
    "Heiti",
    "Apple SD Gothic Neo",
    "AppleMyungjo",
    "NotoSansCJK",
    "NotoSerifCJK",
    "Arial Unicode",
];

/// Registers the platform's CJK faces into `cx` once.
pub(crate) fn register(cx: &mut FontContext) {
    #[cfg(target_vendor = "apple")]
    {
        for path in cjk_font_paths() {
            match std::fs::read(&path) {
                // A file that fails to parse is skipped silently: a broken face
                // is not a reason to refuse to start the text stack.
                Ok(bytes) => {
                    cx.collection.register_fonts(Blob::from(bytes), None);
                }
                // A face we cannot read is reported once and skipped: the rest of
                // the stack still has to come up.
                Err(error) => eprintln!(
                    "floom-text: the system font {} could not be read: {error}",
                    path.display()
                ),
            }
        }
    }
    #[cfg(not(target_vendor = "apple"))]
    {
        let _ = cx;
    }
}

/// Every system font file whose stem matches [`CJK_STEMS`].
#[cfg(target_vendor = "apple")]
fn cjk_font_paths() -> Vec<std::path::PathBuf> {
    let mut paths = Vec::new();
    for directory in SYSTEM_DIRECTORIES {
        let Ok(entries) = std::fs::read_dir(directory) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            let Some(name) = path.file_name().and_then(|name| name.to_str()) else {
                continue;
            };
            let extension = path
                .extension()
                .and_then(|extension| extension.to_str())
                .unwrap_or_default();
            if !matches!(
                extension.to_ascii_lowercase().as_str(),
                "ttc" | "ttf" | "otf"
            ) {
                continue;
            }
            if CJK_STEMS.iter().any(|stem| {
                name.to_ascii_lowercase()
                    .starts_with(&stem.to_ascii_lowercase())
            }) {
                paths.push(path);
            }
        }
    }
    paths
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The gap this module exists for: the files are on disk under
    /// `/System/Library/Fonts`, and the backend's own scan never touches them.
    #[cfg(target_vendor = "apple")]
    #[test]
    fn the_system_directory_holds_faces_the_backend_cannot_see() {
        let paths = cjk_font_paths();
        assert!(
            !paths.is_empty(),
            "/System/Library/Fonts listed no CJK face — the directory moved?"
        );
        let mut cx = FontContext::new();
        let before = cx.collection.family_names().count();
        register(&mut cx);
        let after = cx.collection.family_names().count();
        assert!(
            after > before,
            "registering {paths:?} added no family (before={before}, after={after})"
        );
    }

    /// The acceptance condition, end to end: Han text shapes to real glyphs.
    /// A `.notdef` here means the stack still cannot reach a CJK face.
    #[cfg(target_vendor = "apple")]
    #[test]
    fn han_text_shapes_to_real_glyphs() {
        use crate::{TextLayoutOptions, TextShaper};
        let mut shaper = TextShaper::new();
        let out = shaper.shape("你好世界", &TextLayoutOptions::default());
        let notdef = out
            .runs
            .iter()
            .flat_map(|run| run.glyphs.iter())
            .filter(|glyph| glyph.id == 0)
            .count();
        assert_eq!(notdef, 0, "Han text shaped to .notdef boxes: {out:?}");
    }

    /// Registration has to be harmless where there is nothing to read.
    #[test]
    fn registering_is_safe_on_every_target() {
        let mut cx = FontContext::new();
        register(&mut cx);
    }
}

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
//! So on the devices we read one of those files ourselves and register it.
//!
//! **One**, because a registered face costs its own size in resident memory:
//! `Blob` owns the bytes, and the system's CJK files are tens of megabytes each
//! (`Songti.ttc` alone is 67 MB). Registering the whole list cost a Mac 236 MB
//! of RSS for text that already worked, which is the opposite of what this
//! library is for.
//!
//! **Not on macOS**, because the backend's own paths do reach a CJK face there
//! (`/Library/Fonts/Arial Unicode.ttf` on a stock machine), so macOS rendered
//! Han text before this module existed. Only the platforms that ship their fonts
//! exclusively under `/System/Library/Fonts` need the help.

use parley::FontContext;

/// Whether this target needs the help: Apple, minus macOS.
///
/// A constant rather than a scattered `cfg` so the tests can say what they mean.
#[cfg(target_vendor = "apple")]
const NEEDS_SYSTEM_FONTS: bool = cfg!(not(target_os = "macos"));

#[cfg(any(target_vendor = "apple", target_env = "ohos"))]
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

/// Faces to try on OpenHarmony, most specific first.
///
/// `HarmonyOS_Sans_SC` carries Han coverage; the plain `HarmonyOS_Sans` is the
/// Latin face every image ships.
#[cfg(target_env = "ohos")]
const OHOS_FACES: &[&str] = &[
    "/system/fonts/HarmonyOS_Sans_SC_Regular.ttf",
    "/system/fonts/HarmonyOS_Sans.ttf",
];

/// Records which face was registered, for the hilog line the demo prints.
#[cfg(target_env = "ohos")]
pub(crate) fn note_registered(path: &str) {
    let _ = crate::system_fonts::LAST_FACE.set(Some(path.to_string()));
}

#[cfg(target_env = "ohos")]
pub static LAST_FACE: std::sync::OnceLock<Option<String>> = std::sync::OnceLock::new();

/// Registers one CJK face into `cx`, where the platform needs it.
///
/// The first family in [`CJK_STEMS`] order that is actually on disk wins: that
/// order is the preference order, and stopping at the first hit is the memory
/// budget. A file that cannot be read is reported and skipped — refusing to
/// start the text stack over one face would be worse than the face being absent.
pub(crate) fn register(cx: &mut FontContext) {
    #[cfg(target_vendor = "apple")]
    {
        if NEEDS_SYSTEM_FONTS {
            match first_cjk_font() {
                Some(path) => match std::fs::read(&path) {
                    Ok(bytes) => {
                        cx.collection.register_fonts(Blob::from(bytes), None);
                    }
                    Err(error) => eprintln!(
                        "floom-text: the system font {} could not be read: {error}",
                        path.display()
                    ),
                },
                None => eprintln!(
                    "floom-text: no CJK face under {SYSTEM_DIRECTORIES:?}; Han text will not shape here"
                ),
            }
        }
    }
    #[cfg(target_env = "ohos")]
    {
        // fontique has no OpenHarmony backend, so the collection starts empty and
        // every run measures zero wide: the tree collapses to its padding and the
        // screen shows a dot. The system faces are in /system/fonts.
        let registered = OHOS_FACES
            .iter()
            .find_map(|path| std::fs::read(path).ok().map(|bytes| (*path, bytes)));
        match registered {
            Some((path, bytes)) => {
                cx.collection.register_fonts(Blob::from(bytes), None);
                crate::system_fonts::note_registered(path);
            }
            None => eprintln!(
                "floom-text: none of {OHOS_FACES:?} could be read; text will not shape here"
            ),
        }
    }
    #[cfg(all(not(target_vendor = "apple"), not(target_env = "ohos")))]
    {
        let _ = cx;
    }
}

/// The most-preferred CJK face on disk, if there is one.
#[cfg(target_vendor = "apple")]
fn first_cjk_font() -> Option<std::path::PathBuf> {
    CJK_STEMS.iter().find_map(|stem| {
        cjk_font_paths().into_iter().find(|path| {
            path.file_name()
                .and_then(|name| name.to_str())
                .is_some_and(|name| {
                    name.to_ascii_lowercase()
                        .starts_with(&stem.to_ascii_lowercase())
                })
        })
    })
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
    fn the_system_directory_holds_a_cjk_face_the_backend_cannot_see() {
        let path = first_cjk_font().expect("/System/Library/Fonts listed no CJK face");
        let mut cx = FontContext::new();
        let before = cx.collection.family_names().count();
        cx.collection
            .register_fonts(Blob::from(std::fs::read(&path).expect("readable")), None);
        let after = cx.collection.family_names().count();
        assert!(
            after > before,
            "registering {} added no family (before={before}, after={after})",
            path.display()
        );
    }

    /// One face, on the platforms that need one. Two would double the cost that
    /// made this a problem in the first place.
    #[cfg(target_vendor = "apple")]
    #[test]
    fn only_the_platforms_without_system_fallback_register_a_face() {
        let mut cx = FontContext::new();
        let before = cx.collection.family_names().count();
        register(&mut cx);
        let after = cx.collection.family_names().count();
        if NEEDS_SYSTEM_FONTS {
            assert!(after > before, "the platform needs a face and got none");
        } else {
            assert_eq!(
                after, before,
                "macOS reads from /Library/Fonts; it must not pay for a copy"
            );
        }
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

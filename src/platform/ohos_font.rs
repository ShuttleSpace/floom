#![cfg(target_env = "ohos")]

//! OpenHarmony runtime font discovery.
//!
//! fontique's [`Collection`] discovers system fonts via `fontconfig` on Linux.
//! OpenHarmony has **no** `fontconfig`, so on an OHOS target the collection is
//! empty and text renders with zero glyphs. This module injects bundled fonts
//! into the collection and assigns them as the generic families
//! (sans-serif / serif / monospace) so default text is visible without a system
//! font service.
//!
//! The actual `.ttf` / `.otf` files must be bundled into the HAP (e.g. as
//! `rawfile` resources) and placed at the on-device paths listed in
//! [`BUNDLED_FONT_PATHS`] before app start. Until those files exist this is a
//! no-op — text only renders once the fonts are present. UNVERIFIED on-device.

use std::sync::Arc;

// `fontique` is a transitive dependency surfaced through `parley`, not a direct
// dependency of the `floem` lib crate (see root Cargo.toml). Reference it via
// the re-export so the import resolves on every toolchain.
use parley::fontique::{Blob, Collection, FamilyId, GenericFamily};

/// Bundled font files, read at startup and registered into the collection.
///
/// Populate with the on-device paths where your build places the fonts (e.g. a
/// `rawfile` resource extracted into the app's files directory by an ArkTS
/// bootstrap step). Order determines generic-family assignment: the first
/// font becomes the default sans-serif, the next a serif, then monospace.
pub const BUNDLED_FONT_PATHS: &[&str] = &[
    // "/data/storage/el1/bundle/.../fonts/NotoSansCJK-Regular.ttf",
];

/// Register [`BUNDLED_FONT_PATHS`] into `collection` and set them as the
/// generic families. Call once, immediately after the global `FONT_CONTEXT`
/// is created (see `src/text/layout.rs`).
pub fn register_bundled_fonts(collection: &mut Collection) {
    let mut sans: Option<FamilyId> = None;
    let mut serif: Option<FamilyId> = None;
    let mut mono: Option<FamilyId> = None;

    for path in BUNDLED_FONT_PATHS {
        let Ok(bytes) = std::fs::read(path) else {
            eprintln!("[floem] OHOS font: cannot read {path}; skipping");
            continue;
        };
        // `Blob::new` takes `Arc<dyn AsRef<[u8]> + Send + Sync>`; an
        // `Arc<Vec<u8>>` coerces to that (Vec<u8>: AsRef<[u8]> + Send + Sync).
        let blob = Blob::new(Arc::new(bytes));
        // `register_fonts` returns the families this data introduced.
        let families = collection.register_fonts(blob, None);
        for (family_id, _infos) in families {
            if sans.is_none() {
                sans = Some(family_id);
            } else if serif.is_none() {
                serif = Some(family_id);
            } else if mono.is_none() {
                mono = Some(family_id);
            }
        }
    }

    if let Some(id) = sans {
        collection.set_generic_families(GenericFamily::SansSerif, [id].into_iter());
    }
    if let Some(id) = serif {
        collection.set_generic_families(GenericFamily::Serif, [id].into_iter());
    }
    if let Some(id) = mono {
        collection.set_generic_families(GenericFamily::Monospace, [id].into_iter());
    }
}

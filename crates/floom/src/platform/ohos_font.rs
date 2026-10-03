#![cfg(target_env = "ohos")]

//! OpenHarmony runtime font discovery.
//!
//! fontique's [`Collection`] discovers system fonts via `fontconfig` on Linux.
//! OpenHarmony has **no** `fontconfig`, so on an OHOS target the collection is
//! empty and text renders with zero glyphs. This module registers the fonts
//! shipped in the device's `/system/fonts` (the HarmonyOS Sans family) into
//! the collection and assigns them as the generic families
//! (sans-serif / serif / monospace) so default text renders.
//!
//! Font files may also be bundled into the HAP and listed in
//! [`BUNDLED_FONT_PATHS`]; those take precedence over the system scan.

use std::sync::Arc;

// `fontique` is a transitive dependency surfaced through `parley`, not a direct
// dependency of the `floom` lib crate (see root Cargo.toml). Reference it via
// the re-export so the import resolves on every toolchain.
use parley::fontique::{Blob, Collection, FamilyId, GenericFamily};

/// Bundled font files, read at startup and registered into the collection
/// before the system font scan.
///
/// Populate with the on-device paths where your build places the fonts (e.g. a
/// `rawfile` resource extracted into the app's files directory by an ArkTS
/// bootstrap step).
pub const BUNDLED_FONT_PATHS: &[&str] = &[
    // "/data/storage/el1/bundle/.../fonts/NotoSansCJK-Regular.ttf",
];

/// The on-device system font directory scanned for the HarmonyOS Sans family.
const SYSTEM_FONT_DIR: &str = "/system/fonts";

/// File-name prefix selecting the system fonts to register. Registering the
/// full `/system/fonts` set (CJK + emoji + Noto fallbacks) would pin hundreds
/// of megabytes of font data; the HarmonyOS Sans family alone covers the
/// default UI scripts.
const SYSTEM_FONT_PREFIX: &str = "HarmonyOS_Sans";

/// Register the bundled and system fonts into `collection` and set the generic
/// families. Call once, immediately after the global `FONT_CONTEXT` is created
/// (see `crates/floom/src/text/layout.rs`).
pub fn register_bundled_fonts(collection: &mut Collection) {
    // (family id, family name) in registration order.
    let mut families: Vec<(FamilyId, String)> = Vec::new();

    let mut register_bytes = |collection: &mut Collection, bytes: Vec<u8>| {
        // `Blob::new` takes `Arc<dyn AsRef<[u8]> + Send + Sync>`; an
        // `Arc<Vec<u8>>` coerces to that (Vec<u8>: AsRef<[u8]> + Send + Sync).
        let blob = Blob::new(Arc::new(bytes));
        // `register_fonts` returns the families this data introduced.
        let new_families = collection.register_fonts(blob, None);
        for (family_id, _infos) in new_families {
            let name = collection
                .family_name(family_id)
                .unwrap_or_default()
                .to_owned();
            families.push((family_id, name));
        }
    };

    // Explicitly bundled fonts first.
    for path in BUNDLED_FONT_PATHS {
        match std::fs::read(path) {
            Ok(bytes) => register_bytes(collection, bytes),
            Err(err) => eprintln!("[floom] OHOS font: cannot read {path}: {err}; skipping"),
        }
    }

    // Then the system scan.
    let entries = match std::fs::read_dir(SYSTEM_FONT_DIR) {
        Ok(entries) => entries,
        Err(err) => {
            eprintln!("[floom] OHOS font: cannot read {SYSTEM_FONT_DIR}: {err}");
            return;
        }
    };
    let mut paths: Vec<_> = entries
        .filter_map(|entry| entry.ok())
        .map(|entry| entry.path())
        .filter(|path| {
            let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
            let ext = path
                .extension()
                .and_then(|e| e.to_str())
                .unwrap_or("")
                .to_ascii_lowercase();
            name.starts_with(SYSTEM_FONT_PREFIX)
                && matches!(ext.as_str(), "ttf" | "otf" | "ttc")
        })
        .collect();
    paths.sort();
    for path in paths {
        match std::fs::read(&path) {
            Ok(bytes) => register_bytes(collection, bytes),
            Err(err) => eprintln!("[floom] OHOS font: cannot read {}: {err}; skipping", path.display()),
        }
    }

    // Assign generic families: prefer "HarmonyOS Sans" (the family reported by
    // HarmonyOS_Sans.ttf and friends), falling back to the first registered
    // family.
    let pick = |wanted: &str| -> Option<FamilyId> {
        families
            .iter()
            .find(|(_, name)| name.eq_ignore_ascii_case(wanted))
            .map(|(id, _)| *id)
            .or_else(|| families.first().map(|(id, _)| *id))
    };
    if let Some(id) = pick("HarmonyOS Sans") {
        collection.set_generic_families(GenericFamily::SansSerif, [id].into_iter());
        collection.set_generic_families(GenericFamily::SystemUi, [id].into_iter());
    }
    if let Some(id) = pick("HarmonyOS Serif") {
        collection.set_generic_families(GenericFamily::Serif, [id].into_iter());
    }
    if let Some(id) = pick("HarmonyOS Mono") {
        collection.set_generic_families(GenericFamily::Monospace, [id].into_iter());
    }
}

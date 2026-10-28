//! The file arm: a percent-encoded `key<TAB>value` store in the config dir.
//!
//! Linux and Windows have no OS preferences store to ask, so this arm *is*
//! the store — the same decision Flutter's own `shared_preferences_linux` and
//! `_windows` made. The format is deliberately plain:
//!
//! ```text
//! gallery.theme<TAB>dark
//! gallery.zoom<TAB>1.25
//! ```
//!
//! …with keys and values percent-encoded outside `[A-Za-z0-9_.~-]`, so an
//! `=` or a newline or a tab in a value is a byte sequence like any other
//! and the file stays line-oriented, greppable, and hand-editable.
//!
//! # Writes are temp-then-rename
//!
//! A reader must never see a half-written store, and `rename` within a
//! directory is atomic on every filesystem this arm targets — so the write
//! goes to a sibling temp file and is renamed over the store. The temp file
//! is cleaned up on failure.
//!
//! # What this arm does not do
//!
//! No locking: two processes writing one store can interleave their
//! read-modify-writes and lose the last writer's key. Preferences are one
//! app's own settings; the OS stores the other arms reach have the same
//! caveat (Android's `SharedPreferences` is not multi-process-safe either,
//! and says so). A cross-process store is a different capability.

use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

/// The store, parsed: every key to its value, in file order.
///
/// `BTreeMap` because the file's key order is part of what a human sees when
/// they open it — re-writing the store must not shuffle it into an
/// unreadable order.
fn load(path: &Path) -> BTreeMap<String, String> {
    let mut map = BTreeMap::new();
    let Ok(contents) = std::fs::read_to_string(path) else {
        return map; // absent store = empty store
    };
    for line in contents.lines() {
        if line.is_empty() {
            continue;
        }
        // One encoded key, one tab, one encoded value. A line that does not
        // parse is skipped rather than failing the store: a human editing
        // the file by hand must not lose every preference over one typo.
        let Some((key, value)) = line.split_once('\t') else {
            continue;
        };
        if let (Some(key), Some(value)) = (percent_decode(key), percent_decode(value)) {
            map.insert(key, value);
        }
    }
    map
}

pub fn get(path: &Path, key: &str) -> Option<String> {
    load(path).get(key).cloned()
}

pub fn set(path: &Path, key: &str, value: &str) -> Result<(), String> {
    let mut map = load(path);
    map.insert(key.to_string(), value.to_string());
    write(path, &map)
}

pub fn remove(path: &Path, key: &str) -> Result<(), String> {
    let mut map = load(path);
    map.remove(key);
    write(path, &map)
}

/// The store, serialized and renamed into place.
fn write(path: &Path, map: &BTreeMap<String, String>) -> Result<(), String> {
    let mut contents = String::new();
    for (key, value) in map {
        contents.push_str(&percent_encode(key));
        contents.push('\t');
        contents.push_str(&percent_encode(value));
        contents.push('\n');
    }

    // The parent directory is created here, not by `open`: creating it is
    // part of *storing*, and a read-only open of a store that was never
    // written must not manufacture directories.
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)
            .map_err(|e| format!("prefs: could not create {}: {e}", parent.display()))?;
    }

    let temp: PathBuf = path.with_extension(format!("prefs-tmp-{}", std::process::id()));
    let result = std::fs::write(&temp, contents)
        .and_then(|_| std::fs::rename(&temp, path))
        .map_err(|e| format!("prefs: could not write {}: {e}", path.display()));
    if result.is_err() {
        // The temp file is garbage the moment the rename did not happen.
        let _ = std::fs::remove_file(&temp);
    }
    result
}

/// Percent-encode everything outside the unreserved set.
///
/// Unreserved is `[A-Za-z0-9_.~-]` — the RFC 3986 set plus `~`, chosen so
/// that an ordinary preference value ("dark", "1.25", "zh-CN") round-trips
/// as itself and only the structurally dangerous bytes (`=`, tab, newline,
/// `%`, and anything non-ASCII) become escapes.
fn percent_encode(s: &str) -> String {
    let mut out = String::with_capacity(s.len());
    for byte in s.bytes() {
        match byte {
            b'A'..=b'Z' | b'a'..=b'z' | b'0'..=b'9' | b'_' | b'.' | b'-' | b'~' => {
                out.push(byte as char)
            }
            _ => out.push_str(&format!("%{byte:02X}")),
        }
    }
    out
}

/// The inverse of [`percent_encode`], or `None` when the input is not valid
/// output of it (a lone `%`, or a truncated escape).
///
/// Decoding is total over what the encoder produces and refuses everything
/// else — a hand-edited `%ZZ` line is skipped by [`load`] rather than
/// interpreted.
///
/// Read off the bytes, not the `str`: slicing `s[i + 1..i + 3]` assumes those
/// two byte offsets land on character boundaries, and after a `%` they need
/// not. `%€` is five bytes, so the bounds check passes and the slice panics
/// *inside* the three-byte `€` — a hand-edited file would abort the process
/// instead of being skipped, which is the opposite of what [`load`] promises.
fn percent_decode(s: &str) -> Option<String> {
    let bytes = s.as_bytes();
    let mut out = Vec::with_capacity(bytes.len());
    let mut i = 0;
    while i < bytes.len() {
        if bytes[i] == b'%' {
            // An escape needs exactly two hex digits after it; anything
            // shorter is not something the encoder ever wrote.
            let hi = hex_digit(*bytes.get(i + 1)?)?;
            let lo = hex_digit(*bytes.get(i + 2)?)?;
            out.push((hi << 4) | lo);
            i += 3;
        } else {
            out.push(bytes[i]);
            i += 1;
        }
    }
    String::from_utf8(out).ok()
}

/// One ASCII hex digit as its value, or `None` for anything else.
///
/// The encoder writes uppercase only, but a human editing the file may type
/// either case, and refusing lowercase would make the file's own docs wrong.
fn hex_digit(byte: u8) -> Option<u8> {
    match byte {
        b'0'..=b'9' => Some(byte - b'0'),
        b'A'..=b'F' => Some(byte - b'A' + 10),
        b'a'..=b'f' => Some(byte - b'a' + 10),
        _ => None,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The codec round-trips every byte, and escapes exactly the set the
    /// format needs escaped.
    #[test]
    fn the_codec_round_trips_everything() {
        // The structural characters must not survive encoding as themselves —
        // that is what keeps the file parseable no matter what a value
        // contains. `%` is the one exception: it *is* the escape marker, so it
        // has to appear in the encoded form, escaped as `%25` like any other
        // non-unreserved byte.
        for dangerous in ["a\tb", "a\nb", "a=b", "a%b", ""] {
            let encoded = percent_encode(dangerous);
            assert!(
                !encoded.contains(['\t', '\n', '=']),
                "{dangerous:?} escaped to {encoded:?}"
            );
            assert_eq!(percent_decode(&encoded).as_deref(), Some(dangerous));
        }
        // A literal `%` cannot be left bare, or the decoder would read it as
        // the start of an escape and the value would not come back.
        assert_eq!(percent_encode("100%"), "100%25");
        // Ordinary values stay human-readable in the file.
        assert_eq!(percent_encode("dark-theme_1.25~zh"), "dark-theme_1.25~zh");
        // Non-ASCII becomes bytes, and comes back.
        let s = "设置=dark";
        assert_eq!(percent_decode(&percent_encode(s)).as_deref(), Some(s));
    }

    /// A hand-edited corruption is refused, not interpreted.
    #[test]
    fn decoding_refuses_what_the_encoder_never_writes() {
        assert_eq!(percent_decode("a%Z"), None); // not hex
        assert_eq!(percent_decode("a%"), None); // truncated
        assert_eq!(percent_decode("%0"), None); // one nibble
        // A multi-byte character after `%` passes a byte-count bounds check
        // and then panics on a `str` slice that cuts it in half. The file is
        // meant to survive hand-editing, so this must be a skip, not an abort.
        assert_eq!(percent_decode("%€"), None);
        assert_eq!(percent_decode("x%€"), None);
        // Lowercase is still hex; a human may type it either way.
        assert_eq!(percent_decode("%2f").as_deref(), Some("/"));
    }
}

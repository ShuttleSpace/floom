//! The Windows store's file format: a magic, then `(key, sealed value)` pairs
//! with length prefixes. Lengths are little-endian `u32`s, so a key or a value
//! may hold anything a UTF-8 string can, including newlines.
//!
//! Kept out of the Windows-only module so the parser is tested on every host:
//! a corrupt file has to be an `Err`, never a silently empty store.

const MAGIC: &[u8; 4] = b"FLS1";

pub fn encode(entries: &[(String, Vec<u8>)]) -> Vec<u8> {
    let mut out = Vec::new();
    out.extend_from_slice(MAGIC);
    for (key, value) in entries {
        out.extend_from_slice(&(key.len() as u32).to_le_bytes());
        out.extend_from_slice(key.as_bytes());
        out.extend_from_slice(&(value.len() as u32).to_le_bytes());
        out.extend_from_slice(value);
    }
    out
}

pub fn decode(bytes: &[u8]) -> Result<Vec<(String, Vec<u8>)>, String> {
    let mut rest = bytes;
    if rest.len() < 4 || &rest[..4] != MAGIC {
        return Err("secure-storage: the store file is not one this crate wrote".into());
    }
    rest = &rest[4..];

    let mut entries = Vec::new();
    while !rest.is_empty() {
        let key = take(rest, "key")?;
        rest = &rest[4 + key.len()..];
        let value = take(rest, "value")?;
        rest = &rest[4 + value.len()..];

        let key = String::from_utf8(key.to_vec())
            .map_err(|_| "secure-storage: a key in the store file is not UTF-8".to_string())?;
        entries.push((key, value.to_vec()));
    }
    Ok(entries)
}

/// The length-prefixed bytes at the front of `rest`, after checking the prefix
/// actually covers a slice that exists.
fn take<'a>(rest: &'a [u8], what: &str) -> Result<&'a [u8], String> {
    if rest.len() < 4 {
        return Err(format!(
            "secure-storage: the store file ends inside a {what} length"
        ));
    }
    let len = u32::from_le_bytes([rest[0], rest[1], rest[2], rest[3]]) as usize;
    if rest.len() - 4 < len {
        return Err(format!(
            "secure-storage: the store file ends inside a {what}"
        ));
    }
    Ok(&rest[4..4 + len])
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn entries_round_trip() {
        let entries = vec![
            ("token".to_string(), vec![1, 2, 3]),
            ("a\nb".to_string(), vec![]),
            ("".to_string(), vec![0xff; 300]),
        ];
        assert_eq!(decode(&encode(&entries)).unwrap(), entries);
    }

    #[test]
    fn foreign_bytes_are_refused() {
        assert!(decode(b"").is_err());
        assert!(decode(b"FLS2\x00\x00\x00\x00").is_err());
        assert!(decode(b"json-like").is_err());
    }

    #[test]
    fn a_truncated_entry_is_refused() {
        let full = encode(&[("token".to_string(), vec![9, 9, 9])]);
        for cut in 5..full.len() {
            assert!(
                decode(&full[..cut]).is_err(),
                "{cut} bytes should not decode"
            );
        }
    }
}

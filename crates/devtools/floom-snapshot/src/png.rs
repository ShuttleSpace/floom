// SPDX-License-Identifier: MIT
//! A dependency-free RGBA8 PNG encoder.
//!
//! A visual-regression dump has to be readable by any image viewer and by the
//! accuracy loop, without pulling an image codec into the snapshot crate: the
//! crate is deliberately std-only (see Cargo.toml), and its install size is a
//! release gate. PNG is a container we can write correctly in ~150 lines:
//!
//! * one `IHDR` chunk (8-bit RGBA, no interlace),
//! * `IDAT` holding a zlib stream whose DEFLATE blocks are **stored** (type
//!   `00`, i.e. uncompressed) — compression is not the goal, portability is,
//! * one `IEND` chunk,
//! * each chunk length-prefixed and CRC-32'd, each scanline prefixed with a
//!   filter-type-0 byte, and the zlib stream trailed by an Adler-32 of the raw
//!   scanline bytes.
//!
//! Stored blocks keep the encoder honest: there is no compressor to drift, and
//! every byte of the raw stream is verifiable against the source buffer. The
//! cost is file size; these are CI artifacts, not shipped assets.
//!
//! [`encode_png`] returns the bytes so callers can hash or ship them;
//! [`write_png`] is the convenience that writes them to a path.

use std::path::Path;

/// Why an encoding could not be produced.
#[derive(Debug)]
pub enum PngError {
    /// Width or height was zero.
    ZeroDimension,
    /// The dimensions are too large to address as a byte buffer.
    TooLarge,
    /// The buffer is not `width * height * 4` bytes.
    LengthMismatch { expected: usize, got: usize },
    /// Writing to a path failed.
    Io(std::io::Error),
}

impl std::fmt::Display for PngError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            PngError::ZeroDimension => write!(f, "PNG dimensions must be non-zero"),
            PngError::TooLarge => write!(f, "PNG dimensions overflow the byte buffer"),
            PngError::LengthMismatch { expected, got } => {
                write!(f, "expected {expected} RGBA bytes, got {got}")
            }
            PngError::Io(e) => write!(f, "{e}"),
        }
    }
}

impl std::error::Error for PngError {}

impl From<std::io::Error> for PngError {
    fn from(e: std::io::Error) -> Self {
        PngError::Io(e)
    }
}

/// Encode `rgba` (row-major, 4 bytes per pixel, top-to-bottom) as a PNG.
pub fn encode_png(rgba: &[u8], width: u32, height: u32) -> Result<Vec<u8>, PngError> {
    if width == 0 || height == 0 {
        return Err(PngError::ZeroDimension);
    }
    let expected = (width as usize)
        .checked_mul(height as usize)
        .and_then(|n| n.checked_mul(4))
        .ok_or(PngError::TooLarge)?;
    if rgba.len() != expected {
        return Err(PngError::LengthMismatch {
            expected,
            got: rgba.len(),
        });
    }

    // Raw scanlines: each row is one filter byte (0 = None) + its RGBA bytes.
    let row_bytes = width as usize * 4;
    let mut raw = Vec::with_capacity((row_bytes + 1) * height as usize);
    for row in rgba.chunks_exact(row_bytes) {
        raw.push(0);
        raw.extend_from_slice(row);
    }

    let mut out = Vec::with_capacity(raw.len() + 64);
    out.extend_from_slice(&[0x89, b'P', b'N', b'G', b'\r', b'\n', 0x1a, b'\n']);

    // IHDR: width, height, bit depth 8, colour type 6 (RGBA), compression 0,
    // filter method 0, interlace 0.
    let mut ihdr = Vec::with_capacity(13);
    ihdr.extend_from_slice(&width.to_be_bytes());
    ihdr.extend_from_slice(&height.to_be_bytes());
    ihdr.extend_from_slice(&[8, 6, 0, 0, 0]);
    chunk(&mut out, b"IHDR", &ihdr);

    chunk(&mut out, b"IDAT", &zlib_stored(&raw));
    chunk(&mut out, b"IEND", &[]);
    Ok(out)
}

/// Write an RGBA PNG to `path`. See [`encode_png`] for the buffer layout.
pub fn write_png(
    path: impl AsRef<Path>,
    rgba: &[u8],
    width: u32,
    height: u32,
) -> Result<(), PngError> {
    let bytes = encode_png(rgba, width, height)?;
    std::fs::write(path, bytes)?;
    Ok(())
}

/// Append one PNG chunk: big-endian length, type, data, CRC-32(type + data).
fn chunk(out: &mut Vec<u8>, kind: &[u8; 4], data: &[u8]) {
    out.extend_from_slice(&(data.len() as u32).to_be_bytes());
    out.extend_from_slice(kind);
    out.extend_from_slice(data);
    let mut crc_input = Vec::with_capacity(4 + data.len());
    crc_input.extend_from_slice(kind);
    crc_input.extend_from_slice(data);
    out.extend_from_slice(&crc32(&crc_input).to_be_bytes());
}

/// A zlib stream (RFC 1950) whose DEFLATE payload is stored blocks (RFC 1951
/// §3.2.3). `0x78 0x01` is CM=8/CINFO=7 with a valid FCHECK and no dictionary.
fn zlib_stored(raw: &[u8]) -> Vec<u8> {
    let mut out = Vec::with_capacity(raw.len() + raw.len() / 65535 * 5 + 6);
    out.extend_from_slice(&[0x78, 0x01]);

    // DEFLATE stored blocks cap their length at 65535 bytes (a u16).
    let mut offset = 0;
    loop {
        let remaining = raw.len() - offset;
        let len = remaining.min(65535);
        let final_block = len == remaining;
        out.push(u8::from(final_block)); // BFINAL in bit 0, BTYPE=00
        let n = len as u16;
        out.extend_from_slice(&n.to_le_bytes());
        out.extend_from_slice(&(!n).to_le_bytes());
        out.extend_from_slice(&raw[offset..offset + len]);
        offset += len;
        if final_block {
            break;
        }
    }

    out.extend_from_slice(&adler32(raw).to_be_bytes());
    out
}

/// CRC-32/IEEE (polynomial 0xEDB88320), the checksum PNG chunks use.
fn crc32(bytes: &[u8]) -> u32 {
    let mut crc = 0xffff_ffffu32;
    for &byte in bytes {
        crc ^= byte as u32;
        for _ in 0..8 {
            crc = if crc & 1 != 0 {
                (crc >> 1) ^ 0xedb8_8320
            } else {
                crc >> 1
            };
        }
    }
    !crc
}

/// Adler-32 of the uncompressed stream, as zlib requires. Modulo is taken
/// every `NMAX` bytes so `a`/`b` stay within `u32` for arbitrarily large input.
fn adler32(bytes: &[u8]) -> u32 {
    const MOD: u32 = 65521;
    const NMAX: usize = 5552;
    let (mut a, mut b) = (1u32, 0u32);
    for group in bytes.chunks(NMAX) {
        for &byte in group {
            a += byte as u32;
            b += a;
        }
        a %= MOD;
        b %= MOD;
    }
    (b << 16) | a
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn crc32_matches_the_known_check_value() {
        // CRC-32("123456789") = 0xCBF43926 (the classic check value).
        assert_eq!(crc32(b"123456789"), 0xcbf4_3926);
    }

    #[test]
    fn adler32_matches_the_known_check_value() {
        // Adler-32("Wikipedia") = 0x11E60398.
        assert_eq!(adler32(b"Wikipedia"), 0x11e6_0398);
    }

    #[test]
    fn a_png_has_the_expected_chunk_structure() {
        let rgba = [10, 20, 30, 255, 40, 50, 60, 128]; // 2x1
        let png = encode_png(&rgba, 2, 1).unwrap();
        assert_eq!(
            &png[..8],
            &[0x89, b'P', b'N', b'G', b'\r', b'\n', 0x1a, b'\n']
        );

        // Walk the chunks and verify each CRC; collect the types in order.
        let mut at = 8;
        let mut kinds = Vec::new();
        let mut idat = Vec::new();
        while at + 12 <= png.len() {
            let len = u32::from_be_bytes(png[at..at + 4].try_into().unwrap()) as usize;
            let kind = &png[at + 4..at + 8];
            let data_end = at + 8 + len;
            let mut crc_input = Vec::with_capacity(4 + len);
            crc_input.extend_from_slice(kind);
            crc_input.extend_from_slice(&png[at + 8..data_end]);
            let stored = u32::from_be_bytes(png[data_end..data_end + 4].try_into().unwrap());
            assert_eq!(stored, crc32(&crc_input), "CRC of {:?}", kind);
            if kind == b"IDAT" {
                idat.extend_from_slice(&png[at + 8..data_end]);
            }
            kinds.push(String::from_utf8_lossy(kind).to_string());
            at = data_end + 4;
        }
        assert_eq!(at, png.len(), "no trailing bytes after IEND");
        assert_eq!(kinds, ["IHDR", "IDAT", "IEND"]);

        // IHDR carries the dimensions and the RGBA8 signature.
        let ihdr_at = 8 + 8;
        assert_eq!(
            u32::from_be_bytes(png[ihdr_at..ihdr_at + 4].try_into().unwrap()),
            2
        );
        assert_eq!(
            u32::from_be_bytes(png[ihdr_at + 4..ihdr_at + 8].try_into().unwrap()),
            1
        );
        assert_eq!(&png[ihdr_at + 8..ihdr_at + 13], &[8, 6, 0, 0, 0]);

        // zlib header, one final stored block, then Adler-32 of the raw stream.
        assert_eq!(&idat[..2], &[0x78, 0x01]);
        assert_eq!(idat[2], 0x01, "a single final stored block");
        let expected_raw = vec![0, 10, 20, 30, 255, 40, 50, 60, 128];
        assert_eq!(
            u32::from_be_bytes(idat[idat.len() - 4..].try_into().unwrap()),
            adler32(&expected_raw)
        );
    }

    #[test]
    fn pixels_round_trip_through_a_real_decoder() {
        // 3x2 with distinct, non-grey pixels including a transparent one.
        let rgba = [
            1, 2, 3, 255, 200, 100, 50, 128, 0, 0, 0, 0, //
            255, 255, 255, 255, 7, 8, 9, 64, 90, 180, 220, 255,
        ];
        let png = encode_png(&rgba, 3, 2).unwrap();
        let decoded = image::load_from_memory(&png).unwrap().to_rgba8();
        assert_eq!((decoded.width(), decoded.height()), (3, 2));
        assert_eq!(decoded.as_raw(), &rgba);
    }

    #[test]
    fn large_images_split_into_multiple_deflate_blocks() {
        // Width 30000 forces a row of 120000 bytes -> more than one 65535 block.
        let width = 30_000u32;
        let rgba = vec![0x2a; width as usize * 4];
        let png = encode_png(&rgba, width, 1).unwrap();
        assert!(png.len() > 65535);
        let decoded = image::load_from_memory(&png).unwrap().to_rgba8();
        assert_eq!(decoded.width(), width);
        assert_eq!(decoded.as_raw(), &rgba);
    }

    #[test]
    fn bad_buffers_are_rejected() {
        assert!(matches!(
            encode_png(&[], 0, 1),
            Err(PngError::ZeroDimension)
        ));
        assert!(matches!(
            encode_png(&[], 1, 0),
            Err(PngError::ZeroDimension)
        ));
        assert!(matches!(
            encode_png(&[0u8; 3], 1, 1),
            Err(PngError::LengthMismatch {
                expected: 4,
                got: 3
            })
        ));
    }
}

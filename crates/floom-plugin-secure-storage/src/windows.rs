//! Windows: one file per store in the user's config directory, each value
//! sealed with DPAPI (`CryptProtectData`).
//!
//! DPAPI ties the ciphertext to the current user on this machine, which is the
//! closest thing Windows has to a keychain — and what
//! `flutter_secure_storage`'s Windows implementation uses.

use std::path::PathBuf;

use windows_sys::Win32::Foundation::LocalFree;
use windows_sys::Win32::Security::Cryptography::{
    CRYPT_INTEGER_BLOB, CRYPTPROTECT_UI_FORBIDDEN, CryptProtectData, CryptUnprotectData,
};

use crate::map;

pub fn get(service: &str, key: &str) -> Result<Option<String>, String> {
    let entries = read(service)?;
    let Some((_, sealed)) = entries.iter().find(|(stored, _)| stored == key) else {
        return Ok(None);
    };
    let plaintext = unseal(sealed)?;
    String::from_utf8(plaintext)
        .map(Some)
        .map_err(|_| format!("secure-storage: the value under {key:?} is not UTF-8"))
}

pub fn set(service: &str, key: &str, value: &str) -> Result<(), String> {
    let mut entries = read(service)?;
    let sealed = seal(value.as_bytes())?;
    match entries.iter_mut().find(|(stored, _)| stored == key) {
        Some(slot) => slot.1 = sealed,
        None => entries.push((key.to_string(), sealed)),
    }
    write(service, &entries)
}

pub fn remove(service: &str, key: &str) -> Result<(), String> {
    let mut entries = read(service)?;
    entries.retain(|(stored, _)| stored != key);
    write(service, &entries)
}

fn store_path(service: &str) -> PathBuf {
    let mut path = dirs::config_dir().unwrap_or_else(std::env::temp_dir);
    path.push(format!("floom-secure-{service}.bin"));
    path
}

fn read(service: &str) -> Result<Vec<(String, Vec<u8>)>, String> {
    match std::fs::read(store_path(service)) {
        Ok(bytes) => map::decode(&bytes),
        Err(e) if e.kind() == std::io::ErrorKind::NotFound => Ok(Vec::new()),
        Err(e) => Err(format!(
            "secure-storage: cannot read the store for {service:?}: {e}"
        )),
    }
}

/// Temp file then rename: a reader never sees a half-written store.
fn write(service: &str, entries: &[(String, Vec<u8>)]) -> Result<(), String> {
    let path = store_path(service);
    let temp = path.with_extension("bin.tmp");
    std::fs::write(&temp, map::encode(entries))
        .map_err(|e| format!("secure-storage: cannot write the store: {e}"))?;
    std::fs::rename(&temp, &path)
        .map_err(|e| format!("secure-storage: cannot replace the store: {e}"))
}

fn seal(plaintext: &[u8]) -> Result<Vec<u8>, String> {
    let input = CRYPT_INTEGER_BLOB {
        cbData: plaintext.len() as u32,
        pbData: plaintext.as_ptr() as *mut u8,
    };
    let mut output = CRYPT_INTEGER_BLOB::default();
    let ok = unsafe {
        CryptProtectData(
            &input,
            std::ptr::null(),
            std::ptr::null(),
            std::ptr::null(),
            std::ptr::null(),
            CRYPTPROTECT_UI_FORBIDDEN,
            &mut output,
        )
    };
    finish(ok, output, "CryptProtectData")
}

fn unseal(sealed: &[u8]) -> Result<Vec<u8>, String> {
    let input = CRYPT_INTEGER_BLOB {
        cbData: sealed.len() as u32,
        pbData: sealed.as_ptr() as *mut u8,
    };
    let mut output = CRYPT_INTEGER_BLOB::default();
    let ok = unsafe {
        CryptUnprotectData(
            &input,
            std::ptr::null_mut(),
            std::ptr::null(),
            std::ptr::null(),
            std::ptr::null(),
            CRYPTPROTECT_UI_FORBIDDEN,
            &mut output,
        )
    };
    finish(ok, output, "CryptUnprotectData")
}

/// Copies DPAPI's output blob into a `Vec` and releases it.
fn finish(ok: i32, output: CRYPT_INTEGER_BLOB, call: &str) -> Result<Vec<u8>, String> {
    if ok == 0 {
        return Err(format!(
            "secure-storage: {call} failed: {}",
            std::io::Error::last_os_error()
        ));
    }
    let bytes = unsafe { std::slice::from_raw_parts(output.pbData, output.cbData as usize) };
    let bytes = bytes.to_vec();
    unsafe {
        LocalFree(output.pbData as *mut core::ffi::c_void);
    }
    Ok(bytes)
}

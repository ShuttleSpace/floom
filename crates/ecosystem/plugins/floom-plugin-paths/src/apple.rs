//! iOS and macOS directories via `NSSearchPathForDirectoriesInDomains`.
//!
//! Both platforms answer with the same Foundation call — the one
//! `path_provider`'s darwin implementations use — so one arm serves both
//! targets, like `package-info`'s `apple.rs`.
//!
//! # Why a C function and not a message send
//!
//! `NSSearchPathForDirectoriesInDomains` is a Foundation *global function*,
//! not a method on any class, so it is declared and called through plain FFI
//! rather than `msg_send!`. An earlier version of this arm sent it to
//! `NSFileManager` as a class method, and `objc2`'s runtime check refused it
//! (`method not found`) — which is exactly the kind of mistake a typed
//! boundary is there to catch.
//!
//! # Downloads
//!
//! iOS has no per-app Downloads directory (the system routes user downloads
//! through the Files app). Redirecting at `Documents` would answer a different
//! question, so the arm reports `None`. macOS *does* have a Downloads folder,
//! but the search-path API does not expose a per-app subdirectory there, and
//! inventing `~/Downloads/<app>` would be this crate choosing policy — the
//! crate doc's "does not create, does not invent" rule — so it reports `None`
//! there too and leaves the desktop answer to the XDG/known-folder arm's
//! platforms.

use super::AppDirectory;
use std::ffi::c_char;
use std::path::PathBuf;

#[link(name = "Foundation", kind = "framework")]
unsafe extern "C" {
    /// `NSArray<NSSearchPathForDirectoriesInDomains...>` — Foundation's
    /// global function, not a method.
    ///
    /// The `directory` argument is `NSSearchPathDirectory` (an `NSUInteger`
    /// enum), `domain_mask` is `NSSearchPathDomainMask`, and `expand_tilde`
    /// is `BOOL` (`signed char` in Foundation's C ABI, where `true` is 1).
    ///
    /// Returns an `NSArray` of `NSString` — autoreleased, so it is valid for
    /// the duration of this call and copied out immediately.
    fn NSSearchPathForDirectoriesInDomains(
        directory: usize,
        domain_mask: usize,
        expand_tilde: bool,
    ) -> *mut AnyObject;
}

use objc2::msg_send;
/// objc2's `AnyObject` keeps the declaration from needing a second runtime
/// crate: the array and the strings are traversed by `UTF8String`, the same
/// trick the other Apple arms use.
use objc2::runtime::AnyObject;

/// `NSSearchPathDirectory` values, from `NSPathUtilities.h`.
const NS_DOCUMENT_DIRECTORY: usize = 9;
const NS_CACHES_DIRECTORY: usize = 13;
const NS_APPLICATION_SUPPORT_DIRECTORY: usize = 14;

/// `NSUserDomainMask` — the per-user domain. The local and network domains
/// are shared volumes an app has no business writing to.
const NS_USER_DOMAIN_MASK: usize = 1;

pub fn app_dir(dir: AppDirectory) -> Option<PathBuf> {
    let directory = match dir {
        AppDirectory::ApplicationSupport => NS_APPLICATION_SUPPORT_DIRECTORY,
        AppDirectory::Documents => NS_DOCUMENT_DIRECTORY,
        AppDirectory::Cache => NS_CACHES_DIRECTORY,
        AppDirectory::Downloads => return None,
    };
    unsafe { search(directory) }
}

unsafe fn search(directory: usize) -> Option<PathBuf> {
    let paths =
        unsafe { NSSearchPathForDirectoriesInDomains(directory, NS_USER_DOMAIN_MASK, true) };
    if paths.is_null() {
        return None;
    }
    // The array's first entry is the domain's answer.
    let first: *mut AnyObject = unsafe { msg_send![paths, firstObject] };
    if first.is_null() {
        return None;
    }
    let cstr: *const c_char = unsafe { msg_send![first, UTF8String] };
    if cstr.is_null() {
        return None;
    }
    let s = unsafe { std::ffi::CStr::from_ptr(cstr) }
        .to_string_lossy()
        .into_owned();
    if s.is_empty() {
        None
    } else {
        Some(PathBuf::from(s))
    }
}

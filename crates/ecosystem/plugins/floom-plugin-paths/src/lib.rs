//! The app's own directories: application support, documents, cache, downloads.
//!
//! This is `path_provider`'s question — "where may this app write" — and it is
//! the first thing almost every app that persists anything needs, which is why
//! it opens the catalog's Tier A (`docs/plugin-catalog.md`, A1).
//!
//! A **capability plugin**: one capability, one crate, opt-in by dependency.
//!
//! # Why every arm is a real read
//!
//! Unlike the nine mobile-only plugins, this plugin has no host-inert arm at
//! all. On the desktop the *host conventions are the source of truth*: Linux
//! resolves `XDG_DATA_HOME`/`XDG_CACHE_HOME`, macOS answers
//! `~/Library/Application Support` and `~/Library/Caches`, Windows answers
//! `%APPDATA%` and `%LOCALAPPDATA%` — the same questions the phone arms ask
//! the OS, answered by the same kind of authority. An `Err` here would be a
//! lie about the desktop, and a "no directories" default would be worse: it
//! would look like an answer while being none.
//!
//! The one place with no answer is a platform the crate does not cover (say,
//! wasm or a terminal target): there [`app_dir`] reports `None`, which is
//! "this platform has nothing to say", not "an empty path".
//!
//! # What this crate deliberately does not do
//!
//! It does not create the directories it reports — the OS convention says
//! where they belong, not that they exist (`dirs` itself documents that
//! `data_dir` may point at a non-existent path on some platforms). Creation is
//! policy: an app may want permissions, may want a versioned subdirectory, or
//! may not want the directory at all until first write. Callers combine this
//! with `std::fs::create_dir_all` and own the failure.
//!
//! It also does not answer "where are the user's *Documents* on a phone".
//! Mobile `Documents` is app-private storage — the app's own files, not the
//! user-visible folder — because that is what `path_provider`'s
//! `documentsDirectory` means there. A shared user-documents picker is a
//! different capability (file-dialog, A3 in the catalog) and deliberately not
//! this crate's promise.

use std::path::PathBuf;

#[cfg(target_os = "android")]
mod android;
#[cfg(any(target_os = "ios", target_os = "macos"))]
mod apple;
#[cfg(all(
    any(target_os = "linux", target_os = "windows"),
    not(target_env = "ohos")
))]
mod desktop;
#[cfg(target_env = "ohos")]
mod ohos;

/// One of the app's own directories.
///
/// The variants are the union of what `path_provider` and `xdg_directories`
/// answer, kept small on purpose: five names a caller can hold in their head,
/// each mapping to a concrete OS location on every supported platform.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum AppDirectory {
    /// Persistent app-private data the user does not see.
    ///
    /// `NSApplicationSupportDirectory`, Android `getFilesDir`,
    /// XDG `XDG_DATA_HOME`-backed, Windows `%APPDATA%`.
    ApplicationSupport,
    /// User-visible documents this app creates.
    ///
    /// `NSDocumentDirectory`, Android `getFilesDir` (see the crate doc: a
    /// phone's app-private storage is the honest equivalent — the shared
    /// user-visible folder is a different capability), XDG `XDG_DOCUMENTS_DIR`
    /// / `~/Documents`, Windows `{FOLDERID_Documents}`.
    Documents,
    /// Throwaway files the OS may evict.
    ///
    /// `NSCachesDirectory`, Android `getCacheDir`, XDG `XDG_CACHE_HOME`,
    /// Windows `%LOCALAPPDATA%\cache`.
    Cache,
    /// Files the user asked to keep.
    ///
    /// Not a `path_provider` name — it is here because downloads is the one
    /// directory both `xdg_directories` and Windows expose distinctly, and
    /// Android keeps separate storage for it (see [`external_files_dir`]).
    /// XDG `XDG_DOWNLOAD_DIR`, Windows `{FOLDERID_Downloads}`; iOS has no
    /// per-app equivalent (the Files app owns user downloads), so that arm
    /// reports `None`.
    Downloads,
}

/// The app's own external storage on Android: the app-scoped directory on
/// shared storage, no permission required, removed with the app.
///
/// This is deliberately *not* an [`AppDirectory`] variant: it exists on exactly
/// one platform, and a variant that is `None` everywhere else would make every
/// caller write a fallback for a directory only Android distinguishes. A
/// separate function says "Android-only capability" by existing.
pub fn external_files_dir() -> Option<PathBuf> {
    #[cfg(target_os = "android")]
    {
        android::external_files_dir()
    }
    #[cfg(not(target_os = "android"))]
    {
        None
    }
}

/// Where the app's own `[AppDirectory::Downloads]` files live, if this platform
/// has the concept.
///
/// iOS has no per-app Downloads directory (the system routes downloads through
/// the Files app, which is the file-picker capability, not a path), so that arm
/// reports `None` rather than redirecting at `Documents` — the nearest
/// approximate path would answer a different question.
pub fn app_dir(dir: AppDirectory) -> Option<PathBuf> {
    #[cfg(any(
        target_os = "android",
        target_os = "ios",
        target_os = "macos",
        target_os = "linux",
        target_os = "windows"
    ))]
    {
        #[cfg(target_os = "android")]
        {
            android::app_dir(dir)
        }
        #[cfg(any(target_os = "ios", target_os = "macos"))]
        {
            apple::app_dir(dir)
        }
        #[cfg(all(
            any(target_os = "linux", target_os = "windows"),
            not(target_env = "ohos")
        ))]
        {
            desktop::app_dir(dir)
        }
        #[cfg(target_env = "ohos")]
        {
            ohos::app_dir(dir)
        }
    }
    #[cfg(not(any(
        target_os = "android",
        target_os = "ios",
        target_os = "macos",
        target_os = "linux",
        target_os = "windows"
    )))]
    {
        None
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn every_directory_answers_on_a_supported_desktop() {
        // The desktop arm is a real read (crate doc: "why every arm is a real
        // read"), so on any desktop this crate compiles for, the directories
        // the platform conventions actually define must resolve. A `None`
        // there would mean the arm silently fell through to the inert default
        // — exactly what this test exists to catch.
        //
        // Downloads is the one exception the arms document: macOS's search-path
        // API exposes no per-app Downloads directory and the crate refuses to
        // invent `~/Downloads/<app>`, so `None` on macOS is the honest answer
        // there, not an inert arm. The assertion stays strict for the other
        // three, which every supported desktop defines.
        if cfg!(any(
            target_os = "linux",
            target_os = "macos",
            target_os = "windows"
        )) {
            for dir in [
                AppDirectory::ApplicationSupport,
                AppDirectory::Documents,
                AppDirectory::Cache,
            ] {
                let path = app_dir(dir);
                assert!(
                    path.is_some(),
                    "{dir:?} did not resolve on {} — a desktop arm went inert",
                    std::env::consts::OS
                );
            }
            if cfg!(any(target_os = "linux", target_os = "windows")) {
                assert!(
                    app_dir(AppDirectory::Downloads).is_some(),
                    "Downloads did not resolve on {} — this platform defines it",
                    std::env::consts::OS
                );
            }
        }
    }

    #[test]
    fn no_answer_is_none_not_an_empty_path() {
        // On a platform the crate does not cover (and in the one place the
        // type lets an arm decline), the answer is `None`. An empty `PathBuf`
        // would be a path-shaped value nobody measured.
        if cfg!(not(any(
            target_os = "android",
            target_os = "ios",
            target_os = "macos",
            target_os = "linux",
            target_os = "windows"
        ))) {
            assert_eq!(app_dir(AppDirectory::Cache), None);
        }
    }

    #[test]
    fn external_storage_is_android_only() {
        // The function exists because Android distinguishes internal from
        // app-scoped external storage. Everywhere else the honest answer is
        // `None` — not `Some` pointing at the internal dir, which would
        // rename the capability out from under the caller.
        if cfg!(not(target_os = "android")) {
            assert_eq!(external_files_dir(), None);
        }
    }
}

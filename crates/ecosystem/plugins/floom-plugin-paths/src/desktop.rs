//! Linux and Windows directories via the `dirs` crate.
//!
//! The desktop arm is a real read, not a host-inert stub — the crate doc's
//! "why every arm is a real read". `dirs` resolves the platform conventions
//! (`XDG_DATA_HOME`/`XDG_CACHE_HOME` on Linux, known folders on Windows) and
//! is the maintained crate for exactly this, so this arm's job is the mapping,
//! not the resolution.
//!
//! # The mapping, and one honest gap
//!
//! | [`AppDirectory`] | Linux | Windows |
//! |---|---|---|
//! | `ApplicationSupport` | `XDG_DATA_HOME` | `%APPDATA%` (`dirs::data_dir`) |
//! | `Documents` | `XDG_DOCUMENTS_DIR`, else `~/Documents` | `{FOLDERID_Documents}` |
//! | `Cache` | `XDG_CACHE_HOME` | `%LOCALAPPDATA%` (`dirs::cache_dir`) |
//! | `Downloads` | `XDG_DOWNLOAD_DIR`, else `~/Downloads` | `{FOLDERID_Downloads}` |
//!
//! Documents and Downloads are the two variants `dirs` answers only through
//! its user folders, which can be absent (a Windows account with the folders
//! relocated, or a Linux box with no `XDG_DOCUMENTS_DIR` and no home): those
//! become `None`, which is the honest "this user has no Documents folder",
//! not an error.

use super::AppDirectory;
use std::path::PathBuf;

pub fn app_dir(dir: AppDirectory) -> Option<PathBuf> {
    match dir {
        AppDirectory::ApplicationSupport => dirs::data_dir(),
        AppDirectory::Documents => dirs::document_dir(),
        AppDirectory::Cache => dirs::cache_dir(),
        AppDirectory::Downloads => dirs::download_dir(),
    }
}

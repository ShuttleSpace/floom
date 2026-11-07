//! OpenHarmony app directories need an N-API bridge that is not wired yet.

use std::path::PathBuf;

use crate::AppDirectory;

pub fn app_dir(_dir: AppDirectory) -> Option<PathBuf> {
    None
}

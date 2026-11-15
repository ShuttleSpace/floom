// SPDX-License-Identifier: MIT
//! `cargo metadata` inspection (implemented in Task 3).

use std::path::PathBuf;

/// A resolved Cargo project, ready to package.
pub struct Project {
    pub root: PathBuf,
    pub target_dir: PathBuf,
    pub package_name: String,
    pub default_features: Vec<String>,
}

pub fn inspect(_dir: &std::path::Path) -> Result<Project, String> {
    unimplemented!("implemented in Task 3")
}

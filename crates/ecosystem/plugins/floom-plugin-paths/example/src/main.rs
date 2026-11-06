// SPDX-License-Identifier: MIT
//! Runs the `paths` plugin on this host and prints what it answers.
//!
//! Nothing is asserted: the point is to show the real return values, including
//! the named refusal on a target that does not implement the capability.

use floom_plugin_paths::{AppDirectory, app_dir, external_files_dir};

fn main() {
    for dir in [
        AppDirectory::ApplicationSupport,
        AppDirectory::Documents,
        AppDirectory::Cache,
        AppDirectory::Downloads,
    ] {
        println!("{dir:?}: {:?}", app_dir(dir));
    }
    println!("external_files_dir: {:?}", external_files_dir());
}

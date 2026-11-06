// SPDX-License-Identifier: MIT
//! Runs the `share` plugin on this host and prints what it answers.
//!
//! Nothing is asserted: the point is to show the real return values, including
//! the named refusal on a target that does not implement the capability.

use floom_plugin_share::{share_files, share_text};

fn main() {
    if std::env::args().any(|arg| arg == "--share") {
        println!("share_text: {:?}", share_text("Floom plugin example"));
        println!(
            "share_files: {:?}",
            share_files(&["/tmp/floom-example.txt"])
        );
    } else {
        println!("pass --share to open the share sheet");
    }
}

// SPDX-License-Identifier: MIT
//! Runs the `file-dialog` plugin on this host and prints what it answers.
//!
//! Nothing is asserted: the point is to show the real return values, including
//! the named refusal on a target that does not implement the capability.

use floom_plugin_file_dialog::{Options, SUPPORTED, pick_directory, pick_file, pick_save_file};

fn main() {
    println!("SUPPORTED={SUPPORTED}");
    let options = Options::new();
    if !SUPPORTED {
        // A phone/unknown target has a refusal, not a dialog, and printing it
        // is the honest thing to do.
        println!("pick_file: {:?}", pick_file(&options));
        return;
    }
    if std::env::args().any(|arg| arg == "--pick") {
        println!("pick_file: {:?}", pick_file(&options));
        println!("pick_save_file: {:?}", pick_save_file(&options));
        println!("pick_directory: {:?}", pick_directory(&options));
    } else {
        println!("pass --pick to open the real dialogs");
    }
}

// SPDX-License-Identifier: MIT
//! Runs the `local-auth` plugin on this host and prints what it answers.
//!
//! Nothing is asserted: the point is to show the real return values, including
//! the named refusal on a target that does not implement the capability.

use floom_plugin_local_auth::{authenticate, available_biometric};

fn main() {
    println!("available_biometric: {:?}", available_biometric());
    if std::env::args().any(|arg| arg == "--auth") {
        match authenticate("floom plugin example") {
            Ok(()) => println!("authenticated"),
            Err(e) => println!("authenticate: {e}"),
        }
    } else {
        println!("pass --auth to prompt for authentication");
    }
}

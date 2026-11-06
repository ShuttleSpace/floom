// SPDX-License-Identifier: MIT
//! Runs the `open-url` plugin on this host and prints what it answers.
//!
//! Nothing is asserted: the point is to show the real return values, including
//! the named refusal on a target that does not implement the capability.

use floom_plugin_open_url::open_url;

fn main() {
    // The validation is the part a host build can always show.
    println!("empty URL: {:?}", open_url(""));
    println!("bare string: {:?}", open_url("example.com"));
    if std::env::args().any(|arg| arg == "--open") {
        let url = std::env::args()
            .skip(1)
            .find(|arg| arg != "--open")
            .unwrap_or_else(|| "https://example.com".to_string());
        match open_url(&url) {
            Ok(()) => println!("handed {url} to the OS"),
            Err(e) => println!("open_url: {e}"),
        }
    } else {
        println!("pass --open [url] to hand a URL to the OS");
    }
}

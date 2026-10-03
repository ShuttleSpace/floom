//! Headless entry: `cargo run -p floom2-capabilities`.
//!
//! Prints the capability report. No window, no GPU — this is the mode the
//! golden-example harness runs, and the one to reach for when you want to know
//! what a device can do rather than to look at it.

fn main() {
    if let Err(err) = floom2_capabilities::headless::run() {
        eprintln!("capabilities headless run failed: {err}");
        std::process::exit(1);
    }
}

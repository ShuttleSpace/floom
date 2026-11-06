// SPDX-License-Identifier: MIT
//! Runs the `tray` plugin on this host and prints what it answers.
//!
//! Nothing is asserted: the point is to show the real return values, including
//! the named refusal on a target that does not implement the capability.

use floom_plugin_tray::{MenuItem, Tray};

fn main() {
    // Validation is the part that answers on every host, so it is shown first.
    // `map(|_| ())` keeps the tray handle out of the printout: it is not `Debug`.
    println!("empty label: {:?}", Tray::show("", &[]).map(|_| ()));
    println!(
        "id 0: {:?}",
        Tray::show("Floom", &[MenuItem::new(0, "bad")]).map(|_| ())
    );

    if std::env::args().any(|arg| arg == "--tray") {
        match Tray::show(
            "Floom",
            &[
                MenuItem::new(1, "Open"),
                MenuItem::separator(),
                MenuItem::new(2, "Quit"),
            ],
        ) {
            Ok(mut tray) => println!("events: {:?}", tray.events()),
            Err(e) => println!("Tray::show: {e}"),
        }
    } else {
        println!("pass --tray to create a real tray icon");
    }
}

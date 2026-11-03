//! Puts a tray icon in the menu bar, watches it for ten seconds, then removes
//! it.
//!
//! ```text
//! cargo run --manifest-path crates/ecosystem/plugins/Cargo.toml \
//!   -p floom-plugin-tray --example tray
//! ```
//!
//! Click the icon while it runs: each menu row you choose is printed as it
//! arrives. Nothing is asserted, because an icon no human clicked is not
//! evidence of anything — this example exists to be watched. What it does prove
//! without a human is the pair of calls that can abort the process rather than
//! return: creating the status item and removing it.

fn main() {
    #[cfg(target_os = "macos")]
    run();

    #[cfg(not(target_os = "macos"))]
    println!("tray: this example drives the macOS arm; on other targets run the app itself");
}

#[cfg(target_os = "macos")]
fn run() {
    use floom_plugin_tray::{MenuItem, Tray};
    use objc2::MainThreadMarker;
    use objc2_app_kit::NSApplication;
    use objc2_foundation::{NSDate, NSRunLoop};

    let mtm = MainThreadMarker::new().expect("main is the main thread");
    let _app = NSApplication::sharedApplication(mtm);

    let items = [
        MenuItem::new(1, "Say hello"),
        MenuItem::separator(),
        MenuItem::new(2, "Not available").disabled(),
        MenuItem::new(3, "Quit"),
    ];
    let mut tray = match Tray::show("floom", &items) {
        Ok(tray) => tray,
        Err(error) => {
            println!("tray: {error}");
            return;
        }
    };
    println!("tray: the icon is in the menu bar — click it. Watching for 10 seconds.");

    // Pumping the run loop is what lets AppKit deliver the click, and therefore
    // what lets the menu's action method write into the queue.
    let run_loop = NSRunLoop::currentRunLoop();
    for _ in 0..10 {
        run_loop.runUntilDate(&NSDate::dateWithTimeIntervalSinceNow(1.0));
        for event in tray.events() {
            println!("tray: {event:?}");
        }
    }

    drop(tray);
    println!("tray: removed");
}

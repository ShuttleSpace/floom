//! The headless mode: mount the gallery, verify it painted, and print what every
//! capability said.
//!
//! This is the mode CI and the golden-example harness run, and it is the most
//! useful of the three for judging the example at all. A windowed gallery cannot
//! be asserted on; a report can. Every row is printed with its state, so a change
//! in a capability's reach (a bridge that starts working, a permission that
//! starts being refused) shows up as a diff rather than as a screenshot nobody
//! looks at twice.
//!
//! It also mounts the *same* tree the windowed backends mount — not a
//! stand-in — so "it compiles" and "it lays out" are the same claim here as they
//! are on a device.

use floom_app::WidgetHost;
use floom_layout::LayoutStyle;
use floom_types::Size;

use crate::probe::{self, State};
use crate::ui;

/// The report's viewport. Wide enough for the two-column table, so a row that
/// would clip on a phone still renders here.
const REPORT_SIZE: Size = Size::new(900.0, 900.0);

/// Mount the gallery headlessly, verify it painted, and print the report.
pub fn run() -> Result<(), String> {
    let mut host = WidgetHost::new();
    host.set_size(REPORT_SIZE);
    host.mount_root(LayoutStyle::default(), ui::gallery_root());

    let stats = host.frame();
    if stats.painted == 0 {
        return Err("gallery painted nothing".into());
    }

    print_report();
    Ok(())
}

/// Print every capability's live answer.
pub fn print_report() {
    let rows = probe::all();
    let answered = rows.iter().filter(|r| r.state.is_value()).count();

    println!("floom-capabilities — platform capability report");
    println!(
        "target: {} / os: {} / native capability host: {}",
        std::env::consts::ARCH,
        std::env::consts::OS,
        if probe::native_capability_host() {
            "yes"
        } else {
            "no"
        }
    );
    println!();

    for row in &rows {
        // The marker is the point: `=` answered, `-` the platform has no such
        // capability, `!` it exists but the call failed. A report that printed
        // only the values would hide the absences, which are the interesting
        // half — on a host build almost every row is one of the other two.
        let marker = match row.state {
            State::Value(_) => "=",
            State::Absent => "-",
            State::Error(_) => "!",
        };
        println!("{marker} {:<22} {}", row.name, row.state.text());
    }

    println!();
    println!(
        "{answered} of {} capabilities answered on this host",
        rows.len()
    );
    if !probe::native_capability_host() {
        println!(
            "note: a host build has no native arms, so `-` and `!` rows are the correct \
             answer here, not a failure. Run on Android or iOS to see them fill in."
        );
    }

    let actions = probe::actions();
    println!();
    println!(
        "{} actions available (nothing runs until pressed):",
        actions.len()
    );
    for (label, id) in &actions {
        println!("  {id:<30} {label}");
    }
}

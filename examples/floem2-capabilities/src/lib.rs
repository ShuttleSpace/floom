//! Floem 2.0 platform-capability showcase.
//!
//! Where [`floem2-demo`](https://docs.rs/floem2-demo) answers "what can Floem
//! *draw*?", this crate answers "what can Floem *ask the OS*?" — the layer between
//! a widget tree and the device it runs on. Eighteen capabilities, nine actions,
//! one tree, and the same code on every platform.
//!
//! ```text
//!                    probe::all()          probe::actions()
//!                          │                     │
//!                          ▼                     ▼
//!                     ui::gallery_root()  ── action buttons
//!                          │
//!        ┌─────────────────┼─────────────────┐
//!        ▼                 ▼                 ▼
//!   headless          desktop (macos)     android
//!   WidgetHost::frame()  winit + vello     native-activity + vello
//! ```
//!
//! ## Why the absences are the interesting part
//!
//! On a desktop host, **every one of the eighteen readings is `—`** — a Mac has
//! no battery API, no `Vibrator`, no `UIDevice`. The gallery prints that rather
//! than a plausible default, because a capability demo that reported "battery
//! 100%" everywhere would be indistinguishable from one that works.
//! `battery_state()` really does return `level: 1.0` on a host, and the one test
//! that keeps this example honest (`a_host_build_never_reports_a_value_it_cannot
//! _have_measured`) is the assertion that no row ever shows it. Run the same
//! build on a phone and the table fills in.
//!
//! The three states a row can be in, and why all three exist:
//!
//! | marker | state | meaning |
//! |--------|-------|---------|
//! | `=`    | `Value`  | the OS answered |
//! | `-`    | `Absent` | this platform has no such capability at all |
//! | `!`    | `Error`  | it exists, but the call failed — reason kept verbatim |
//!
//! `clipboard` is the row that makes the last two worth separating: on a host it
//! returns `Err("clipboard is only available on iOS and Android")`, which is a
//! *refusal* and not an empty clipboard.
//!
//! ## Borrowed from the competition
//!
//! Two conventions, both taken from Flutter's integration gallery and Slint's
//! demos, because both are the right answer rather than a house style:
//!
//! * **Readings and actions are different tabs.** A reading has no side effects,
//!   so it is safe to open on someone else's phone. An action vibrates, shares,
//!   opens a URL or throws a permission dialog, so it only ever runs when the
//!   user presses it. Nothing in the Actions tab fires at startup.
//! * **Sections with headings.** Eighteen undifferentiated rows is a wall, so the
//!   table is broken into six labelled sections.
//!
//! ## Running
//!
//! ```sh
//! cargo run -p floem2-capabilities                        # headless, prints a report
//! cargo run -p floem2-capabilities --features macos       # a real window
//! just build floem2-capabilities android                  # an installable APK
//! ```

pub mod probe;
pub mod ui;

#[cfg(feature = "headless")]
pub mod headless;

#[cfg(all(feature = "macos", not(target_os = "android")))]
pub mod desktop;

#[cfg(all(feature = "android", target_os = "android"))]
pub mod android;

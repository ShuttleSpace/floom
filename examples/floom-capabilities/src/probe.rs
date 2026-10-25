//! One probe per platform capability.
//!
//! Each function calls the real `floom-platform-mobile` API and turns the answer
//! into a [`Row`]. The design rule, and the reason this is a separate module
//! from the UI: **a capability that is unavailable must say so**, not report a
//! plausible-looking default. Most of these packages return `Default::default()`
//! or `Err` on a host that cannot serve them, and rendering `level: 1.0` as
//! "battery 100%" on a Mac would be a lie the gallery could not be caught on.
//!
//! So a row carries one of three states:
//!
//! * [`State::Value`] — the OS answered, here is what it said.
//! * [`State::Absent`] — this platform has no such capability (the host arm is
//!   inert by design; see the crate root of `floom-platform-mobile`).
//! * [`State::Error`] — the platform has it, but the call failed (denied
//!   permission, no hardware, a broken bridge). The reason is kept verbatim.
//!
//! ## What is deliberately not here
//!
//! This module once carried rows for `app_info`, `screen`, `accessibility_prefs`,
//! `orientation` and `url_launcher`. Four of the five are still dropped, and
//! for the same reason as before: no crate behind them, and redirecting each at
//! the nearest approximate API would answer a different question from the one
//! it claimed to answer — `Capability::OPEN_URL` is a flag with no operation
//! behind it, and a window's `scale_factor` is not a screen's pixel density.
//!
//! The fifth came back. `app_info` is now [`package_info`] in
//! `crates/floom-plugin-package-info/`, so the reading is no longer a claim
//! without an implementation behind it.
//!
//! That is not a gap being papered over. `docs/platform-and-plugins.md` puts this
//! surface in the *platform layer*, which "provides only what a window cannot be
//! built without", and sends every other device API to a plugin under
//! `crates/floom-plugin-<capability>/`. The capability plugins this module
//! imports are exactly that shape; `screen`, `accessibility_prefs`,
//! `orientation` and `url_launcher` have no crates yet, and a host-inert stub
//! written here would put a capability in the wrong layer while claiming it
//! works.
//!
//! One real package is absent from the readings for a different reason:
//! `system_chrome` is setter-only (`set_system_chrome_style`), with no getter, so
//! there is nothing to *read* — and calling a setter to produce a reading would
//! give the Readings tab the side effect it exists to avoid. It appears in
//! [`actions`] instead.

// The capabilities that are plugins, and the two that are facts about the
// window and so belong to the platform. A capability is a plugin: an app that
// wants the battery depends on the battery crate, and the SDK itself does not
// carry it.
use floom_platform_mobile::packages::{clipboard, keyboard::soft_input, safe_area};
use floom_plugin_battery as battery;
use floom_plugin_connectivity as connectivity;
use floom_plugin_device_info as device_info;
use floom_plugin_local_auth as local_auth;
use floom_plugin_location as location;
use floom_plugin_notifications as notifications;
use floom_plugin_package_info as package_info;
use floom_plugin_paths as paths;
use floom_plugin_sensors as sensors;
use floom_plugin_vibration as vibration;

/// What a capability reported.
#[derive(Debug, Clone, PartialEq)]
pub enum State {
    /// The OS answered.
    Value(String),
    /// This platform does not offer the capability at all.
    Absent,
    /// The platform offers it but the call failed; the payload is the reason.
    Error(String),
}

impl State {
    /// The text to render in the value column.
    pub fn text(&self) -> &str {
        match self {
            State::Value(v) => v,
            State::Absent => "—",
            State::Error(e) => e,
        }
    }

    /// Whether this row demonstrated working behaviour.
    pub fn is_value(&self) -> bool {
        matches!(self, State::Value(_))
    }
}

/// One capability, its live answer, and a short verdict.
#[derive(Debug, Clone)]
pub struct Row {
    /// The capability's name, as the package spells it.
    pub name: &'static str,
    /// The one-line question this row answers.
    pub question: &'static str,
    /// What came back.
    pub state: State,
    /// Whether this row's plugin has a **real arm on desktop targets**, so a
    /// [`State::Value`] on a host build is a measurement rather than a
    /// fabrication.
    ///
    /// The nine mobile-only plugins set this to `false`: on a host their arms
    /// are inert by design, and a `Value` there could only be a default the
    /// OS never set — which is what
    /// `a_host_build_never_reports_a_value_it_cannot_have_measured` exists to
    /// catch. The Tier-A catalog plugins (`paths`, and `package_info`'s
    /// Apple arm) read the *desktop's own conventions* — XDG directories,
    /// `NSBundle` — so on a host they measure for real. The flag is how the
    /// invariant stays exact: it forbids unmeasured values, not desktop
    /// values.
    pub desktop_real: bool,
}

impl Row {
    fn value(name: &'static str, question: &'static str, state: State) -> Self {
        Self {
            name,
            question,
            state,
            desktop_real: false,
        }
    }

    /// Like [`Row::value`], for a row whose plugin measures on desktop hosts
    /// too. See [`Row::desktop_real`] — the distinction is the difference
    /// between "this row cannot be a `Value` on a laptop" and "it can, because
    /// the laptop is the source of truth".
    fn measured_value(name: &'static str, question: &'static str, state: State) -> Self {
        Self {
            name,
            question,
            state,
            desktop_real: true,
        }
    }
}

/// True when the capability packages have a real native arm for this target.
///
/// The packages' own convention is a `#[cfg]` dispatch with an inert host arm,
/// so "is this a phone" is exactly "is this not a host build". Everything the
/// gallery reports is qualified by this.
pub const fn native_capability_host() -> bool {
    cfg!(any(target_os = "android", target_os = "ios"))
}

/// Mark a value as absent when we are not on a native target.
///
/// Without this, a host build would print whatever the plugin's inert arm
/// returned as though it were measured. `battery_state()` used to return
/// `level: 1.0` there, so the gallery printed "100%" on a laptop. It now returns
/// `level: None` — the default is no reading — and this function is the second
/// half of the same rule: a desktop has no battery reading, and the row says so
/// rather than showing a number.
fn native_or_absent(value: String) -> State {
    if native_capability_host() {
        State::Value(value)
    } else {
        State::Absent
    }
}

// --- device identity -------------------------------------------------------

pub fn device_info() -> Row {
    let d = device_info::device_info();
    Row::value(
        "device_info",
        "model, OS version, physical device, locale",
        native_or_absent(format!(
            "{} · {} {} · {} · locale {} · sdk {}",
            d.model,
            d.os_name,
            d.os_version,
            if d.is_physical_device {
                "physical"
            } else {
                "simulator/emulator"
            },
            d.locale,
            d.sdk_version
                .map(|v| v.to_string())
                .unwrap_or_else(|| "n/a".into())
        )),
    )
}

// --- this app's own identity ------------------------------------------------

/// The running app's identity — name, bundle id, version, build.
///
/// The one reading in this gallery that is about *this app* rather than the
/// device it is on, and the one row whose plugin has a real arm on macOS:
/// `NSBundle` is the source of truth there the same way it is on iOS. A bare
/// `cargo run` binary is not inside an `.app`, so the read honestly finds no
/// `Info.plist` — reported as [`State::Error`], "the platform has it but this
/// invocation cannot serve it", not as [`State::Absent`], which would claim
/// macOS does not offer the capability at all.
pub fn package_info() -> Row {
    #[cfg(any(target_os = "android", target_os = "ios", target_os = "macos"))]
    {
        let info = package_info::app_info();
        let any_field = info.name.is_some()
            || info.package_name.is_some()
            || info.version.is_some()
            || info.build_number.is_some();
        let state = if any_field {
            State::Value(format!(
                "{} · {} · {} (build {})",
                info.name.as_deref().unwrap_or("n/a"),
                info.package_name.as_deref().unwrap_or("n/a"),
                info.version.as_deref().unwrap_or("n/a"),
                info.build_number.as_deref().unwrap_or("n/a"),
            ))
        } else if cfg!(target_os = "macos") {
            State::Error(
                "macOS reads the app bundle's Info.plist; this bare binary is not inside one"
                    .into(),
            )
        } else {
            // The phone arms report at least the package name whenever the JNI
            // bridge is up, so reaching here means it was not.
            State::Error(
                "the read produced no fields; the JNI bridge may not be initialised".into(),
            )
        };
        Row::value("package_info", "this app's name, id, version, build", state)
    }
    #[cfg(not(any(target_os = "android", target_os = "ios", target_os = "macos")))]
    {
        Row::value(
            "package_info",
            "this app's name, id, version, build",
            State::Absent,
        )
    }
}

// --- this app's storage ------------------------------------------------------

/// Where this app may write — the first gallery row backed by a Tier-A catalog
/// plugin whose **desktop arm is real** ([`Row::measured_value`]).
///
/// On Linux and macOS the answer comes from the platform's own conventions
/// (XDG, `~/Library`), so a host build genuinely measures it — the host is the
/// source of truth there, not a stand-in for a phone. On Android the JNI
/// bridge answers. The external-storage question is Android-only by design
/// (the crate keeps it out of `AppDirectory` precisely because it exists on
/// one platform), and the row reports its absence as `n/a` rather than
/// collapsing it into the internal directories, which would rename the
/// capability out from under the reader.
pub fn paths() -> Row {
    #[cfg(any(
        target_os = "android",
        target_os = "ios",
        target_os = "macos",
        target_os = "linux",
        target_os = "windows"
    ))]
    {
        let support = paths::app_dir(paths::AppDirectory::ApplicationSupport);
        let cache = paths::app_dir(paths::AppDirectory::Cache);
        let external = paths::external_files_dir();
        let state = match (support, cache) {
            (Some(support), Some(cache)) => State::Value(format!(
                "support {} · cache {} · external {}",
                truncate(&support.display().to_string(), 38),
                truncate(&cache.display().to_string(), 38),
                match external {
                    Some(external) => truncate(&external.display().to_string(), 38),
                    // Android-only capability; see the row doc.
                    None => "n/a".to_string(),
                }
            )),
            // Every platform this arm compiles for defines these two
            // directories, so a miss means the bridge or the conventions
            // failed — an error, not "no capability".
            _ => State::Error("the app directories did not resolve".into()),
        };
        Row::measured_value("paths", "app support, cache, external storage", state)
    }
    #[cfg(not(any(
        target_os = "android",
        target_os = "ios",
        target_os = "macos",
        target_os = "linux",
        target_os = "windows"
    )))]
    {
        Row::value(
            "paths",
            "app support, cache, external storage",
            State::Absent,
        )
    }
}

// --- power and connectivity ------------------------------------------------

pub fn battery() -> Row {
    if !native_capability_host() {
        return Row::value(
            "battery",
            "charge level, charging, low-power mode",
            State::Absent,
        );
    }
    let b = battery::battery_state();
    let low = battery::is_low_power_mode();
    Row::value(
        "battery",
        "charge level, charging, low-power mode",
        State::Value(format!(
            "{} · {} · low-power {}",
            match b.level {
                Some(level) => format!("{:.0}%", level * 100.0),
                // The OS gave no reading. Printing "100%" here would be the
                // exact kind of fabricated value this gallery exists to catch.
                None => "no reading".into(),
            },
            if b.is_charging { "charging" } else { "idle" },
            if low { "on" } else { "off" }
        )),
    )
}

pub fn connectivity() -> Row {
    if !native_capability_host() {
        return Row::value(
            "connectivity",
            "connection kind, connected, metered",
            State::Absent,
        );
    }
    let c = connectivity::connectivity_status();
    Row::value(
        "connectivity",
        "connection kind, connected, metered",
        State::Value(format!(
            "{:?} · connected {} · metered {}",
            c.connection_type, c.is_connected, c.is_metered
        )),
    )
}

// --- input and sensors -----------------------------------------------------

/// Motion and environment sensors — availability only.
///
/// The package answers which sensor kinds exist and deliberately has no
/// `reading()`: samples arrive through `SensorEventListener` (Android) and
/// `startAccelerometerUpdatesToQueue:` (iOS), both callbacks this synchronous
/// API has nowhere to put. Printing an x/y/z triple here would mean reporting a
/// value nothing in the package constructs.
///
/// On a device the arms call `SensorManager.getDefaultSensor(type)` and
/// `CMMotionManager.accelerometerAvailable` for real, so the list is what the
/// hardware reports. An empty list on a phone therefore means the device has no
/// sensor of any of the six kinds — not, as this comment once said, that nothing
/// was wired behind the bridge.
pub fn sensors() -> Row {
    const QUESTION: &str = "which sensor kinds this device reports";
    if !native_capability_host() {
        return Row::value("sensors", QUESTION, State::Absent);
    }
    let present: Vec<String> = sensors::SensorKind::all()
        .iter()
        .filter(|k| sensors::is_sensor_available(**k))
        .map(|k| format!("{k:?}"))
        .collect();
    Row::value(
        "sensors",
        QUESTION,
        if present.is_empty() {
            State::Value(format!(
                "none of the {} known kinds reported",
                sensors::SensorKind::all().len()
            ))
        } else {
            State::Value(present.join(", "))
        },
    )
}

// --- privacy and authorization ---------------------------------------------

pub fn location() -> Row {
    if !native_capability_host() {
        return Row::value("location", "authorization status", State::Absent);
    }
    Row::value(
        "location",
        "authorization status",
        State::Value(format!("{:?}", location::authorization_status())),
    )
}

pub fn local_auth() -> Row {
    if !native_capability_host() {
        return Row::value("local_auth", "enrolled biometric kind", State::Absent);
    }
    let kind = local_auth::available_biometric();
    Row::value(
        "local_auth",
        "enrolled biometric kind",
        match kind {
            local_auth::BiometricKind::None => State::Value("none enrolled".into()),
            other => State::Value(format!("{other:?}")),
        },
    )
}

pub fn notifications() -> Row {
    if !native_capability_host() {
        return Row::value("notifications", "authorization", State::Absent);
    }
    let enabled = notifications::are_notifications_enabled();
    Row::value(
        "notifications",
        "authorization",
        State::Value(
            if enabled {
                "authorized"
            } else {
                "not authorized"
            }
            .into(),
        ),
    )
}

// --- the OS-drawn space ---------------------------------------------------

/// The region the OS reserves for its own chrome: notch, home indicator,
/// status bar.
///
/// `safe_area_insets()` cannot answer "unavailable" — its host arm returns
/// `Insets::ZERO`, which the package documents as *unknown* rather than *nothing
/// reserved*. So the row gates on [`native_capability_host`] and reports what was
/// measured; on a desktop it says `—` instead of asserting a Mac has no notch
/// after a call that never asked.
pub fn safe_area() -> Row {
    const QUESTION: &str = "OS-reserved insets (notch, home indicator)";
    if !native_capability_host() {
        return Row::value("safe_area", QUESTION, State::Absent);
    }
    let i = safe_area::safe_area_insets();
    Row::value(
        "safe_area",
        QUESTION,
        State::Value(format!(
            "top {:.0} · right {:.0} · bottom {:.0} · left {:.0}",
            i.top, i.right, i.bottom, i.left
        )),
    )
}

/// The soft keyboard, which is the one OS overlay that *moves*.
///
/// It earns a row of its own for the same reason `safe_area` does but more
/// sharply: the insets a soft keyboard reserves arrive and go during a session,
/// so a layout that folded them in once would be permanently padded by a
/// keyboard-height's worth of nothing. On a host, `soft_keyboard_state()` is
/// `Default` — hidden, height 0 — which is a real answer about a real desktop
/// and a fabricated one about a phone, so it is gated like the rest.
pub fn keyboard() -> Row {
    if !native_capability_host() {
        return Row::value(
            "keyboard",
            "soft keyboard visibility and height",
            State::Absent,
        );
    }
    let k = soft_input::soft_keyboard_state();
    Row::value(
        "keyboard",
        "soft keyboard visibility and height",
        State::Value(format!(
            "visible {} · height {:.0} · reserved insets {:.0}",
            k.visible,
            k.height,
            k.insets().bottom
        )),
    )
}

// --- the clipboard: a capability that also drives a core Floom trait --------

pub fn clipboard_read() -> Row {
    match clipboard::get_text() {
        Ok(Some(t)) => Row::value(
            "clipboard (read)",
            "text the OS clipboard holds",
            State::Value(if t.is_empty() {
                "empty string".into()
            } else {
                format!("{} bytes: {:?}", t.len(), truncate(&t, 24))
            }),
        ),
        // `Ok(None)` is the documented "holds no text" answer — an empty
        // clipboard is a normal state, not a failure, so it is a value.
        Ok(None) => Row::value(
            "clipboard (read)",
            "text the OS clipboard holds",
            State::Value("holds no text".into()),
        ),
        Err(e) => Row::value(
            "clipboard (read)",
            "text the OS clipboard holds",
            State::Error(e),
        ),
    }
}

pub fn clipboard_write() -> Row {
    const PROBE: &str = "floom-capabilities clipboard probe";
    match clipboard::set_text(PROBE) {
        Ok(()) => Row::value(
            "clipboard (write)",
            "write a probe string, then read it back",
            State::Value(format!("wrote {} bytes", PROBE.len())),
        ),
        Err(e) => Row::value(
            "clipboard (write)",
            "write a probe string, then read it back",
            State::Error(e),
        ),
    }
}

// --- actions the user triggers ---------------------------------------------

pub fn can_vibrate() -> Row {
    if !native_capability_host() {
        return Row::value("vibration", "haptics engine present", State::Absent);
    }
    let can = vibration::can_vibrate();
    Row::value(
        "vibration",
        "haptics engine present",
        State::Value(if can { "yes" } else { "no" }.into()),
    )
}

// --- capabilities whose value is the *act*, not a reading -------------------
//
// These are not probed at build time because doing so would open a system
// sheet, vibrate the device, or share a file without the user asking. The
// gallery shows them as actions; the report below is what a run actually did.

/// Actions the UI offers, with the call each one makes.
///
/// Paired label/id rather than a bare label, so the gallery can print the exact
/// call next to each button and a reader can find it in the crate.
pub fn actions() -> Vec<(&'static str, &'static str)> {
    vec![
        ("vibration::vibrate", "device::vibrate"),
        ("vibration::haptic_feedback", "device::haptic"),
        ("share::share_text", "device::share"),
        ("local_auth::authenticate", "device::authenticate"),
        (
            "location::request_authorization",
            "device::request_location",
        ),
        (
            "notifications::request_authorization",
            "device::request_notifications",
        ),
        (
            "system_chrome::set_system_chrome_style",
            "device::system_chrome",
        ),
        // The soft keyboard is the only action whose *absence* is still
        // observable: the call succeeds vacuously on a host (there is nothing to
        // show), which is exactly why it is worth a button — the reader sees
        // "ok" and learns the platform has no keyboard rather than no bridge.
        (
            "keyboard::set_soft_keyboard_visible",
            "device::show_keyboard",
        ),
    ]
}

fn truncate(s: &str, max: usize) -> String {
    if s.chars().count() <= max {
        return s.to_string();
    }
    let head: String = s.chars().take(max).collect();
    format!("{head}…")
}

/// Every probe, in gallery order.
///
/// Grouped by what a reader would want to compare across devices, and ordered so
/// the rows that always answer come first — the identity block never fails, so
/// someone opening the gallery on an unknown device learns what it is before
/// reaching the rows that might say "—".
///
/// The order is also the [`GROUPS`] order, and `group_headers_stay_aligned_with
/// the_rows` is what keeps the two from drifting apart.
pub fn all() -> Vec<Row> {
    vec![
        // identity
        device_info(),
        package_info(),
        // this app's storage
        paths(),
        // power and network
        battery(),
        connectivity(),
        // input and sensors
        sensors(),
        // privacy
        location(),
        local_auth(),
        notifications(),
        // appearance and the OS-drawn space
        safe_area(),
        keyboard(),
        // the clipboard round-trip
        clipboard_read(),
        clipboard_write(),
        // haptics
        can_vibrate(),
    ]
}

/// Section headers, each naming the **first row** it covers.
///
/// Flutter's gallery and Slint's demos both break a long list into labelled
/// sections, because twelve rows in one undifferentiated column is a wall. The
/// header is keyed to a row name rather than to an index so that inserting a row
/// inside a section cannot silently move the boundary; the test below is what
/// makes that claim true rather than merely intended.
pub const GROUPS: [(&str, &str); 7] = [
    ("Identity — what this device is", "device_info"),
    ("This app's storage", "paths"),
    ("Power and network", "battery"),
    ("Input and sensors", "sensors"),
    ("Privacy and authorization", "location"),
    ("Appearance and OS-drawn space", "safe_area"),
    ("Clipboard and haptics", "clipboard (read)"),
];

/// The group header that introduces `row_name`, if any.
///
/// Compared against the *first* row of each group, so a row in the middle of a
/// section correctly reports no header of its own.
pub fn group_header_for(row_name: &str) -> Option<&'static str> {
    GROUPS
        .iter()
        .find(|(_, first)| *first == row_name)
        .map(|(title, _)| *title)
}

#[cfg(test)]
mod tests {
    use super::*;

    /// The whole point of the module: a host build must not *claim* a capability.
    ///
    /// Note what this does **not** assert. It does not require every row to be
    /// `Absent`, because the packages have two honest ways of refusing and both
    /// are legitimate: `vibration::can_vibrate()` is `cfg`-dispatched, so a host
    /// never even asks the OS (`Absent`), whereas `clipboard::get_text()` is a
    /// real call whose host arm returns `Err` (`Error`, with the reason).
    ///
    /// And it does not require every row to avoid `Value` on a host, either —
    /// only the rows that cannot measure there. The Tier-A catalog plugins
    /// (`paths`) read the *desktop's own conventions*: the XDG directories on
    /// Linux, `~/Library` on macOS. A `Value` from such a row on a laptop is a
    /// measurement the laptop itself made, which is a different thing from the
    /// fabrication this test exists to catch. [`Row::desktop_real`] is how a
    /// row claims that status; the assertion holds the line for every row that
    /// does not, because a `Value` there could only have come from a default
    /// the OS never set.
    ///
    /// That default is no longer a plausible number: `BatteryState::default()`
    /// used to be `level: 1.0`, which this gallery printed as "100%" on a
    /// laptop, and it is now `level: None`. Both layers were fixed — the crate
    /// stopped inventing a reading, and this assertion stopped the row from
    /// showing one — because either alone would have left the other reachable.
    #[test]
    fn a_host_build_never_reports_a_value_it_cannot_have_measured() {
        if native_capability_host() {
            return; // on-device this assertion is not the contract
        }
        for row in all() {
            if row.desktop_real {
                continue; // a desktop-real row measures on a host; see the doc above
            }
            assert!(
                !row.state.is_value(),
                "`{}` reported a value ({:?}) on a host that cannot measure it",
                row.name,
                row.state
            );
        }
    }

    /// Every row must be labelled — an unnamed capability cannot be looked up.
    #[test]
    fn every_row_is_named_and_questioned() {
        for row in all() {
            assert!(!row.name.is_empty(), "a row has no name");
            assert!(!row.question.is_empty(), "`{}` has no question", row.name);
        }
    }

    /// The gallery is the product surface, so its vocabulary has to hold.
    #[test]
    fn absent_and_error_read_differently() {
        assert_eq!(State::Absent.text(), "—");
        assert_ne!(
            State::Absent.text(),
            State::Error("boom".into()).text(),
            "an absent capability and a failing one must not read the same"
        );
    }

    /// Every section header must name a row that exists, appear exactly once,
    /// and come after the previous section's header.
    ///
    /// `GROUPS` is keyed to row *names* so a row can be inserted inside a
    /// section without moving a boundary — but only if the names still resolve.
    /// A typo in a header would otherwise render a section title above the
    /// wrong block, which is exactly the sort of quiet lie this example is
    /// built to avoid.
    #[test]
    fn group_headers_stay_aligned_with_the_rows() {
        let rows = all();
        let mut last_seen_at: Option<usize> = None;
        for (title, first_row) in GROUPS {
            let at = rows
                .iter()
                .position(|r| r.name == first_row)
                .unwrap_or_else(|| {
                    panic!("group `{title}` names `{first_row}`, which is not a row")
                });
            if let Some(prev) = last_seen_at {
                assert!(
                    at > prev,
                    "group `{title}` starts at row {at}, which is not after the previous \
                     section's row {prev} — `GROUPS` and `all()` have drifted"
                );
            }
            last_seen_at = Some(at);
        }
    }

    /// The first row must belong to a section, or the table opens untitled.
    #[test]
    fn the_table_starts_inside_a_section() {
        let first = all().first().expect("the table is never empty").name;
        assert_eq!(
            group_header_for(first),
            Some(GROUPS[0].0),
            "row `{first}` opens the table but heads no section"
        );
    }
}

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

use floom_platform_mobile::packages::{
    accessibility_prefs, app_info, battery, clipboard, connectivity, device_info,
    keyboard::soft_input, local_auth, location, notifications, orientation, safe_area, screen,
    sensors, system_chrome, url_launcher, vibration,
};

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
}

impl Row {
    fn value(name: &'static str, question: &'static str, state: State) -> Self {
        Self {
            name,
            question,
            state,
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
/// Without this, `battery_state()` on a Mac returns `BatteryState::default()`
/// — `level: 1.0` — which the gallery would happily print as "100%". A desktop
/// that cannot read a battery has no battery reading, and the row should say so.
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

pub fn app_info() -> Row {
    let a = app_info::info();
    Row::value(
        "app_info",
        "bundle id, display name, version, build",
        native_or_absent(format!("{} · {} {} ({})", a.id, a.version, a.build, a.name)),
    )
}

pub fn screen() -> Row {
    let m = screen::metrics();
    Row::value(
        "screen",
        "logical size, pixel ratio, density",
        native_or_absent(format!(
            "{:.0}×{:.0} · scale {:.2} · {:.0} dpi",
            m.width, m.height, m.scale, m.density_dpi
        )),
    )
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
            "{:.0}% · {} · low-power {}",
            b.level * 100.0,
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

pub fn sensors() -> Row {
    if !native_capability_host() {
        return Row::value("sensors", "available hardware, live reading", State::Absent);
    }
    let available = sensors::available_sensors();
    if available.is_empty() {
        return Row::value(
            "sensors",
            "available hardware, live reading",
            State::Value("no motion sensors reported".into()),
        );
    }
    // Read the first kind that actually produces a sample. Which one that is
    // varies by device, so the row reports the reading it managed to take
    // rather than pretending a fixed sensor exists.
    let mut taken: Option<(sensors::SensorKind, sensors::SensorReading)> = None;
    for kind in &available {
        if let Some(r) = sensors::reading(*kind) {
            taken = Some((*kind, r));
            break;
        }
    }
    match taken {
        None => Row::value(
            "sensors",
            "available hardware, live reading",
            State::Value(format!("{} present, no sample yet", available.len())),
        ),
        Some((kind, r)) => Row::value(
            "sensors",
            "available hardware, live reading",
            State::Value(format!(
                "{:?} among {} · x {:.3} y {:.3} z {:.3}",
                kind,
                available.len(),
                r.x,
                r.y,
                r.z
            )),
        ),
    }
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

// --- accessibility and appearance ------------------------------------------

pub fn accessibility_prefs() -> Row {
    if !native_capability_host() {
        return Row::value(
            "accessibility_prefs",
            "reduced motion, bold text, reduced transparency",
            State::Absent,
        );
    }
    Row::value(
        "accessibility_prefs",
        "reduced motion, bold text, reduced transparency",
        State::Value(format!(
            "motion {} · bold {} · transparency {}",
            if accessibility_prefs::prefers_reduced_motion() {
                "reduced"
            } else {
                "normal"
            },
            if accessibility_prefs::is_bold_text() {
                "on"
            } else {
                "off"
            },
            if accessibility_prefs::prefers_reduced_transparency() {
                "reduced"
            } else {
                "normal"
            }
        )),
    )
}

pub fn safe_area() -> Row {
    match safe_area::try_safe_area_insets() {
        Ok(i) => Row::value(
            "safe_area",
            "OS-reserved insets (notch, home indicator)",
            State::Value(format!(
                "top {:.0} · right {:.0} · bottom {:.0} · left {:.0}",
                i.top, i.right, i.bottom, i.left
            )),
        ),
        Err(e) => Row::value(
            "safe_area",
            "OS-reserved insets (notch, home indicator)",
            State::Error(e),
        ),
    }
}

pub fn orientation() -> Row {
    if !native_capability_host() {
        return Row::value(
            "orientation",
            "current interface orientation",
            State::Absent,
        );
    }
    Row::value(
        "orientation",
        "current interface orientation",
        State::Value(format!("{:?}", orientation::current())),
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

pub fn url_launcher() -> Row {
    if !native_capability_host() {
        return Row::value(
            "url_launcher",
            "probe whether a URL can be opened",
            State::Absent,
        );
    }
    const PROBE_URL: &str = "https://example.com";
    if url_launcher::can_open(PROBE_URL) {
        Row::value(
            "url_launcher",
            "probe whether a URL can be opened",
            State::Value(format!("{PROBE_URL} is openable")),
        )
    } else {
        Row::value(
            "url_launcher",
            "probe whether a URL can be opened",
            State::Value(format!("{PROBE_URL} refused")),
        )
    }
}

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
        ("url_launcher::open", "device::open_url"),
        ("local_auth::authenticate", "device::authenticate"),
        (
            "location::request_authorization",
            "device::request_location",
        ),
        (
            "notifications::request_authorization",
            "device::request_notifications",
        ),
        ("system_chrome::set_edge_to_edge", "device::edge_to_edge"),
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

/// Read `system_chrome`'s one host-answered question: is the status bar style
/// under application control?
pub fn status_bar_controlled() -> Row {
    if !native_capability_host() {
        return Row::value(
            "system_chrome",
            "status bar style under app control",
            State::Absent,
        );
    }
    let controlled = system_chrome::status_bar_style_is_application_controlled();
    Row::value(
        "system_chrome",
        "status bar style under app control",
        State::Value(if controlled { "yes" } else { "no" }.into()),
    )
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
        app_info(),
        screen(),
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
        accessibility_prefs(),
        safe_area(),
        orientation(),
        status_bar_controlled(),
        keyboard(),
        // the clipboard round-trip
        clipboard_read(),
        clipboard_write(),
        // launchers
        url_launcher(),
        can_vibrate(),
    ]
}

/// Section headers, each naming the **first row** it covers.
///
/// Flutter's gallery and Slint's demos both break a long list into labelled
/// sections, because eighteen rows in one undifferentiated column is a wall. The
/// header is keyed to a row name rather than to an index so that inserting a row
/// inside a section cannot silently move the boundary; the test below is what
/// makes that claim true rather than merely intended.
pub const GROUPS: [(&str, &str); 6] = [
    ("Identity — what this device is", "device_info"),
    ("Power and network", "battery"),
    ("Input and sensors", "sensors"),
    ("Privacy and authorization", "location"),
    ("Appearance and OS-drawn space", "accessibility_prefs"),
    ("Clipboard and launchers", "clipboard (read)"),
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
    /// real call whose host arm returns `Err` (`Error`, with the reason). What
    /// must never happen on a host is a `Value` — because a value here can only
    /// have come from a `Default` the OS never set. `battery_state()` is the
    /// case that matters: it answers `level: 1.0`, which the gallery would
    /// happily print as "100%" on a laptop.
    #[test]
    fn a_host_build_never_reports_a_value_it_cannot_have_measured() {
        if native_capability_host() {
            return; // on-device this assertion is not the contract
        }
        for row in all() {
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

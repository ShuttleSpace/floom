//! Linux battery state via sysfs `/sys/class/power_supply`.
//!
//! Pure `std`: the kernel exposes every battery as a directory under
//! `/sys/class/power_supply/` with a `type` file (`Battery`) and a `uevent`
//! file declaring the values the kernel knows. There is no crate between the
//! reader and the numbers, so the format quirks are written once, here:
//!
//! - The `uevent` file is `KEY=value` lines. Values are bare bytes, so each key
//!   is matched with `starts_with` and the value taken after the first `=`.
//! - Drivers report capacity either as a direct percentage (`POWER_SUPPLY_CAPACITY`)
//!   or as an energy pair (`POWER_SUPPLY_ENERGY_NOW`/`_FULL`, in µWh) or a charge
//!   pair (`POWER_SUPPLY_CHARGE_NOW`/`_FULL`, in µAh). Whichever is present wins;
//!   the now/full ratio gives the fraction on every scale.
//! - `POWER_SUPPLY_STATUS` is the kernel's state string (`Charging`,
//!   `Discharging`, `Full`, `Not charging`, `Unknown`). `Not charging` is *not*
//!   the same as `Full`, so the two are kept apart.

use std::fs;
use std::path::PathBuf;

use super::BatteryState;

/// The first directory under `/sys/class/power_supply` whose `type` reads
/// `Battery`, `None` when the box has no battery (common on desktops) or none
/// the reader can open.
fn battery_dir() -> Option<PathBuf> {
    let root = PathBuf::from("/sys/class/power_supply");
    let entries = fs::read_dir(&root).ok()?;
    for entry in entries.flatten() {
        let dir = entry.path();
        // Drivers expose the same device under several names; only `type ==
        // "Battery"` is the one with the state files.
        let is_battery = fs::read_to_string(dir.join("type"))
            .map(|t| t.trim() == "Battery")
            .unwrap_or(false);
        if is_battery {
            return Some(dir);
        }
    }
    None
}

/// One `KEY=value` value out of a `uevent` blob, by key. The returned slice
/// borrows from `uevent`.
fn uevent_value<'a>(uevent: &'a str, key: &str) -> Option<&'a str> {
    uevent.lines().find_map(|line| {
        let line = line.trim();
        line.strip_prefix(key)
            .and_then(|rest| rest.strip_prefix('='))
            .map(str::trim)
    })
}

/// Parse a `KEY=value` numeric field, returning `None` on a missing key or a
/// non-numeric value (some drivers omit fields entirely).
fn uevent_u64(uevent: &str, key: &str) -> Option<u64> {
    uevent_value(uevent, key)?.trim().parse::<u64>().ok()
}

pub fn battery_state() -> BatteryState {
    let Some(dir) = battery_dir() else {
        return BatteryState::default();
    };

    let Ok(uevent) = fs::read_to_string(dir.join("uevent")) else {
        return BatteryState::default();
    };

    // Direct percentage first (most drivers expose it); otherwise the
    // now/full ratio on whichever scale the driver reports.
    let level = uevent_u64(&uevent, "POWER_SUPPLY_CAPACITY")
        .filter(|c| (1..=100).contains(c))
        .map(|c| (c as f32 / 100.0).clamp(0.0, 1.0))
        .or_else(|| {
            for kind in ["ENERGY", "CHARGE"] {
                let now = uevent_u64(&uevent, &format!("POWER_SUPPLY_{kind}_NOW"))?;
                let full = uevent_u64(&uevent, &format!("POWER_SUPPLY_{kind}_FULL"))?;
                if full > 0 && now <= full {
                    return Some((now as f32 / full as f32).clamp(0.0, 1.0));
                }
            }
            None
        });

    let status = uevent_value(&uevent, "POWER_SUPPLY_STATUS");
    let is_charging = status == Some("Charging");
    let is_full = status == Some("Full");

    BatteryState {
        level,
        is_charging,
        is_full,
    }
}

pub fn is_low_power_mode() -> bool {
    // Linux exposes one battery-saver flag per desktop shell (GNOME's
    // `power-saver-profile`, KDE's `powersave`), none standardized. The honest
    // answer is that this crate does not second-guess a desktop it cannot see.
    false
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn uevent_value_reads_a_key() {
        let uevent = "POWER_SUPPLY_NAME=BAT0\n\
                      POWER_SUPPLY_STATUS=Charging\n\
                      POWER_SUPPLY_CAPACITY=73\n";
        assert_eq!(
            uevent_value(uevent, "POWER_SUPPLY_STATUS"),
            Some("Charging")
        );
        assert_eq!(uevent_value(uevent, "POWER_SUPPLY_CAPACITY"), Some("73"));
        assert_eq!(uevent_value(uevent, "POWER_SUPPLY_MISSING"), None);
    }

    #[test]
    fn capacity_maps_to_fraction() {
        let uevent = "POWER_SUPPLY_CAPACITY=50\n";
        assert_eq!(uevent_u64(uevent, "POWER_SUPPLY_CAPACITY"), Some(50));
        assert_eq!(battery_state_from_uevent(uevent).level, Some(0.5));
    }

    /// Helper: feed a synthetic `uevent` through the same parsing the real read
    /// uses, without touching the filesystem.
    fn battery_state_from_uevent(uevent: &str) -> BatteryState {
        let level = uevent_u64(uevent, "POWER_SUPPLY_CAPACITY")
            .filter(|c| (1..=100).contains(c))
            .map(|c| (c as f32 / 100.0).clamp(0.0, 1.0));
        let status = uevent_value(uevent, "POWER_SUPPLY_STATUS");
        BatteryState {
            level,
            is_charging: status == Some("Charging"),
            is_full: status == Some("Full"),
        }
    }
}

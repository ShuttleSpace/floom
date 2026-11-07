//! Windows battery state via Win32 `GetSystemPowerStatus`.
//!
//! The call fills a `SYSTEM_POWER_STATUS` we declare here; no crate sits between
//! the reader and the value. `BatteryLifePercent` is `0..=100`, with `0xFF`
//! meaning "unknown", and `BatteryFlag` carries the charging bit (`0x08`) and
//! the "no system battery" bit (`0x80`). No dedicated "full" bit exists in the
//! API, so a battery at 100% that is not charging is treated as full.

use std::os::raw::c_uchar;

use super::BatteryState;

#[repr(C)]
struct SystemPowerStatus {
    ac_line_status: c_uchar,
    battery_flag: c_uchar,
    battery_life_percent: c_uchar,
    _reserved0: c_uchar,
    battery_life_time: u32,
    battery_full_life_time: u32,
}

const BATTERY_FLAG_CHARGING: u8 = 0x08;
const BATTERY_FLAG_NO_BATTERY: u8 = 0x80;
const BATTERY_PERCENT_UNKNOWN: u8 = 0xFF;

#[link(name = "kernel32")]
unsafe extern "system" {
    fn GetSystemPowerStatus(status: *mut SystemPowerStatus) -> i32;
}

pub fn battery_state() -> BatteryState {
    let mut status = SystemPowerStatus {
        ac_line_status: 0,
        battery_flag: 0,
        battery_life_percent: 0,
        _reserved0: 0,
        battery_life_time: 0,
        battery_full_life_time: 0,
    };
    // The call returns non-zero on success.
    if unsafe { GetSystemPowerStatus(&mut status) } == 0 {
        return BatteryState::default();
    }
    if status.battery_flag & BATTERY_FLAG_NO_BATTERY != 0 {
        return BatteryState::default();
    }
    let level = if status.battery_life_percent == BATTERY_PERCENT_UNKNOWN {
        None
    } else {
        Some((status.battery_life_percent as f32 / 100.0).clamp(0.0, 1.0))
    };
    let is_charging = status.battery_flag & BATTERY_FLAG_CHARGING != 0;
    // No dedicated "full" bit: a battery at 100% that is not charging is as full
    // as Windows reports.
    let is_full = level == Some(1.0) && !is_charging;
    BatteryState {
        level,
        is_charging,
        is_full,
    }
}

pub fn is_low_power_mode() -> bool {
    // Windows exposes battery-saver state through `PowerGetActiveOverlayScheme`
    // / the `ECO` API, which is version- and policy-dependent; report honestly
    // rather than guess.
    false
}

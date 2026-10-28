//! iOS sensor availability via `CoreMotion` and `UIDevice`.
//!
//! Adapted from `gpui-mobile` (Apache-2.0).
//!
//! # What iOS can and cannot answer
//!
//! `CMMotionManager` carries a boolean per sensor family — `accelerometerAvailable`,
//! `gyroAvailable`, `magnetometerAvailable` — and each is the platform's own
//! statement that the hardware exists. `CMAltimeter` answers the barometer the
//! same way.
//!
//! Two kinds have no honest answer on iOS:
//!
//! * **Proximity** — `UIDevice.proximityMonitoringEnabled` only *arms* the
//!   sensor; there is no "does this device have one" query. Turning monitoring
//!   on to find out would be a side effect on behalf of a caller that only asked
//!   a question, so this reports `false`.
//! * **Ambient light** — iOS exposes no ambient light sensor to third-party
//!   apps at all (the camera's auto-exposure reads it, the app cannot), so this
//!   is `false` on every device.
//!
//! Reporting `false` for both is not a stub: it is what the platform supports,
//! and it is distinguishable from a broken bridge because the four kinds that
//! *do* have an API report their real value.
//!
//! # Why `msg_send!` and not `objc2-core-motion`
//!
//! There is no `objc2-core-motion` crate in the workspace's dependency set, and
//! adding one for four boolean properties would pull a binding for the whole
//! framework. The four selectors used here are stable since iOS 4 and are
//! written as literals, so the usual reason to want generated bindings —
//! checking selector spelling — is covered by the compiler.

use super::SensorKind;
use objc2::runtime::AnyObject;
use objc2::{class, msg_send};

// `class!(CMMotionManager)` resolves through `objc_getClass`, which only finds
// the class if the framework is loaded — so the plugin links CoreMotion even
// though it calls no C function from it.
#[link(name = "CoreMotion", kind = "framework")]
unsafe extern "C" {}

pub fn available_sensors() -> Vec<SensorKind> {
    SensorKind::all()
        .iter()
        .copied()
        .filter(|kind| is_sensor_available(*kind))
        .collect()
}

pub fn is_sensor_available(kind: SensorKind) -> bool {
    match kind {
        SensorKind::Accelerometer => unsafe {
            let available: bool = msg_send![motion_manager(), accelerometerAvailable];
            available
        },
        SensorKind::Gyroscope => unsafe {
            let available: bool = msg_send![motion_manager(), gyroAvailable];
            available
        },
        SensorKind::Magnetometer => unsafe {
            let available: bool = msg_send![motion_manager(), magnetometerAvailable];
            available
        },
        SensorKind::Barometer => unsafe {
            let available: bool = msg_send![class!(CMAltimeter), isRelativeAltitudeAvailable];
            available
        },
        // See the module note: neither has a query that answers this question.
        SensorKind::Proximity | SensorKind::AmbientLight => false,
    }
}

/// `CMMotionManager.sharedManager`.
///
/// The singleton is the documented entry point; a fresh `CMMotionManager` per
/// call would allocate a sensor pipeline just to read a flag. It is autoreleased
/// and owned by CoreMotion, so nothing here releases it.
fn motion_manager() -> *mut AnyObject {
    unsafe { msg_send![class!(CMMotionManager), sharedManager] }
}

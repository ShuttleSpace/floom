//! Mobile capability packages.
//!
//! Each submodule is a capability that follows the `mod.rs` + `ios.rs` +
//! `android.rs` dispatch convention documented in the crate root. A capability
//! that is unsupported on the current target returns `Err`/`false`/`ZERO` from
//! its host arm, so the whole crate compiles (and is inert) on desktop/web.
//!
//! ## Implemented
//!
//! * [`vibration`] — haptics and device vibration. Both backends wired
//!   (iOS `UIKit`/`AudioToolbox`, Android `Vibrator`/`performHapticFeedback`).
//! * [`keyboard`] — Android NDK `AKeyEvent` → Floem `Key` (pure, always
//!   compiled — see [`keyboard::android_table`]) plus soft-keyboard
//!   visibility/height with hysteresis ([`keyboard::soft_input`]).
//! * [`safe_area`] — OS-reserved insets as `floem_types::Insets`.
//! * [`system_chrome`] — status/navigation bar appearance (`SystemChromeStyle`).
//! * [`device_info`], [`battery`], [`connectivity`], [`location`],
//!   [`sensors`], [`local_auth`], [`notifications`], [`share`] — typed
//!   capability APIs with a `#[cfg]`-dispatched public surface. The data types
//!   and getters are host-inert today; the native iOS/Android arms are stubs
//!   that need the real platform backend wired (see each package's
//!   `android.rs` / `ios.rs` for the TODO and the `gpui-mobile` class names).
//!
//! ## Reference
//!
//! The per-package iOS/Android class names come from the upstream
//! `gpui-mobile` `PACKAGES-TO-IMPL.md`. `camera` (iOS `AVFoundation`,
//! Android `Camera2`/`CameraX`) is intentionally deferred — it is a streaming
//! media surface, not a single-shot device query, and belongs behind its own
//! feature gate.

pub mod battery;
pub mod connectivity;
pub mod device_info;
pub mod keyboard;
pub mod local_auth;
pub mod location;
pub mod notifications;
pub mod safe_area;
pub mod sensors;
pub mod share;
pub mod system_chrome;
pub mod vibration;

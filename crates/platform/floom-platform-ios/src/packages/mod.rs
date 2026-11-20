// SPDX-License-Identifier: MIT
//! The mobile capabilities that belong to the **platform**, not to a plugin.
//!
//! The packages here are the ones a window cannot be built without, or that
//! are facts about the window rather than capabilities a plugin provides:
//!
//! * [`clipboard`] — the system clipboard. One of the three traits
//!   `floom-platform` defines, so it is the platform's job by definition.
//! * [`keyboard`] — the soft keyboard (the IME), and the hardware-key
//!   translation table. The IME is the third of those three traits; the
//!   hardware table is input translation, which is windowing work.
//! * [`safe_area`] — the OS-reserved insets of the window: a notch, a home
//!   indicator. A fact about the window, so the platform knows it.
//! * [`system_chrome`] — the status and navigation bars' appearance. Also
//!   the window's.
//!
//! # What is not here
//!
//! Every other mobile capability — battery, connectivity, identity, biometrics,
//! location, notifications, sensors, share sheet, haptics — is a **plugin**, not
//! platform code. The split and the architecture-gate rule that enforces it are
//! in `docs/api/mobile-platform-layering.md`.

pub mod clipboard;
pub mod keyboard;
pub mod safe_area;
pub mod system_chrome;

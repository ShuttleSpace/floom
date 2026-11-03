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
//! Everything else — the battery, the connectivity, the device's identity,
//! the biometrics, the location, the notifications, the sensors, the share
//! sheet, the haptics — is a **capability**, and a capability is a plugin:
//! one crate, opt-in by dependency, reaching the OS through a native host
//! library. They live under `crates/floom-plugin-<capability>/`.
//!
//! The split is not cosmetic. A capability in this crate would be linked into
//! every mobile app that depends on the platform adapter, whether it wanted
//! the capability or not; a capability in its own crate is linked only by the
//! apps that ask for it. That is the whole of "the SDK stays small".
//!
//! The rule is enforced, not merely documented: the architecture gate refuses
//! a dependency from a `floom-platform-*` crate to a `floom-plugin-*` crate,
//! because the platform reaching *up* to a plugin is the direction that would
//! put the capability back in.

pub mod clipboard;
pub mod keyboard;
pub mod safe_area;
pub mod system_chrome;

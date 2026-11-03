//! Every target without a tray: mobile, and anything else.
//!
//! A phone has no tray. Android's status bar is the *system's*, and an app
//! cannot put an icon in it; iOS's status bar is not interactive at all. The
//! way a mobile app reaches a user who is not looking at it is a notification,
//! which is a different plugin with different permission. Saying so is the
//! honest arm — an `Ok` here would claim an icon nobody can see.

pub struct Tray;

impl Tray {
    pub fn show(_label: &str, _items: &[crate::MenuItem]) -> Result<Self, String> {
        Err(message().into())
    }

    pub fn events(&mut self) -> Vec<crate::TrayEvent> {
        Vec::new()
    }
}

#[cfg(any(target_os = "android", target_os = "ios"))]
fn message() -> &'static str {
    "tray: there is no system tray on mobile — Android's status bar belongs to \
     the system and iOS's is not interactive, so an app there reaches the user \
     who is not looking at it with a notification, not an icon"
}

#[cfg(not(any(target_os = "android", target_os = "ios")))]
fn message() -> &'static str {
    "tray: this target has no system tray"
}

//! OpenHarmony has no system tray.

pub struct Tray;

impl Tray {
    pub fn show(_label: &str, _items: &[crate::MenuItem]) -> Result<Self, String> {
        Err(
            "tray: OpenHarmony has no system tray — an app reaches a user who is \
             not looking at it with a notification, not an icon"
                .into(),
        )
    }

    pub fn events(&mut self) -> Vec<crate::TrayEvent> {
        Vec::new()
    }
}

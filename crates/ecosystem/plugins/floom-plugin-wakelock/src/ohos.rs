//! OpenHarmony has no keep-display-awake bridge in this crate yet.

pub fn keep_screen_on(on: bool) -> Result<(), String> {
    if !on {
        return Ok(());
    }
    Err("keep_screen_on: OpenHarmony has no keep-display-awake bridge in this crate yet".into())
}

pub fn is_keep_screen_on() -> bool {
    false
}

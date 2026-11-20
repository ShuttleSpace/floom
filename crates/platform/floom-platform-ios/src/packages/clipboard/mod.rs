// SPDX-License-Identifier: MIT
//! Clipboard — read and write the system clipboard on iOS via `UIPasteboard`.

#[cfg(target_os = "ios")]
mod ios;

pub fn get_text() -> Result<Option<String>, String> {
    #[cfg(target_os = "ios")]
    {
        ios::get_text()
    }
    #[cfg(not(target_os = "ios"))]
    {
        Err("clipboard is only available on iOS".into())
    }
}

pub fn set_text(text: &str) -> Result<(), String> {
    #[cfg(target_os = "ios")]
    {
        ios::set_text(text)
    }
    #[cfg(not(target_os = "ios"))]
    {
        let _ = text;
        Err("clipboard is only available on iOS".into())
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn host_or_non_ios_refused_when_built_elsewhere() {
        // This test runs on host; behavior is "refused" there.
        if cfg!(target_os = "ios") {
            return;
        }
        assert!(get_text().is_err());
    }
}

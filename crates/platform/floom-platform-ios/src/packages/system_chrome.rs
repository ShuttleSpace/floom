// SPDX-License-Identifier: MIT
//! iOS system chrome styling (stub — requires UIViewController context).

#[cfg(target_os = "ios")]
mod ios;

pub use floom_platform::mobile::{BarIconBrightness, SystemChromeStyle};

pub fn set_system_chrome_style(style: SystemChromeStyle) -> Result<(), String> {
    #[cfg(target_os = "ios")]
    {
        ios::set_system_chrome_style(style)
    }
    #[cfg(not(target_os = "ios"))]
    {
        let _ = style;
        Err("system chrome styling is only available on iOS and Android".into())
    }
}

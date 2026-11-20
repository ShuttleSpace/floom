// SPDX-License-Identifier: MIT
//! iOS system-bar styling via `UIApplication` / `UINavigationBar`.
//!
//! UIKit has no "status bar background color" property on modern iOS (the bar
//! is transparent over the app's content), so `status_bar_color` is applied to
//! the navigation bar's tint instead when one is present; the icon brightness
//! goes through `setStatusBarStyle:animated:`. The generated host plist sets
//! `UIViewControllerBasedStatusBarAppearance = NO`, which is what makes the
//! application-level style call effective.

use objc2::msg_send;
use objc2::runtime::AnyObject;

use super::{BarIconBrightness, SystemChromeStyle};

/// `UIStatusBarStyle`: `.darkContent` = 3 (dark icons on a light bar),
/// `.lightContent` = 1 (light icons on a dark bar).
const STATUS_STYLE_DARK_CONTENT: isize = 3;
const STATUS_STYLE_LIGHT_CONTENT: isize = 1;

pub fn set_system_chrome_style(style: SystemChromeStyle) -> Result<(), String> {
    unsafe {
        let app: *mut AnyObject = msg_send![objc2::class!(UIApplication), sharedApplication];
        if app.is_null() {
            return Err("UIApplication.sharedApplication was null".into());
        }
        let status_style = match style.status_bar_icon_brightness {
            BarIconBrightness::Dark => STATUS_STYLE_DARK_CONTENT,
            BarIconBrightness::Light => STATUS_STYLE_LIGHT_CONTENT,
        };
        let _: () = msg_send![app, setStatusBarStyle: status_style, animated: true];

        // UIKit has no status-bar background color; the closest host-visible
        // surface is the navigation bar's tint, used when the caller supplies
        // a color. Which of the two colors wins is the host's layout decision.
        if let Some(color) = style.status_bar_color.or(style.navigation_bar_color) {
            let nav: *mut AnyObject = msg_send![objc2::class!(UINavigationBar), appearance];
            if !nav.is_null() {
                let ui_color: *mut AnyObject = msg_send![
                    objc2::class!(UIColor),
                    colorWithRed: f64::from(color.r) / 255.0,
                             green: f64::from(color.g) / 255.0,
                              blue: f64::from(color.b) / 255.0,
                             alpha: f64::from(color.a) / 255.0
                ];
                if !ui_color.is_null() {
                    let _: () = msg_send![nav, setBarTintColor: ui_color];
                }
            }
        }
    }
    Ok(())
}

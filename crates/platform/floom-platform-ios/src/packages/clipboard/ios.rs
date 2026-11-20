// SPDX-License-Identifier: MIT
//! iOS clipboard via `UIPasteboard.general`.

use objc2::msg_send;
use objc2::runtime::AnyObject;

pub fn get_text() -> Result<Option<String>, String> {
    unsafe {
        let pb: *mut AnyObject = msg_send![objc2::class!(UIPasteboard), generalPasteboard];
        if pb.is_null() {
            return Ok(None);
        }
        let s: *mut AnyObject = msg_send![pb, string];
        if s.is_null() {
            return Ok(None);
        }
        // Read as a C string view via UTF8String; copies into Rust String.
        let cstr: *const std::ffi::c_char = msg_send![s, UTF8String];
        if cstr.is_null() {
            return Ok(None);
        }
        Ok(Some(
            std::ffi::CStr::from_ptr(cstr)
                .to_string_lossy()
                .into_owned(),
        ))
    }
}

pub fn set_text(text: &str) -> Result<(), String> {
    let Ok(c) = std::ffi::CString::new(text) else {
        return Err("clipboard text contains a NUL byte".into());
    };
    unsafe {
        let pb: *mut AnyObject = msg_send![objc2::class!(UIPasteboard), generalPasteboard];
        if pb.is_null() {
            return Err("UIPasteboard.generalPasteboard was null".into());
        }
        let ns: *mut AnyObject =
            msg_send![objc2::class!(NSString), stringWithUTF8String: c.as_ptr()];
        if ns.is_null() {
            return Err("failed to allocate NSString".into());
        }
        let _: () = msg_send![pb, setString: ns];
        Ok(())
    }
}

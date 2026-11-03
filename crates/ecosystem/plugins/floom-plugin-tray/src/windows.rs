//! Windows: `Shell_NotifyIconW` plus a message-only window of our own.
//!
//! Win32 has no tray object to hand a callback to: `Shell_NotifyIconW` posts
//! `uCallbackMessage` to an `HWND` the caller supplies. So this arm creates a
//! hidden window whose entire job is to receive that message, and stores the
//! tray queue in its user data. The app's message pump — a winit event loop is
//! one — dispatches to it; [`Tray::events`] only drains what arrived.
//!
//! The window exists for the menu as well: `TrackPopupMenu` needs a window to
//! own the popup and reports the chosen command id to the call, not through a
//! message.

use std::sync::{Arc, Mutex};

use windows_sys::Win32::Foundation::{HWND, LPARAM, LRESULT, POINT, WPARAM};
use windows_sys::Win32::System::LibraryLoader::GetModuleHandleW;
use windows_sys::Win32::UI::Shell::{
    NIF_ICON, NIF_MESSAGE, NIF_TIP, NIM_ADD, NIM_DELETE, NOTIFYICONDATAW, Shell_NotifyIconW,
};
use windows_sys::Win32::UI::WindowsAndMessaging::{
    AppendMenuW, CreatePopupMenu, CreateWindowExW, DefWindowProcW, DestroyMenu, DestroyWindow,
    GWLP_USERDATA, GetCursorPos, GetWindowLongPtrW, HMENU, HWND_MESSAGE, IDI_APPLICATION,
    LoadIconW, MF_GRAYED, MF_SEPARATOR, MF_STRING, PostMessageW, RegisterClassW,
    SetForegroundWindow, SetWindowLongPtrW, TPM_RETURNCMD, TPM_RIGHTBUTTON, TrackPopupMenu, WM_APP,
    WM_LBUTTONDBLCLK, WM_LBUTTONUP, WM_NULL, WM_RBUTTONUP, WNDCLASSW,
};

use crate::{MenuItem, Queue, TrayEvent, drain, push};

/// The tray's callback message. `WM_APP` is where an application puts its own
/// messages, and the low word of `lParam` carries the mouse event that caused
/// it.
const WM_TRAYICON: u32 = WM_APP + 1;

const WINDOW_CLASS: &str = "FloomTrayWindow";

/// What the window procedure needs, in one allocation whose address is stable
/// for as long as the window exists.
struct WindowState {
    queue: Queue,
    menu: HMENU,
}

pub struct Tray {
    hwnd: HWND,
    menu: HMENU,
    state: *mut WindowState,
    /// Kept because `NIM_DELETE` needs the same `cbSize`/`hWnd`/`uID` the add
    /// used, and rebuilding it would be a second copy of the truth.
    icon_data: NOTIFYICONDATAW,
    queue: Queue,
}

impl Tray {
    pub fn show(label: &str, items: &[MenuItem]) -> Result<Self, String> {
        let queue: Queue = Arc::new(Mutex::new(Vec::new()));

        // SAFETY: every call below is a documented Win32 call, made on one
        // thread. The window procedure is invoked only by that thread's message
        // pump and reads the `WindowState` stored in the window's user data,
        // which outlives the window.
        unsafe {
            let instance = GetModuleHandleW(std::ptr::null());
            let class_name = wide(WINDOW_CLASS);

            let mut class: WNDCLASSW = std::mem::zeroed();
            class.lpfnWndProc = Some(window_procedure);
            class.hInstance = instance;
            class.lpszClassName = class_name.as_ptr();
            if RegisterClassW(&class) == 0 {
                return Err(
                    "tray: RegisterClassW failed — the tray window class could not \
                     be registered on this thread"
                        .into(),
                );
            }

            let hwnd = CreateWindowExW(
                0,
                class_name.as_ptr(),
                class_name.as_ptr(),
                0,
                0,
                0,
                0,
                0,
                HWND_MESSAGE,
                std::ptr::null_mut(),
                instance,
                std::ptr::null(),
            );
            if hwnd.is_null() {
                return Err("tray: CreateWindowExW failed for the tray window".into());
            }

            let menu = CreatePopupMenu();
            for item in items {
                match item {
                    MenuItem::Separator => {
                        AppendMenuW(menu, MF_SEPARATOR, 0, std::ptr::null());
                    }
                    MenuItem::Item { id, label, enabled } => {
                        let flags = MF_STRING | if *enabled { 0 } else { MF_GRAYED };
                        let text = wide(label);
                        AppendMenuW(menu, flags, *id as usize, text.as_ptr());
                    }
                }
            }

            let state = Box::into_raw(Box::new(WindowState {
                queue: queue.clone(),
                menu,
            }));
            SetWindowLongPtrW(hwnd, GWLP_USERDATA, state as isize);

            let mut icon_data: NOTIFYICONDATAW = std::mem::zeroed();
            icon_data.cbSize = std::mem::size_of::<NOTIFYICONDATAW>() as u32;
            icon_data.hWnd = hwnd;
            icon_data.uID = 1;
            icon_data.uFlags = NIF_MESSAGE | NIF_ICON | NIF_TIP;
            icon_data.uCallbackMessage = WM_TRAYICON;
            // A tray icon cannot be blank: the shell draws whatever `HICON` it
            // is given. The stock application icon is the honest default rather
            // than a transparent pixel pretending to be an icon.
            icon_data.hIcon = LoadIconW(std::ptr::null_mut(), IDI_APPLICATION);
            copy_wide(&mut icon_data.szTip, label);

            if Shell_NotifyIconW(NIM_ADD, &icon_data) == 0 {
                SetWindowLongPtrW(hwnd, GWLP_USERDATA, 0);
                drop(Box::from_raw(state));
                DestroyMenu(menu);
                DestroyWindow(hwnd);
                return Err(
                    "tray: Shell_NotifyIconW(NIM_ADD) was refused — this desktop may \
                     have no notification area"
                        .into(),
                );
            }

            Ok(Self {
                hwnd,
                menu,
                state,
                icon_data,
                queue,
            })
        }
    }

    pub fn events(&mut self) -> Vec<TrayEvent> {
        drain(&self.queue)
    }
}

impl Drop for Tray {
    fn drop(&mut self) {
        // SAFETY: the handles belong to this object, are used once here, and
        // the `WindowState` pointer was allocated in `show` and is freed once.
        unsafe {
            Shell_NotifyIconW(NIM_DELETE, &self.icon_data);
            DestroyMenu(self.menu);
            SetWindowLongPtrW(self.hwnd, GWLP_USERDATA, 0);
            drop(Box::from_raw(self.state));
            DestroyWindow(self.hwnd);
        }
    }
}

unsafe extern "system" fn window_procedure(
    hwnd: HWND,
    message: u32,
    wparam: WPARAM,
    lparam: LPARAM,
) -> LRESULT {
    if message != WM_TRAYICON {
        return unsafe { DefWindowProcW(hwnd, message, wparam, lparam) };
    }

    // SAFETY: the pointer was stored by `Tray::show`, is read only on the
    // thread that owns the window, and the window cannot outlive the `Tray`
    // that owns the allocation.
    let state = unsafe {
        let raw = GetWindowLongPtrW(hwnd, GWLP_USERDATA) as *const WindowState;
        raw.as_ref()
    };
    let Some(state) = state else {
        return 0;
    };

    match (lparam as u32) & 0xFFFF {
        WM_LBUTTONDBLCLK => push(&state.queue, TrayEvent::Activate),
        WM_LBUTTONUP | WM_RBUTTONUP => unsafe { open_menu(hwnd, state) },
        _ => {}
    }
    0
}

unsafe fn open_menu(hwnd: HWND, state: &WindowState) {
    let mut point = POINT { x: 0, y: 0 };
    unsafe {
        if GetCursorPos(&mut point) == 0 {
            return;
        }
        // A popup shown by a background window has to be brought forward
        // first, or it stays on screen the moment the user clicks elsewhere.
        SetForegroundWindow(hwnd);
        let chosen = TrackPopupMenu(
            state.menu,
            TPM_RETURNCMD | TPM_RIGHTBUTTON,
            point.x,
            point.y,
            0,
            hwnd,
            std::ptr::null(),
        );
        // Documented workaround: without this post the popup can refuse to
        // close.
        PostMessageW(hwnd, WM_NULL, 0, 0);
        if chosen != 0 {
            push(&state.queue, TrayEvent::Menu(chosen as u32));
        }
    }
}

/// A NUL-terminated UTF-16 copy of `text`.
fn wide(text: &str) -> Vec<u16> {
    text.encode_utf16().chain(std::iter::once(0)).collect()
}

/// Copy `text` into a fixed-size Win32 buffer, truncating rather than
/// overflowing — those buffers are not NUL-terminated for us.
fn copy_wide(buffer: &mut [u16], text: &str) {
    let units: Vec<u16> = text.encode_utf16().collect();
    let room = buffer.len().saturating_sub(1);
    let take = units.len().min(room);
    buffer[..take].copy_from_slice(&units[..take]);
    buffer[take] = 0;
    for slot in &mut buffer[take + 1..] {
        *slot = 0;
    }
}

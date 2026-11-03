//! Capability flags queried by applications instead of `cfg(target_os)`.
//!
//! # A bit is a promise
//!
//! A capability says "this platform can do X". A caller reads it to decide
//! whether to offer a feature, so a bit that is set when the platform cannot
//! do X is worse than a missing bit: the caller builds the feature, the
//! feature does nothing, and the gap is invisible.
//!
//! That is why the rule here is **narrower than it looks**. A bit exists
//! only when something implements it:
//!
//! | Bit | Implemented by |
//! |---|---|
//! | [`Capability::WINDOWING`] | every platform backend — the `Platform` trait's window creation |
//! | [`Capability::CLIPBOARD`] | `NSPasteboard` on macOS, `ClipboardManager` over JNI on Android, OSC 52 on a terminal |
//! | [`Capability::IME`] | winit's IME request on the desktop, the anchor view's `Editable` on Android |
//! | [`Capability::ACCESSIBILITY`] | a platform whose semantics tree reaches the OS — none of them yet |
//! | [`Capability::WEBGPU`] | the browser's `navigator.gpu` probe |
//! | [`Capability::SOFTWARE_RENDER`] | every platform that can present a rasterised buffer |
//! | [`Capability::TOUCH`] | the mobile backends, whose pointer events come from touches |
//! | [`Capability::NATIVE_VIEW`] | the desktop's native-view bridge, and only when one is installed |
//!
//! A bit that nothing implements **does not exist**. That is the
//! enforcement: setting a capability a preset does not implement is a
//! compile error until the bit is added back, and adding the bit is a
//! deliberate act that says where the implementation is. The three bits that
//! used to sit here with nothing behind them — `OPEN_URL`, `FILE_DIALOG`,
//! `DRM_KMS` — are gone, and their absence is the record of that.
//!
//! # A stub promises nothing
//!
//! The `*_stub` presets are for backends that do not exist yet, and they
//! claim only what a stand-in can honestly claim. A stand-in that claims a
//! soft keyboard has one is claiming a capability it does not have, and the
//! real backend has to remember to remove the claim — which is the drift this
//! whole file exists to prevent.
//!
//! # Deriving a flag from the implementation
//!
//! The last row of the table is the one that has to be *derived* rather
//! than written down. `Capability::NATIVE_VIEW` says "a provider exists",
//! and the desktop's own bridge is the only thing that knows whether one
//! does — so the desktop derives the flag from the bridge's presence rather
//! than having two places to keep in step. The same shape applies to the
//! clipboard on a platform whose adapter still holds the text in process, and
//! to a platform's accessibility tree reaching the OS.

use bitflags::bitflags;

bitflags! {
    /// What a platform can do.
    ///
    /// Every bit is a promise, and every promise has an implementation
    /// behind it. See the module documentation for the table.
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
    pub struct Capability: u32 {
        /// The platform can create and present windows.
        const WINDOWING        = 1 << 0;
        /// The platform has a **system** clipboard — one another process can
        /// read.
        ///
        /// A clipboard that only works inside its own process is not a
        /// clipboard, so a platform whose adapter still holds the text
        /// in memory does not claim this.
        const CLIPBOARD        = 1 << 1;
        /// The platform has a soft keyboard, and text typed into it reaches
        /// the framework.
        const IME              = 1 << 2;
        /// The platform's semantics tree reaches the operating system's
        /// accessibility API.
        const ACCESSIBILITY    = 1 << 3;
        /// The browser has WebGPU.
        const WEBGPU           = 1 << 6;
        /// The platform can present a rasterised buffer.
        const SOFTWARE_RENDER  = 1 << 7;
        /// The platform's pointer events come from touches.
        const TOUCH            = 1 << 8;
        /// The platform can composite a native view into the window.
        const NATIVE_VIEW      = 1 << 10;
    }
}

/// Snapshot of what this platform backend supports.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub struct PlatformCapabilities {
    pub caps: Capability,
}

impl PlatformCapabilities {
    pub fn supports(self, cap: Capability) -> bool {
        self.caps.contains(cap)
    }

    /// What a desktop platform can do.
    ///
    /// The common truth, with the two flags that follow an implementation
    /// left out: the **system** clipboard is real on macOS and not yet on the
    /// others, and the native-view flag follows the bridge's presence. Each
    /// platform that has one says so where it has one.
    pub fn desktop_default() -> Self {
        Self {
            caps: Capability::WINDOWING | Capability::IME | Capability::SOFTWARE_RENDER,
        }
    }

    /// What a platform that cannot compose native views can do.
    ///
    /// A stand-in for a backend that does not exist yet, so it claims only
    /// what a stand-in can honestly claim: a window, and — on the platforms
    /// whose pointers are touches — a touch.
    ///
    /// Notably **not** a soft keyboard: the stand-in's IME is an in-memory
    /// map, and a real backend adds [`Capability::IME`] when its keyboard
    /// actually delivers text.
    pub fn unsupported_a11y_stub() -> Self {
        Self {
            caps: Capability::WINDOWING | Capability::TOUCH | Capability::SOFTWARE_RENDER,
        }
    }

    /// What the web can do.
    ///
    /// The IME bit is honest for a narrow reason: the web's run loop installs
    /// a real `compositionend` listener, so committed text reaches the
    /// framework. The caret-area handshake has no browser equivalent and is
    /// a no-op there.
    pub fn web_default(webgpu_available: bool) -> Self {
        let mut caps = Capability::WINDOWING | Capability::SOFTWARE_RENDER | Capability::IME;
        if webgpu_available {
            caps |= Capability::WEBGPU;
        }
        Self { caps }
    }

    /// What the embedded platform can do.
    ///
    /// A framebuffer and a software rasteriser. There is no DRM/KMS backend
    /// yet, so the platform does not claim one: the bit is added when the
    /// code that would honour it exists.
    pub fn embedded_default() -> Self {
        Self {
            caps: Capability::WINDOWING | Capability::SOFTWARE_RENDER,
        }
    }

    /// What a terminal can do.
    ///
    /// A character-grid terminal: it paints software-rendered cells, owns a
    /// window the framework lays out into, and can push clipboard text out via
    /// OSC 52. It has no native IME surface (the caret-area handshake is a
    /// no-op) and no accessibility tree the OS can read, so those stay off.
    pub fn tui_default() -> Self {
        Self {
            caps: Capability::WINDOWING | Capability::SOFTWARE_RENDER | Capability::CLIPBOARD,
        }
    }
}

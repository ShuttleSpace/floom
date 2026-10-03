//! Capability flags queried by applications instead of `cfg(target_os)`.

use bitflags::bitflags;

bitflags! {
    #[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
    pub struct Capability: u32 {
        const WINDOWING        = 1 << 0;
        const CLIPBOARD        = 1 << 1;
        const IME              = 1 << 2;
        const ACCESSIBILITY    = 1 << 3;
        const OPEN_URL         = 1 << 4;
        const FILE_DIALOG      = 1 << 5;
        const WEBGPU           = 1 << 6;
        const SOFTWARE_RENDER  = 1 << 7;
        const TOUCH            = 1 << 8;
        const DRM_KMS          = 1 << 9;
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

    pub fn desktop_default() -> Self {
        Self {
            caps: Capability::WINDOWING
                | Capability::CLIPBOARD
                | Capability::IME
                | Capability::ACCESSIBILITY
                | Capability::OPEN_URL
                | Capability::FILE_DIALOG
                | Capability::SOFTWARE_RENDER,
        }
    }

    pub fn unsupported_a11y_stub() -> Self {
        // Mobile stubs may expose windowing/touch while a11y is explicitly absent.
        Self {
            caps: Capability::WINDOWING
                | Capability::TOUCH
                | Capability::SOFTWARE_RENDER
                | Capability::IME,
        }
    }

    pub fn web_default(webgpu_available: bool) -> Self {
        let mut caps = Capability::WINDOWING
            | Capability::SOFTWARE_RENDER
            | Capability::NATIVE_VIEW
            | Capability::IME;
        if webgpu_available {
            caps |= Capability::WEBGPU;
        }
        Self { caps }
    }

    pub fn embedded_default(drm: bool) -> Self {
        let mut caps = Capability::SOFTWARE_RENDER;
        if drm {
            caps |= Capability::DRM_KMS | Capability::WINDOWING;
        } else {
            caps |= Capability::WINDOWING;
        }
        Self { caps }
    }

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

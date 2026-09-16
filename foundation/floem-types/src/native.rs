//! Shared vocabulary for native composition participants.
//!
//! These live in `floem-types` and not in `floem-native` for the same reason
//! [`OverlayKind`] does: a widget asks for one, the host grants it, and a
//! platform provider implements it, and none of those three may depend upward
//! at the others. `floem-native` is at L4 and the widget layer is at L6, so a
//! widget naming "I want a video, composited as an overlay" needs the words to
//! exist below both of them.
//!
//! [`OverlayKind`]: crate::OverlayKind

/// What kind of native view a participant is.
///
/// Deliberately a tag rather than a typed handle: the framework names intent
/// without knowing any concrete OS type, and a provider with no opinion about
/// maps can still answer `Custom("com.example.threed")`.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NativeViewKind {
    Map,
    Video,
    Web,
    /// A native text field. Focusable participants of this kind are the case
    /// where the framework must yield IME ownership.
    TextInput,
    Custom(&'static str),
}

/// How a participant is composited relative to the framework's own surface.
///
/// The order of the variants is *not* a strength order. Which of these is
/// "best" is a property of the platform: X11 can raise a subwindow above a
/// surface, Wayland has no subwindows at all, Windows child `HWND`s fight D3D
/// airspace, KMS has real hardware planes. A provider publishes what it can do
/// and the host negotiates.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum NativeCompositionMode {
    /// A sibling window/view stacked above the framework's surface.
    ///
    /// Cheap and widely available, but it composes above the *whole* scene: a
    /// participant can be occluded by another participant, never by
    /// framework-painted content.
    Overlay,
    /// Contents imported as a texture and drawn as a scene resource, so the
    /// participant composites *inside* the scene and can be occluded, clipped
    /// and transformed like anything else.
    Texture,
    /// A real compositor surface or hardware plane handed to the OS compositor.
    Surface,
    /// Rendered offscreen and blitted into the framework's surface.
    Offscreen,
}

impl NativeCompositionMode {
    /// Whether the *framework* holds this participant's pixels, and can
    /// therefore crop them itself.
    ///
    /// This is the property that decides the fallback for a provider that
    /// cannot crop a view it does not own: `Texture` and `Offscreen` are drawn
    /// by the framework, so a host-side clip applies for free, while `Overlay`
    /// and `Surface` are shown by the OS and are cropped only if the platform
    /// or the provider does it.
    pub fn is_host_composited(self) -> bool {
        matches!(self, Self::Texture | Self::Offscreen)
    }
}

/// Whether a participant takes pointer input itself.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum InputMode {
    /// The native view does not want input; the framework routes hits as if the
    /// participant were not there.
    #[default]
    Passthrough,
    /// The native view takes OS input itself. The framework must *subtract*
    /// the participant's rect from its own hit-test area rather than add to
    /// it — a capturing participant's region resolves to the participant, not
    /// to whatever framework widget lies beneath it.
    Capture,
}

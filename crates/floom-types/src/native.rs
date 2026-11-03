//! Shared vocabulary for native composition participants.
//!
//! These live in `floom-types` and not in `floom-native` for the same reason
//! [`OverlayKind`] does: a widget asks for one, the host grants it, and a
//! platform provider implements it, and none of those three may depend upward
//! at the others. `floom-native` is at L4 and the widget layer is at L6, so a
//! widget naming "I want a video, composited as an overlay" needs the words to
//! exist below both of them.
//!
//! [`OverlayKind`]: crate::OverlayKind

use std::sync::Arc;

use crate::ResourceId;

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

/// A participant frozen into a texture, for a transition that moves it.
///
/// This lives beside the other three nouns rather than in `floom-native` for
/// the same reason they do: the widget that paints the frozen image sits two
/// layers above the crate that defines the ABI, and asking it to name a type
/// from there would be an upward edge. A fourth noun was not anticipated by
/// 16.1 — a *frozen* participant is still the same participant, and the only
/// thing a widget needs to know about it is how to draw it.
///
/// The resource is owned by whoever took the snapshot until `restore`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct NativeSnapshot {
    /// Texture resource the renderer can draw as an image node.
    pub resource: ResourceId,
    /// Size of the frozen texture in device pixels. A snapshot taken before a
    /// scale change and restored after it is a stale-size bug.
    pub pixel_width: u32,
    /// Height of the frozen texture in device pixels.
    pub pixel_height: u32,
}

/// Pixels of a frozen participant, handed from the OS host to the renderer.
///
/// A snapshotter lives in the windowing crate; the renderer that uploads the
/// pixels lives a layer above it. Carrying the bytes in a type *both* can name
/// is what lets them stay decoupled: this struct holds plain `Arc<[u8]>`, so
/// neither side has to depend on the other's OS or GPU types — the same
/// decoupling that puts the other native nouns in this crate rather than in
/// `floom-native`.
///
/// The bytes are always [`ImageFormat::Rgba8`] / premultiplied, the format the
/// vello path uploads verbatim; the host does the OS-specific conversion
/// (AppKit hands back `ARGB` premultiplied) before it stashes one here.
#[derive(Debug, Clone)]
pub struct NativeCapture {
    /// The resource the renderer must key the uploaded texture on, so the
    /// frozen participant's `SceneNode::Image` (which references this id) and
    /// the GPU brush it resolves to agree.
    ///
    /// Minted by the host at snapshot time; unique among resident captures for
    /// the life of the transition.
    pub resource: ResourceId,
    /// Width of the frozen texture in device pixels.
    pub pixel_width: u32,
    /// Height of the frozen texture in device pixels.
    pub pixel_height: u32,
    /// RGBA8, premultiplied, row-major, `pixel_width * pixel_height * 4` bytes.
    pub pixels: Arc<[u8]>,
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

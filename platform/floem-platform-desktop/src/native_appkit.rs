//! The AppKit half of the desktop bridge (Phase 16, task 16.2, step 7).
//!
//! A desktop participant on macOS is a real `NSView` added to the window's
//! content view, and this module is the [`NativeViewHost`] that puts it there.
//! It is deliberately small: everything with arithmetic in it — the snap, the
//! origin conversion, the crop split, the conversion to points — lives in
//! [`crate::native`] and is tested without a window. What is left is the part
//! that genuinely needs an OS.
//!
//! # Two views per participant, on purpose
//!
//! Each participant is a **container** holding the participant's own view. The
//! container is the crop: it is sized to the visible window onto the
//! participant, and the participant is offset inside it, so AppKit's ordinary
//! "a view clips its subviews to its own bounds" does the cropping. That is why
//! this backend declares [`ClipSupport::Platform`] honestly rather than
//! degrading every clipped participant — the platform really does crop here.
//!
//! Shrinking the participant's own frame would rescale its content, and moving
//! its `bounds` origin would scroll it. A crop of somebody else's view has to be
//! somebody else's view.
//!
//! The container is created for *every* participant, crop or no crop, so that
//! the cropping path is the only path. The cost is one extra `NSView` per
//! participant, which is cheap next to a list that is already reasoning about
//! how many of them exist; the alternative is a second arrangement that only a
//! clipped participant reaches.
//!
//! # The container is flipped, and the parent is asked about itself
//!
//! The container overrides `isFlipped` to `true`, so the coordinates it reads
//! for the participant inside it are top-left — the framework's own convention,
//! with no conversion at all. The container's *own* frame is expressed in its
//! parent's convention, which is not ours to decide: `NSView::isFlipped` is
//! per-view, and winit's content view is flipped while a bare `NSView` is not.
//! So the backend asks the view (`parent.isFlipped()`) instead of assuming, and
//! [`os_frame`] turns that answer into either a copy or a mirror.
//!
//! # What this backend declares, and what it does not
//!
//! - `clip: Platform` — the clipping container above.
//! - `input: ALL` — a passthrough participant's container answers `hitTest:`
//!   with `nil`, so the click reaches the framework's surface underneath.
//! - `focusable: true` — the participant, or failing that its container, becomes
//!   the window's first responder, which is what makes the OS hand it the
//!   keyboard and the IME.
//! - `snapshot: true` — a participant can be frozen into a texture for a
//!   transition (task 16.4 step 3). The bitmap round trip lives in
//!   [`NativeViewHost::snapshot_view`]: it reads the view's rendered pixels back
//!   out of the window server and stashes them for the app runner to import as
//!   an external texture.

use std::cell::Cell;
use std::collections::HashMap;
use std::ptr::NonNull;
use std::sync::Arc;

use objc2::rc::Retained;
use objc2::{DefinedClass, MainThreadMarker, define_class, msg_send};
use objc2_app_kit::{NSBitmapFormat, NSResponder, NSView};
use objc2_foundation::{NSObject, NSPoint, NSRect, NSSize};

use floem_native::{
    ClipSupport, InputMode, InputModes, NativeError, NativeSlot, NativeSnapshot, NativeViewKind,
};
use floem_types::{NativeCapture, ResourceId, Size};

use crate::native::{
    NativeViewHost, Origin, ViewPlacement, device_to_points, os_frame, split_placement,
};

/// The clipping container. One per participant, whether or not it is cropped.
///
/// A class of our own only because two behaviours have to change and neither is
/// reachable from outside the class: `isFlipped` (a method, not a property) and
/// `hitTest:` (the pass-through switch). Everything else is a plain `NSView`.
struct ContainerIvars {
    /// When set, the container does not answer a hit test, so the click falls
    /// through to whatever the framework painted underneath.
    passthrough: Cell<bool>,
}

define_class!(
    #[unsafe(super(NSView, NSResponder, NSObject))]
    #[ivars = ContainerIvars]
    #[name = "FloemNativeViewContainer"]
    struct Container;

    impl Container {
        /// Top-left coordinates for the participant inside it.
        ///
        /// This is what keeps the crop arithmetic a subtraction: the framework's
        /// rects are top-left, and a flipped view reads its subviews' frames the
        /// same way, so the offset never has to be mirrored.
        #[unsafe(method(isFlipped))]
        fn is_flipped(&self) -> bool {
            true
        }

        /// A participant that takes input has to be able to hold the keyboard,
        /// and `NSView` refuses by default.
        #[unsafe(method(acceptsFirstResponder))]
        fn accepts_first_responder(&self) -> bool {
            true
        }

        /// `nil` makes the container and everything inside it invisible to the
        /// pointer.
        ///
        /// The container is the view that has to answer this. Hit-testing
        /// descends from the window, so a passthrough *content* view would still
        /// be preceded by a container that claims the point on its behalf and
        /// swallows the click.
        #[unsafe(method_id(hitTest:))]
        fn hit_test(&self, point: NSPoint) -> Option<Retained<NSView>> {
            // One expression, no early `return`: the macro rewrites the body's
            // tail into the return-value conversion, and a `return` skips it.
            if self.ivars().passthrough.get() {
                None
            } else {
                // SAFETY: `super`'s hit test is what a view with no opinion
                // does, and the argument is the point this method was handed.
                unsafe { msg_send![super(self), hitTest: point] }
            }
        }
    }
);

impl Container {
    fn new(mtm: MainThreadMarker) -> Retained<Self> {
        let this = mtm.alloc().set_ivars(ContainerIvars {
            passthrough: Cell::new(false),
        });
        // SAFETY: a freshly allocated view, sent the initialiser it inherits.
        // `NSView::init` is documented to come out zero-sized, which is what we
        // want: the frame is set by the first placement.
        let this: Retained<Self> = unsafe { msg_send![super(this), init] };
        this
    }

    fn set_passthrough(&self, passthrough: bool) {
        self.ivars().passthrough.set(passthrough);
    }
}

/// Builds the participant's own view. `NSView` when nobody supplied one.
///
/// The seam that makes this a bridge rather than a demo: the framework can only
/// make a view of a *kind*, and a `WKWebView` or a player surface has to come
/// from the layer that knows about it.
pub type ViewFactory = Box<dyn Fn(NativeViewKind, MainThreadMarker) -> Retained<NSView>>;

/// One participant's two views.
struct Participant {
    container: Retained<Container>,
    content: Retained<NSView>,
}

/// [`NativeViewHost`] over a live `NSView`.
pub struct MacNativeViewHost {
    /// The view participants are added to: the window's content view.
    parent: NativeParent,
    /// Held rather than fetched per call: every `Retained` on a main-thread-only
    /// class needs one, and a view created outside the main thread is not a
    /// thing this backend can honour.
    mtm: MainThreadMarker,
    factory: ViewFactory,
    views: HashMap<NativeSlot, Participant>,
    /// Snapshots taken since the last drain, waiting for the app runner to
    /// import them as textures. Filled only by [`Self::snapshot_view`]; the
    /// runner empties it through [`Self::take_captures`].
    pending_captures: Vec<NativeCapture>,
    /// Monotonic id source for snapshot textures. Each frozen participant gets a
    /// fresh resource id, so a texture is never keyed on a stale capture.
    next_capture: u32,
}

impl std::fmt::Debug for MacNativeViewHost {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.debug_struct("MacNativeViewHost")
            .field("participants", &self.views.len())
            .finish_non_exhaustive()
    }
}

impl MacNativeViewHost {
    /// The default factory makes a plain `NSView`, which is a real view that
    /// draws nothing — enough for the lifecycle and the geometry to be exercised
    /// end to end before a web view exists.
    pub fn new(parent: NativeParent, mtm: MainThreadMarker) -> Self {
        Self::with_factory(parent, mtm, Box::new(|_, mtm| NSView::new(mtm)))
    }

    pub fn with_factory(parent: NativeParent, mtm: MainThreadMarker, factory: ViewFactory) -> Self {
        Self {
            parent,
            mtm,
            factory,
            views: HashMap::new(),
            pending_captures: Vec::new(),
            next_capture: 1,
        }
    }

    /// How many participants are currently mounted.
    pub fn len(&self) -> usize {
        self.views.len()
    }

    pub fn is_empty(&self) -> bool {
        self.views.is_empty()
    }

    /// Whether the views are placed in a top-left or a bottom-left frame of
    /// reference.
    ///
    /// Asked of the parent rather than assumed: `isFlipped` is a property of the
    /// view whose coordinate system `setFrame:` is read in, and the two cases
    /// are both live — winit's content view is flipped, a bare `NSView` is not.
    fn origin(&self) -> Origin {
        if self.parent.is_flipped() {
            Origin::TopLeft
        } else {
            Origin::BottomLeft
        }
    }

    /// Set both frames from one placement.
    ///
    /// The container's frame is converted; the participant's is **not**, because
    /// it is read in the container's flipped coordinate system, where the
    /// framework's top-left rects already mean what they say.
    fn set_frames(&self, participant: &Participant, placement: ViewPlacement) {
        let scale = self.scale();
        let frames = split_placement(placement);

        let container = os_frame(frames.container, self.parent_extent(), scale, self.origin());
        let (x, y, width, height) = device_to_points(container, scale);
        participant
            .container
            .setFrame(NSRect::new(NSPoint::new(x, y), NSSize::new(width, height)));

        let (x, y, width, height) = device_to_points(frames.content, scale);
        participant
            .content
            .setFrame(NSRect::new(NSPoint::new(x, y), NSSize::new(width, height)));
    }
}

/// Why a window could not be given a native-view provider.
///
/// Typed rather than a string because the interesting cases are not failures a
/// user sees: they are the reasons a caller must decide what to do instead, and
/// `NoNativeHandle` in particular is a *timing* answer — winit has not produced
/// the handle yet — rather than a verdict on the window.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum NativeViewUnavailable {
    /// No window with that id is live in this platform.
    UnknownWindow,
    /// The OS window exists but has not produced a handle yet, or refuses to.
    NoNativeHandle,
    /// A handle the OS produced, and it is not an AppKit view.
    NotAppKit,
    /// The caller is not on the main thread, so it may not touch a view.
    NotMainThread,
}

impl std::fmt::Display for NativeViewUnavailable {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let what = match self {
            Self::UnknownWindow => "no such window",
            Self::NoNativeHandle => "the window has no OS handle yet",
            Self::NotAppKit => "the window handle is not an AppKit view",
            Self::NotMainThread => "not on the main thread",
        };
        write!(f, "cannot install native views: {what}")
    }
}

impl std::error::Error for NativeViewUnavailable {}

/// One view in the parent's own tree, as AppKit reports it.
///
/// Deliberately not "where we placed it": a device check that asks the bridge
/// what it did gets the bridge's story back, and the questions worth asking —
/// did the view land on the pixel the framework computed, is it where the OS
/// thinks it is — are about what the OS ended up with.
#[derive(Debug, Clone, Copy, PartialEq)]
pub struct ViewFrame {
    /// `(origin.x, origin.y, size.width, size.height)`, in points, in the
    /// parent's coordinate system.
    pub frame: (f64, f64, f64, f64),
    pub hidden: bool,
    /// How many views are directly inside this one. One, for a participant:
    /// the container holds the participant's own view. Zero means something
    /// else is under the parent.
    pub children: usize,
}

/// The view participants are added to, plus the queries a device check needs.
///
/// A handle rather than a bare `NSView` so that the AppKit types stay in this
/// crate: the probe in `examples/floem2-probe` is not a macOS crate and should
/// not have to be one to ask where a participant landed.
#[derive(Debug)]
pub struct NativeParent(Retained<NSView>);

impl NativeParent {
    /// Wrap a view the caller has already retained.
    ///
    /// `pub(crate)` in effect: the platform crate is the only thing that has a
    /// window handle to get one from.
    pub(crate) fn from_retained(view: Retained<NSView>) -> Self {
        Self(view)
    }

    /// The view itself, for this crate's own use.
    fn view(&self) -> &NSView {
        &self.0
    }

    /// The parent's own extent in points.
    pub fn extent(&self) -> (f64, f64) {
        let size = self.0.frame().size;
        (size.width, size.height)
    }

    /// Whether the parent's coordinate system grows downward.
    ///
    /// The fact that decides whether a placement is converted at all, which is
    /// why it is worth being able to see from outside.
    pub fn is_flipped(&self) -> bool {
        self.0.isFlipped()
    }

    /// Every view directly under this parent, in the order AppKit holds them.
    pub fn subview_frames(&self) -> Vec<ViewFrame> {
        let subviews = self.0.subviews();
        subviews
            .iter()
            .map(|view| {
                let frame = view.frame();
                ViewFrame {
                    frame: (
                        frame.origin.x,
                        frame.origin.y,
                        frame.size.width,
                        frame.size.height,
                    ),
                    hidden: view.isHidden(),
                    children: view.subviews().count(),
                }
            })
            .collect()
    }
}

/// Whether `responder` is one of `views`.
///
/// Pointer identity, because that is what it is: the question is whether the
/// window's first responder *is* this object, not whether it is equal to it.
fn is_one_of(responder: &NSResponder, views: [&NSView; 2]) -> bool {
    let responder = std::ptr::from_ref(responder).cast::<()>();
    views
        .iter()
        .any(|view| std::ptr::from_ref(*view).cast::<()>() == responder)
}

impl NativeViewHost for MacNativeViewHost {
    fn parent_extent(&self) -> Size {
        let size = self.parent.view().frame().size;
        Size::new(size.width, size.height)
    }

    fn scale(&self) -> f64 {
        // A view outside a window has no screen to be scaled for, and 1.0 is
        // the only defensible answer. It is also a transient state: the first
        // placement of a participant that is not yet on screen would otherwise
        // divide by whatever the window last reported.
        self.parent
            .view()
            .window()
            .map(|window| window.backingScaleFactor())
            .unwrap_or(1.0)
    }

    /// See the module docs: the container clips, so the platform crops.
    fn clip_support(&self) -> ClipSupport {
        ClipSupport::Platform
    }

    /// Both, because the container's `hitTest:` can say no.
    fn input_modes(&self) -> InputModes {
        InputModes::ALL
    }

    /// The participant — or its container — can hold the keyboard.
    fn can_focus(&self) -> bool {
        true
    }

    /// Freezing a participant into a texture is the one capability this backend
    /// now advertises: the bitmap round trip below is what makes a transition
    /// animate a native view it otherwise could not.
    fn can_snapshot(&self) -> bool {
        true
    }

    fn create_view(&mut self, slot: NativeSlot, kind: NativeViewKind) -> Result<(), NativeError> {
        let container = Container::new(self.mtm);
        let content = (self.factory)(kind, self.mtm);
        container.addSubview(&content);
        self.parent.view().addSubview(&container);
        self.views.insert(slot, Participant { container, content });
        Ok(())
    }

    fn destroy_view(&mut self, slot: NativeSlot) {
        // Tolerating a slot that was never created is part of the contract, and
        // it is not hypothetical: teardown runs on error paths.
        let Some(participant) = self.views.remove(&slot) else {
            return;
        };
        participant.container.removeFromSuperview();
        // Dropped with the map entry, which releases the container and — via it
        // — the participant's own view. Removing the container from its
        // superview is what stops it being drawn; the release is what stops it
        // existing.
    }

    fn place_view(&mut self, slot: NativeSlot, placement: ViewPlacement) {
        let Some(participant) = self.views.get(&slot) else {
            return;
        };
        // Idempotent in the OS sense as well as the bridge's: `setFrame:` with
        // the frame it already has is a no-op, and setting the same passthrough
        // flag costs a store.
        participant
            .container
            .set_passthrough(placement.input == InputMode::Passthrough);
        self.set_frames(participant, placement);
    }

    fn set_view_visible(&mut self, slot: NativeSlot, visible: bool) {
        let Some(participant) = self.views.get(&slot) else {
            return;
        };
        // Hiding, not moving: the placement in a hidden state is meaningless,
        // and a view moved off-screen to hide it can flash at the old position
        // on the way.
        participant.container.setHidden(!visible);
    }

    fn set_view_focus(&mut self, slot: NativeSlot, focused: bool) {
        let Some(participant) = self.views.get(&slot) else {
            return;
        };
        let Some(window) = self.parent.view().window() else {
            return;
        };

        if focused {
            // The participant's own view first: it is the one that knows what a
            // keystroke means. A plain `NSView` refuses the keyboard, and a
            // participant that has been told it owns focus must not end up with
            // nothing, so the container (which accepts) is the fallback.
            if !window.makeFirstResponder(Some(&participant.content)) {
                window.makeFirstResponder(Some(&participant.container));
            }
            return;
        }

        // Hand the keyboard back to the view the framework paints into — but
        // only if it is ours to hand back. A participant that never took focus
        // must not pull it away from whatever did, which is a real case: the
        // pointer can put focus into a view the framework does not track.
        let held = window.firstResponder();
        let ours = held
            .as_deref()
            .is_some_and(|held| is_one_of(held, [&participant.content, &participant.container]));
        if ours {
            window.makeFirstResponder(Some(self.parent.view()));
        }
    }

    fn snapshot_view(&mut self, slot: NativeSlot) -> Option<NativeSnapshot> {
        let participant = self.views.get(&slot)?;
        // The content view's own bounds, in points. `cacheDisplayInRect` renders
        // the view's already-drawn content at the window's backing resolution,
        // so the bitmap comes back in *device* pixels — exactly what the
        // renderer's external texture wants, with no manual scale multiply.
        let bounds = participant.content.bounds();
        let rep = participant
            .content
            .bitmapImageRepForCachingDisplayInRect(bounds)?;
        participant
            .content
            .cacheDisplayInRect_toBitmapImageRep(bounds, &rep);

        let width = rep.pixelsWide() as u32;
        let height = rep.pixelsHigh() as u32;
        let Some(src) = NonNull::new(rep.bitmapData()) else {
            return None;
        };
        if width == 0 || height == 0 {
            return None;
        }

        // AppKit hands back `ARGB` premultiplied by default; vello uploads
        // `RGBA8` premultiplied verbatim. Reorder the channels, and if the
        // platform gave back *non*-premultiplied bytes, premultiply on the way
        // so the pixels match what the renderer expects.
        let format = rep.bitmapFormat();
        let alpha_first = format.contains(NSBitmapFormat::AlphaFirst);
        let premultiplied = !format.contains(NSBitmapFormat::AlphaNonpremultiplied);
        let len = width as usize * height as usize * 4;
        let src_bytes = unsafe { std::slice::from_raw_parts(src.as_ptr(), len) };

        let mut pixels: Vec<u8> = Vec::with_capacity(len);
        let mut push = |a: u8, r: u8, g: u8, b: u8| {
            let (r, g, b) = if premultiplied {
                (r, g, b)
            } else {
                let mul = |c: u8| -> u8 {
                    let v = c as u32 * a as u32 + 127;
                    (v / 255) as u8
                };
                (mul(r), mul(g), mul(b))
            };
            pixels.extend_from_slice(&[r, g, b, a]);
        };
        for chunk in src_bytes.chunks_exact(4) {
            if alpha_first {
                push(chunk[0], chunk[1], chunk[2], chunk[3]);
            } else {
                push(chunk[3], chunk[0], chunk[1], chunk[2]);
            }
        }

        let resource = ResourceId::new(self.next_capture, 1);
        self.next_capture = self.next_capture.wrapping_add(1);
        self.pending_captures.push(NativeCapture {
            resource,
            pixel_width: width,
            pixel_height: height,
            pixels: Arc::from(pixels.into_boxed_slice()),
        });
        Some(NativeSnapshot {
            resource,
            pixel_width: width,
            pixel_height: height,
        })
    }

    fn take_captures(&mut self) -> Vec<NativeCapture> {
        std::mem::take(&mut self.pending_captures)
    }
}

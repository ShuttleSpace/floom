//! A headless app, driven the way a real window would drive it.
//!
//! Three integration suites in `floem-app` had each grown their own copy of the
//! same harness — a fake platform, a runner, a scene-damage drain, a key
//! mapping — and every new widget test paid for it again. This is that harness,
//! once, with the parts a test actually reaches for named.
//!
//! ```no_run
//! use floem_testkit::TestApp;
//! use floem_widgets::Checkbox;
//!
//! let mut app = TestApp::new(floem_types::Size::new(200.0, 32.0), Checkbox::new("Agree"));
//! assert!(app.tap("Agree"), "the click repainted");
//! assert!(app.widget::<Checkbox>("Agree").is_checked());
//! ```
//!
//! # What it is, and what it is not
//!
//! It is the *whole chain*: [`FakePlatform`] delivers the same
//! `PlatformEvent`s a windowing backend would, [`AppRunner`] routes them, and
//! the widget tree reacts and repaints. What comes back out is what a test can
//! legitimately assert on — the widget's own state, the scene that would be
//! drawn, the semantics that would be published, and whether any of it reached
//! the screen as damage.
//!
//! It is not `floem_widgets::testkit`, which drives a *single* widget through a
//! recording context to prove that widget's own logic. This one proves the
//! wiring *around* it: that the press the platform reported reached the control
//! under the cursor, that the key reached whatever held focus, and that the
//! resulting change did not stop inside the widget.
//!
//! # Selectors
//!
//! A test should be able to say "click the thing labelled *Agree*" rather than
//! carry a [`NodeId`] around and hope it is still that. [`Selector`] resolves by
//! the label or role a widget publishes to assistive technology — the same
//! strings the accessibility tree carries, so the tests read the way the screen
//! does. `NodeId` still works everywhere a selector does (`Into<Selector>`), for
//! the cases where a test mounted the node itself.
//!
//! # Damage
//!
//! Every actor returns whether anything was repainted, and the answer is "any
//! frame, not the last one": a single interaction can ask for a cascade (new
//! value → repaint → re-measure → re-layout → repaint) and only the first frame
//! carries damage. A test that wants to know whether an interaction reached the
//! screen asks that question, not which frame answered it.

#![forbid(unsafe_code)]

use floem_app::{AppRunner, FrameStats, NodeId, WidgetHost};
use floem_core::{SemanticsRole, WidgetAny};
use floem_layout::LayoutStyle;
use floem_platform::{FakePlatform, KeyCode, Platform, PlatformEvent, PointerButton, WindowConfig};
use floem_renderer_software::SoftwarePresenter;
use floem_scene::SceneNode;
use floem_types::{Modifiers, Point, Rect, SceneNodeId, Size, WindowId};

/// Screen the tests use when the exact size does not matter.
pub const DEFAULT_SIZE: Size = Size::new(800.0, 600.0);

/// How many rounds [`TestApp::settle`] will wait for a tree to stop moving
/// before it declares the tree stuck.
///
/// Generous on purpose: it has to cover a cascade (re-measure → re-layout →
/// repaint) *and* an overlay entry animation, which is nine frames on its own.
/// A tree that is still asking for frames after this many rounds is not slow,
/// it is looping.
pub const SETTLE_ROUNDS: usize = 64;

/// How to find a node.
///
/// Label and role are read from what the widget publishes to assistive
/// technology, so a selector names the same thing the accessibility tree does.
#[derive(Debug, Clone, PartialEq)]
pub enum Selector {
    /// The handle a mount returned.
    Id(NodeId),
    /// A node whose published label is exactly this.
    Label(String),
    /// The first node publishing this role.
    Role(SemanticsRole),
}

impl Selector {
    pub fn id(node: NodeId) -> Self {
        Self::Id(node)
    }

    pub fn label(label: impl Into<String>) -> Self {
        Self::Label(label.into())
    }

    pub fn role(role: SemanticsRole) -> Self {
        Self::Role(role)
    }
}

impl From<NodeId> for Selector {
    fn from(node: NodeId) -> Self {
        Self::Id(node)
    }
}

impl From<&str> for Selector {
    fn from(label: &str) -> Self {
        Self::Label(label.to_string())
    }
}

impl From<String> for Selector {
    fn from(label: String) -> Self {
        Self::Label(label)
    }
}

/// A booted app: a window, a runner, a mounted root, and the actors to drive
/// them.
pub struct TestApp {
    platform: FakePlatform,
    runner: AppRunner<SoftwarePresenter>,
    window: WindowId,
    size: Size,
}

impl TestApp {
    /// A window of `size` whose root is `root`, pinned to the whole viewport.
    ///
    /// Pinning keeps the arithmetic honest: a click at `x` on a 200px-wide
    /// control is unambiguously at fraction `x / 200`, with no layout in
    /// between to reason about. Use [`TestApp::unpinned`] when the layout *is*
    /// the thing under test.
    ///
    /// The app boots before this returns: the window attaches, the tree lays
    /// out, and the first frame paints.
    pub fn new(size: Size, root: impl WidgetAny + 'static) -> Self {
        Self::boot(size, root, true)
    }

    /// A window of `size` whose root is left to its own [`layout_request`].
    ///
    /// [`layout_request`]: floem_core::Widget::layout_request
    pub fn unpinned(size: Size, root: impl WidgetAny + 'static) -> Self {
        Self::boot(size, root, false)
    }

    fn boot(size: Size, root: impl WidgetAny + 'static, pinned: bool) -> Self {
        let mut platform = FakePlatform::new();
        let mut host = WidgetHost::new();
        let style = if pinned {
            LayoutStyle {
                width: Some(size.width as f32),
                height: Some(size.height as f32),
                ..LayoutStyle::default()
            }
        } else {
            host.set_size(size);
            LayoutStyle::default()
        };
        host.mount_root(style, root);
        let runner = AppRunner::new(host, SoftwarePresenter::new());
        let window = platform.create_window(WindowConfig {
            title: "test".into(),
            size,
            resizable: true,
        });
        let mut app = Self {
            platform,
            runner,
            window,
            size,
        };
        // Let the window attach, lay out and paint its first frame.
        for _ in 0..6 {
            app.runner.pump(&mut app.platform);
        }
        app
    }

    // --- inspection --------------------------------------------------------

    pub fn host(&self) -> &WidgetHost {
        self.runner.host()
    }

    pub fn host_mut(&mut self) -> &mut WidgetHost {
        self.runner.host_mut()
    }

    pub fn runner(&mut self) -> &mut AppRunner<SoftwarePresenter> {
        &mut self.runner
    }

    pub fn platform(&mut self) -> &mut FakePlatform {
        &mut self.platform
    }

    /// The mounted root.
    pub fn root(&self) -> NodeId {
        self.runner.host().root().expect("a root is mounted")
    }

    /// Logical size of the app's window.
    pub fn size(&self) -> Size {
        self.size
    }

    /// The window the app booted into.
    ///
    /// Needed by anything that talks about the window *as a window* rather than
    /// as a viewport — a display change, for one, which is addressed to a window
    /// and not to the tree drawn inside it.
    pub fn window(&self) -> WindowId {
        self.window
    }

    /// The screen rect `node` occupies.
    pub fn bounds(&self, node: impl Into<Selector>) -> Rect {
        self.host().bounds(self.node(node))
    }

    /// The point a click on `node` should aim at: the middle of its box.
    pub fn centre(&self, node: impl Into<Selector>) -> Point {
        let bounds = self.bounds(node);
        Point::new(
            bounds.x0 + bounds.width() / 2.0,
            bounds.y0 + bounds.height() / 2.0,
        )
    }

    /// Number of open overlays.
    pub fn overlay_count(&self) -> usize {
        self.host().overlay_count()
    }

    /// Whether the last frame left the scene with something to paint.
    pub fn damaged(&self) -> bool {
        !self.host().scene().damage().is_empty()
    }

    /// Every scene node the last paint produced, in paint order.
    pub fn scene_nodes(&self) -> Vec<SceneNode> {
        self.host()
            .scene()
            .iter()
            .map(|(_, node)| node.clone())
            .collect()
    }

    /// Ids of every scene node the last paint produced, in paint order.
    pub fn scene_ids(&self) -> Vec<SceneNodeId> {
        self.host().scene().iter().map(|(id, _)| id).collect()
    }

    /// The label `node` publishes to assistive technology, if any.
    pub fn label(&self, node: impl Into<Selector>) -> Option<String> {
        let node = self.node(node);
        self.host()
            .live_semantics_hint(node)
            .and_then(|hint| hint.label)
    }

    /// Every non-empty label currently on screen, in mount order. Handy in an
    /// assertion message, and enough to assert that *something* says a thing
    /// without pointing at which node.
    ///
    /// Only mounted nodes count. An unmounted subtree keeps its widget and its
    /// label in the host's slot table — unmounting flags a node, it does not
    /// free it — so a search that did not check would report text that is no
    /// longer on screen.
    pub fn labels(&self) -> Vec<String> {
        let host = self.host();
        (0..host.node_count())
            .filter_map(|index| host.node_at(index))
            .filter_map(|node| host.live_semantics_hint(node))
            .filter_map(|hint| hint.label)
            .filter(|label| !label.is_empty())
            .collect()
    }

    // --- selectors ---------------------------------------------------------

    /// Resolves `selector` to a node, or `None`.
    ///
    /// Only mounted nodes are candidates, so a stale handle does not resolve:
    /// an unmounted subtree keeps its widget in the host's slot table, and
    /// matching against it would let a test drive something that is not on
    /// screen.
    pub fn find(&self, selector: impl Into<Selector>) -> Option<NodeId> {
        let host = self.host();
        match selector.into() {
            Selector::Id(node) => host.is_mounted(node).then_some(node),
            Selector::Label(label) => (0..host.node_count())
                .filter_map(|index| host.node_at(index))
                .find(|&node| {
                    host.live_semantics_hint(node)
                        .and_then(|hint| hint.label)
                        .is_some_and(|found| found == label)
                }),
            Selector::Role(role) => (0..host.node_count())
                .filter_map(|index| host.node_at(index))
                .find(|&node| {
                    host.live_semantics_hint(node)
                        .is_some_and(|hint| hint.role == role)
                }),
        }
    }

    /// Resolves `selector`, panicking with what *is* on screen if it misses.
    ///
    /// Failing loudly is the point: a selector that silently resolved to
    /// nothing would turn a broken test into a passing one.
    #[track_caller]
    pub fn node(&self, selector: impl Into<Selector>) -> NodeId {
        let selector = selector.into();
        self.find(selector.clone()).unwrap_or_else(|| {
            panic!(
                "no node for {selector:?}; the screen publishes {:?}",
                self.labels()
            )
        })
    }

    /// The first mounted node holding a widget of type `T`.
    pub fn find_widget<T: WidgetAny + 'static>(&self) -> Option<NodeId> {
        let host = self.host();
        (0..host.node_count())
            .filter_map(|index| host.node_at(index))
            .find(|&node| host.is_mounted(node) && host.widget::<T>(node).is_some())
    }

    /// The widget behind `selector`, typed.
    #[track_caller]
    pub fn widget<T: WidgetAny + 'static>(&self, selector: impl Into<Selector>) -> &T {
        let node = self.node(selector);
        self.host()
            .widget::<T>(node)
            .unwrap_or_else(|| panic!("the node at {node:?} is not a {}", type_name::<T>()))
    }

    /// The widget behind `selector`, mutably — the door for changing app state
    /// the way a callback would.
    #[track_caller]
    pub fn widget_mut<T: WidgetAny + 'static>(&mut self, selector: impl Into<Selector>) -> &mut T {
        let node = self.node(selector);
        let name = type_name::<T>();
        self.host_mut()
            .widget_mut::<T>(node)
            .unwrap_or_else(|| panic!("the node at {node:?} is not a {name}"))
    }

    /// Whether `node` currently holds the keyboard.
    pub fn is_focused(&self, node: impl Into<Selector>) -> bool {
        let host = self.host();
        host.focused() == Some(host.node_id(self.node(node)))
    }

    /// Hands the keyboard to `node` directly.
    ///
    /// This is the app telling the framework where focus is, not a click: no
    /// pointer event is synthesised and the widget's own press handling does
    /// not run. Use [`TestApp::tap`] when the point is that clicking focuses
    /// something.
    pub fn focus(&mut self, node: impl Into<Selector>) {
        let node = self.node(node);
        let id = self.host().node_id(node);
        self.host_mut().set_focus(id);
    }

    // --- assertions --------------------------------------------------------

    /// Asserts what `node` announces to assistive technology.
    ///
    /// That is the visible text for a `Text`, `Button`, `Checkbox`; the value
    /// for a `TextField`; the selected option for a `Select`. Reading it back
    /// through the accessibility label rather than by downcasting means the
    /// assertion holds regardless of which widget is doing the announcing.
    #[track_caller]
    pub fn assert_text(&self, node: impl Into<Selector>, expected: &str) {
        let node = self.node(node);
        let found = self
            .host()
            .live_semantics_hint(node)
            .and_then(|hint| hint.label);
        assert_eq!(
            found.as_deref(),
            Some(expected),
            "the node at {node:?} announces something else"
        );
    }

    /// Asserts that something on screen announces `expected`.
    #[track_caller]
    pub fn assert_on_screen(&self, expected: &str) {
        assert!(
            self.labels().iter().any(|label| label == expected),
            "nothing on screen says {expected:?}; it says {:?}",
            self.labels()
        );
    }

    // --- actors ------------------------------------------------------------

    /// Renders every frame the platform has queued, reporting whether any of
    /// them had damage to paint.
    pub fn drain(&mut self) -> bool {
        let mut events = Vec::new();
        self.platform.poll_events(&mut events);
        let mut repainted = false;
        for event in events {
            self.runner.handle(&mut self.platform, event);
            if !self.runner.host().scene().damage().is_empty() {
                repainted = true;
            }
        }
        repainted
    }

    /// Runs one host frame directly, without the platform round trip.
    ///
    /// Enough to lay out and paint freshly mounted or resized content for
    /// hit-testing and damage assertions; a real interaction should go through
    /// [`TestApp::drain`] instead, so that the events are really delivered.
    pub fn frame(&mut self) -> FrameStats {
        self.host_mut().frame()
    }

    /// Runs frames until the tree stops asking for them, and asserts that it
    /// did.
    ///
    /// Returns whether anything was repainted on the way. A tree that never
    /// converges is a bug — an animation that does not finish, a repaint that
    /// damages the scene on every frame — so this fails rather than waiting.
    ///
    /// A change made *outside* the event loop (an app callback, a service, a
    /// timer) is the case worth naming: it leaves the tree owing a frame with
    /// nothing queued to deliver it, so waiting for the platform to speak would
    /// wait forever. When that happens this asks for a frame, which is exactly
    /// what an embedder does with [`AppRunner::request_frame`].
    #[track_caller]
    pub fn settle(&mut self) -> bool {
        let mut painted = false;
        for _ in 0..SETTLE_ROUNDS {
            if self.drain() {
                painted = true;
            } else if self.host().needs_frame() {
                self.runner.request_frame(&mut self.platform);
            } else {
                return painted;
            }
        }
        panic!(
            "the tree never settled: it is still asking for frames after {SETTLE_ROUNDS} of them"
        );
    }

    /// Delivers a raw platform event and drains the frames it asked for.
    pub fn send(&mut self, event: PlatformEvent) -> bool {
        self.runner.handle(&mut self.platform, event);
        self.drain()
    }

    pub fn press(&mut self, at: Point) -> bool {
        self.button(at, true)
    }

    pub fn release(&mut self, at: Point) -> bool {
        self.button(at, false)
    }

    /// A press and a release at `at` — the two events a click is.
    pub fn click(&mut self, at: Point) -> bool {
        let pressed = self.press(at);
        self.release(at) || pressed
    }

    /// Clicks the middle of `node`'s box.
    pub fn tap(&mut self, node: impl Into<Selector>) -> bool {
        self.click(self.centre(node))
    }

    pub fn move_to(&mut self, at: Point) -> bool {
        let window = self.window;
        self.send(PlatformEvent::PointerMoved {
            window,
            position: at,
            modifiers: Modifiers::NONE,
        })
    }

    /// Presses at `from`, moves to `to`, releases — a drag is those three.
    pub fn drag(&mut self, from: Point, to: Point) -> bool {
        let pressed = self.press(from);
        let moved = self.move_to(to);
        self.release(to) || moved || pressed
    }

    pub fn button(&mut self, at: Point, pressed: bool) -> bool {
        let window = self.window;
        self.send(PlatformEvent::PointerButton {
            window,
            position: at,
            button: PointerButton::Primary,
            pressed,
            modifiers: Modifiers::NONE,
        })
    }

    /// A key press with no modifiers, delivered to whatever holds focus.
    pub fn key(&mut self, code: KeyCode) -> bool {
        self.key_with(code, Modifiers::NONE)
    }

    pub fn key_with(&mut self, code: KeyCode, modifiers: Modifiers) -> bool {
        let window = self.window;
        self.send(PlatformEvent::Key {
            window,
            code,
            pressed: true,
            repeat: false,
            modifiers,
        })
    }

    /// Committed text — the path a real keyboard's composed input takes.
    ///
    /// The runner expands it into one character key-down per scalar, so this is
    /// also how a test types plain ASCII; there is no separate path.
    pub fn commit(&mut self, text: &str) -> bool {
        let window = self.window;
        self.send(PlatformEvent::ImeCommit {
            window,
            text: text.into(),
        })
    }

    /// Puts the keyboard on `node` and types `text` into it.
    ///
    /// Focus is taken directly rather than by clicking first: a click on a
    /// `Select` opens a list, and a test that wanted to type would be testing
    /// two things at once. Click-to-focus is the thing
    /// [`TestApp::tap`] covers.
    pub fn type_text(&mut self, node: impl Into<Selector>, text: &str) -> bool {
        self.focus(node);
        self.commit(text)
    }

    /// An in-progress composition on the focused text widget.
    pub fn preedit(&mut self, text: &str) -> bool {
        let window = self.window;
        self.send(PlatformEvent::ImePreedit {
            window,
            text: text.into(),
        })
    }

    pub fn resize(&mut self, size: Size) {
        let window = self.window;
        self.size = size;
        self.runner.handle(
            &mut self.platform,
            PlatformEvent::WindowResized { window, size },
        );
    }
}

/// `std::any::type_name` without the import dance at every call site.
fn type_name<T: ?Sized>() -> &'static str {
    std::any::type_name::<T>()
}

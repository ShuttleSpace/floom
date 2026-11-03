//! A rebuilt keyed subtree must keep the scene stream in paint order.
//!
//! Slot order is draw order. When a shell swaps a keyed child — the shape a
//! router-backed shell has — the old page's slots are freed and the new page's
//! nodes recycle them. The unmount walk frees in post-order while the paint
//! walk allocates in pre-order, so handing the freed slots out in anything but
//! lowest-first order interleaves them: a background lands on a slot *above*
//! its own content's, and paints over it — a page that is there (hit-testing
//! works) but shows only its background. The tree here is branched on purpose:
//! a straight chain frees in pre-order and cannot catch the bug.

use floom_core::{
    Axis, Child, ChildKey, CreateContext, ElementId, LayoutRequest, Size, UpdateContext, Widget,
    WidgetAny,
};
use floom_scene::SceneNode;
use floom_testkit::TestApp;
use floom_types::Color;
use floom_widgets::{Button, Stack, Surface, Text};

const VIEWPORT: Size = Size::new(240.0, 320.0);

const PAGE_BG: Color = Color::rgb(0xF4, 0xF6, 0xF8);
const BUTTON_BG: Color = Color::rgb(0x2B, 0x6C, 0xB0);

/// A shell that re-declares its one keyed child when the page changes.
struct Pages {
    page: u64,
    rendered: u64,
}

impl Pages {
    fn set_page(&mut self, page: u64) {
        self.page = page;
    }
}

impl Widget for Pages {
    fn create(&self, cx: &mut dyn CreateContext) -> ElementId {
        cx.alloc_child()
    }

    fn update(&mut self, cx: &mut dyn UpdateContext, id: ElementId) {
        if self.page != self.rendered {
            self.rendered = self.page;
            cx.mark_children_dirty(id);
        }
    }

    fn children(&self) -> Vec<Child> {
        let page = self.rendered;
        vec![Child::new(
            ChildKey::keyed(page),
            LayoutRequest::new().width(200.0).height(200.0),
            move || Box::new(page_body()),
        )]
    }
}

impl WidgetAny for Pages {
    fn as_any(&self) -> &dyn std::any::Any {
        self
    }

    fn as_any_mut(&mut self) -> &mut dyn std::any::Any {
        self
    }

    fn semantics_hint(&self) -> Option<floom_core::SemanticsHint> {
        None
    }
}

fn slot(
    key: &'static str,
    width: f32,
    height: f32,
    build: impl Fn() -> Box<dyn WidgetAny> + Send + Sync + 'static,
) -> Child {
    Child::new(
        ChildKey::from(key),
        LayoutRequest::new().width(width).height(height),
        build,
    )
}

/// One page, branched the way a real one is: two siblings, the second of
/// which wraps content in a painted background — the shape whose background
/// covers its own content when the stream order breaks.
fn page_body() -> Stack {
    Stack::new(Axis::Vertical)
        .child(slot("heading", 200.0, 24.0, || {
            Box::new(Text::new("Components"))
        }))
        .child(slot("card", 200.0, 120.0, || {
            Box::new(
                Surface::new()
                    .background(PAGE_BG)
                    .child(slot("button", 96.0, 30.0, || {
                        Box::new(Button::new("Primary").color(BUTTON_BG).radius(0.0))
                    })),
            )
        }))
}

/// `(stream index, solid fill)` for every rect in the scene.
fn rect_fills(app: &TestApp) -> Vec<(usize, Option<Color>)> {
    app.scene_nodes()
        .into_iter()
        .enumerate()
        .filter_map(|(i, node)| match node {
            SceneNode::Rect { fill, .. } => Some((i, fill.solid())),
            _ => None,
        })
        .collect()
}

fn assert_background_behind_button(app: &TestApp) {
    let fills = rect_fills(app);
    let bg = fills
        .iter()
        .position(|(_, fill)| *fill == Some(PAGE_BG))
        .expect("the page background is in the scene");
    let button = fills
        .iter()
        .position(|(_, fill)| *fill == Some(BUTTON_BG))
        .expect("the button fill is in the scene");
    assert!(
        bg < button,
        "the page background (slot {bg}) must paint before its own button (slot {button})"
    );
}

#[test]
fn a_swapped_keyed_page_keeps_its_background_behind_its_content() {
    let mut app = TestApp::unpinned(
        VIEWPORT,
        Pages {
            page: 0,
            rendered: 0,
        },
    );
    app.frame();
    assert_background_behind_button(&app);

    // Page 1 frees page 0's slots; returning to page 0 recycles them. The
    // unmount walk frees in post-order, so this is the ordering hazard.
    app.widget_mut::<Pages>(app.root()).set_page(1);
    app.frame();
    assert_background_behind_button(&app);

    app.widget_mut::<Pages>(app.root()).set_page(0);
    app.frame();
    assert_background_behind_button(&app);
}

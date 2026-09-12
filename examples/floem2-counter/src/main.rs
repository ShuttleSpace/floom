//! The first runnable Floem 2.0 app.
//!
//! There is no window here yet — `floem-app` is a *headless* shell, so this
//! example mounts a widget tree, pumps frames and prints what happened. It is
//! the smallest end-to-end exercise of the 2.0 stack:
//!
//! ```text
//! widgets → layout (taffy) → scene (retained) → semantics tree
//! ```
//!
//! Run with `cargo run -p floem2-counter`.

use floem_app::{NodeId, WidgetHost};
use floem_core::{Color, Point, Size};
use floem_layout::{AlignItems, FlexDirection, JustifyContent, LayoutStyle};
use floem_types::{Key, KeyEvent, PointerEvent};
use floem_widgets::{Button, Container, Text};

const VIEWPORT: Size = Size {
    width: 240.0,
    height: 160.0,
};

fn main() {
    let mut host = WidgetHost::new();
    host.set_size(VIEWPORT);

    // --- build the tree ----------------------------------------------------
    let root = host.mount_root(
        LayoutStyle {
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            gap: 8.0,
            ..LayoutStyle::default()
        },
        Container::new().background(Color::WHITE),
    );

    let label = host.mount(
        root,
        LayoutStyle {
            width: Some(160.0),
            height: Some(20.0),
            ..LayoutStyle::default()
        },
        Text::new("Count: 0"),
    );
    let increment = host.mount(
        root,
        LayoutStyle {
            width: Some(120.0),
            height: Some(28.0),
            ..LayoutStyle::default()
        },
        Button::new("Increment"),
    );

    let mut count = 0_i32;
    render(&mut host, label, count);

    println!("viewport      : {}x{}", VIEWPORT.width, VIEWPORT.height);
    println!("mounted nodes : {}", host.node_count());
    println!("root bounds   : {:?}", host.bounds(root));
    println!("label bounds  : {:?}", host.bounds(label));
    println!("button bounds : {:?}", host.bounds(increment));
    println!();

    // --- click the button with the pointer ---------------------------------
    for _ in 0..3 {
        click(&mut host, increment);
        count += 1;
        render(&mut host, label, count);
    }
    println!("after 3 clicks: {}", text_of(&host, label));

    // --- the focused control also responds to the keyboard ------------------
    host.dispatch(KeyEvent::down(Key::Char(' ')).into());
    count += 1;
    render(&mut host, label, count);
    println!("after Space   : {}", text_of(&host, label));
    println!("focused       : {:?}", host.focused());
    println!();

    // --- what the renderer and screen reader would receive -----------------
    let stats = host.last_stats();
    println!("last frame    : {stats:?}");
    println!("scene nodes   : {}", host.scene().len());
    println!(
        "a11y tree     : {} nodes, root {:?}",
        host.semantics().node_count(),
        host.semantics().root()
    );
}

/// Pushes the current count into the label and pumps a frame.
fn render(host: &mut WidgetHost, label: NodeId, count: i32) {
    let text = format!("Count: {count}");
    host.widget_mut::<Text>(label)
        .expect("label is a Text")
        .set_content(text);
    host.frame();
}

fn text_of(host: &WidgetHost, label: NodeId) -> String {
    host.widget::<Text>(label)
        .expect("label is a Text")
        .content()
        .to_string()
}

fn click(host: &mut WidgetHost, node: NodeId) {
    let bounds = host.bounds(node);
    let centre = Point::new(
        bounds.x0 + bounds.width() / 2.0,
        bounds.y0 + bounds.height() / 2.0,
    );
    host.dispatch(PointerEvent::primary_down(centre).into());
    host.dispatch(PointerEvent::primary_up(centre).into());
}

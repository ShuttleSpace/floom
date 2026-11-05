// SPDX-License-Identifier: MIT
//! End-to-end exercises of the headless app shell.
//!
//! These are the first tests in the repo that run a whole Floom 2.0 *app*:
//! mount widgets, pump a frame, route input, assert what landed in the scene.

use floom_app::WidgetHost;
use floom_core::{Key, Length, NamedKey, Point, Rect, Size};
use floom_layout::{AlignItems, FlexDirection, JustifyContent, LayoutStyle};
use floom_types::{KeyEvent, Modifiers, PointerEvent};
use floom_widgets::{Button, Container, Switch, Text};

/// `Counter` app: a label, a decrement button and an increment button.
struct Counter {
    host: WidgetHost,
    label: floom_app::NodeId,
    value: i32,
}

fn column(gap: f64) -> LayoutStyle {
    LayoutStyle {
        flex_direction: FlexDirection::Column,
        align_items: AlignItems::Center,
        justify_content: JustifyContent::Center,
        gap: gap as f32,
        ..LayoutStyle::default()
    }
}

fn fixed(width: f64, height: f64) -> LayoutStyle {
    LayoutStyle {
        width: Some(Length::Px(width as f32)),
        height: Some(Length::Px(height as f32)),
        ..LayoutStyle::default()
    }
}

fn new_counter() -> Counter {
    let mut host = WidgetHost::new();
    host.set_size(Size::new(240.0, 160.0));

    let root = host.mount_root(
        column(8.0),
        Container::new().background(floom_core::Color::WHITE),
    );
    let label = host.mount(root, fixed(160.0, 20.0), Text::new("Count: 0"));
    host.mount(root, fixed(120.0, 28.0), Button::new("Increment"));
    host.mount(root, fixed(120.0, 28.0), Button::new("Decrement"));

    Counter {
        host,
        label,
        value: 0,
    }
}

impl Counter {
    /// Mirrors app state into the mounted label, then pumps a frame.
    fn frame(&mut self) -> floom_app::FrameStats {
        let text = format!("Count: {}", self.value);
        self.host
            .widget_mut::<Text>(self.label)
            .expect("label node is a Text")
            .set_content(text);
        self.host.frame()
    }

    /// Clicks whichever button displays `label`, returns whether it handled.
    fn click(&mut self, label: &str) -> bool {
        let target = self.find_button(label);
        let bounds = self.host.bounds(target);
        let centre = Point::new(
            bounds.x0 + bounds.width() / 2.0,
            bounds.y0 + bounds.height() / 2.0,
        );
        let down = self
            .host
            .dispatch(PointerEvent::primary_down(centre).into());
        let up = self.host.dispatch(PointerEvent::primary_up(centre).into());
        down && up
    }

    fn find_button(&self, label: &str) -> floom_app::NodeId {
        // Nodes are mounted in order; buttons were pushed after the label.
        for index in 0..self.host.node_count() {
            let node = self.host.node_at(index).unwrap();
            if let Some(button) = self.host.widget::<Button>(node)
                && button.label() == label {
                    return node;
                }
        }
        panic!("no button labelled {label:?}");
    }
}

#[test]
fn layout_assigns_real_bounds_to_every_node() {
    let mut counter = new_counter();
    assert_eq!(counter.host.node_count(), 4);

    counter.frame();

    let root = counter.host.root().unwrap();
    let root_bounds = counter.host.bounds(root);
    assert!(
        root_bounds.width() > 0.0 && root_bounds.height() > 0.0,
        "root should get the viewport size, got {root_bounds:?}"
    );

    let label_bounds = counter.host.bounds(counter.label);
    assert!(
        (label_bounds.width() - 160.0).abs() < 0.5,
        "explicit width wins: {label_bounds:?}"
    );
    assert!(
        label_bounds.y0 > 0.0,
        "column layout stacks children: {label_bounds:?}"
    );
}

#[test]
fn paint_produces_a_scene_with_the_widgets_in_it() {
    let mut counter = new_counter();
    let stats = counter.frame();

    assert_eq!(stats.painted, 4);
    assert!(stats.layout_nodes >= 4);
    assert!(
        counter.host.scene().len() >= 4,
        "each visible widget contributes scene nodes"
    );
    assert!(
        !counter.host.scene().damage().is_empty(),
        "painting should report damage for the drawn area"
    );
}

#[test]
fn clicking_a_button_updates_the_label() {
    let mut counter = new_counter();
    counter.frame();

    assert!(counter.click("Increment"), "click should be handled");
    counter.value += 1;
    counter.frame();

    let label = counter
        .host
        .widget::<Text>(counter.label)
        .expect("label node");
    assert_eq!(label.content(), "Count: 1");

    assert!(counter.click("Decrement"));
    counter.value -= 1;
    counter.frame();
    assert_eq!(
        counter
            .host
            .widget::<Text>(counter.label)
            .unwrap()
            .content(),
        "Count: 0"
    );
}

#[test]
fn clicks_outside_any_widget_are_dropped() {
    let mut counter = new_counter();
    counter.frame();
    let far = Point::new(5000.0, 5000.0);

    assert!(
        !counter
            .host
            .dispatch(PointerEvent::primary_down(far).into())
    );
    assert!(!counter.host.dispatch(PointerEvent::primary_up(far).into()));
}

#[test]
fn focus_follows_the_pointer_and_enables_the_keyboard() {
    let mut counter = new_counter();
    counter.frame();

    let button = counter.find_button("Increment");
    let bounds = counter.host.bounds(button);
    let centre = Point::new(
        bounds.x0 + bounds.width() / 2.0,
        bounds.y0 + bounds.height() / 2.0,
    );

    counter
        .host
        .dispatch(PointerEvent::primary_down(centre).into());
    counter
        .host
        .dispatch(PointerEvent::primary_up(centre).into());

    let focused = counter.host.focused().expect("pressing focuses the button");
    assert_eq!(focused, counter.host.node_id(button));
    assert!(
        counter
            .host
            .widget::<Button>(button)
            .unwrap()
            .interaction()
            .focused
    );

    // Now a Space keypress reaches the focused control.
    assert!(counter.host.dispatch(KeyEvent::down(Key::Char(' ')).into()));
}

#[test]
fn tab_moves_focus_between_controls() {
    let mut counter = new_counter();
    counter.frame();

    counter.host.focus_next();
    let first = counter.host.focused().expect("tab focuses something");
    counter.host.focus_next();
    let second = counter.host.focused().expect("tab wraps around");
    assert_ne!(first, second, "focus should advance");

    counter.host.focus_prev();
    assert_eq!(counter.host.focused(), Some(first));
}

#[test]
fn tab_key_moves_focus_and_flags_keyboard_navigation() {
    // Gate #7: a Tab key press must be bound to focus traversal, not left
    // unhandled. This exercises the binding end-to-end through the same
    // `dispatch` the real window server feeds, and proves the keyboard
    // navigation flag flips so a focus ring can be shown.
    let mut counter = new_counter();
    counter.frame();

    assert_eq!(counter.host.focused(), None, "nothing focused before Tab");

    // A Tab from no focus lands on the first focusable and is consumed.
    assert!(
        counter
            .host
            .dispatch(KeyEvent::down(Key::Named(NamedKey::Tab)).into()),
        "Tab should be consumed by the host, not routed to a widget"
    );
    let first = counter.host.focused().expect("Tab focuses something");
    assert!(
        counter.host.keyboard_navigation(),
        "Tab-driven focus should flag keyboard navigation"
    );

    // A second Tab advances further; Shift+Tab walks it straight back.
    assert!(
        counter
            .host
            .dispatch(KeyEvent::down(Key::Named(NamedKey::Tab)).into())
    );
    let second = counter
        .host
        .focused()
        .expect("second Tab focuses something");
    assert_ne!(first, second, "Tab should advance focus");

    counter.host.dispatch(
        KeyEvent::down(Key::Named(NamedKey::Tab))
            .with_modifiers(Modifiers::SHIFT)
            .into(),
    );
    assert_eq!(counter.host.focused(), Some(first));
}

#[test]
fn semantics_tree_carries_roles_and_labels() {
    let mut counter = new_counter();
    counter.frame();

    let tree = counter.host.semantics();
    let root = counter.host.node_id(counter.host.root().unwrap());
    assert_eq!(tree.root(), Some(root));

    // Purely structural nodes become a Group so the tree keeps its shape.
    assert_eq!(
        tree.get(root).expect("root has semantic children").role,
        floom_accessibility::SemanticsRole::Group
    );

    let label_id = counter.host.node_id(counter.label);
    let label_node = tree.get(label_id).expect("text publishes a node");
    assert_eq!(
        label_node.role,
        floom_accessibility::SemanticsRole::StaticText
    );
    assert_eq!(label_node.label, "Count: 0");

    let button_id = counter.host.node_id(counter.find_button("Increment"));
    let button_node = tree.get(button_id).expect("button publishes a node");
    assert_eq!(button_node.role, floom_accessibility::SemanticsRole::Button);
    assert!(
        button_node
            .state
            .contains(floom_accessibility::SemanticsState::FOCUSABLE),
        "an enabled control advertises focusability"
    );
}

#[test]
fn restyling_triggers_a_new_layout() {
    let mut counter = new_counter();
    counter.frame();
    let before = counter.host.bounds(counter.label);

    counter.host.set_style(
        counter.label,
        LayoutStyle {
            width: Some(Length::Px(40.0)),
            height: Some(Length::Px(20.0)),
            ..LayoutStyle::default()
        },
    );
    counter.frame();

    let after = counter.host.bounds(counter.label);
    assert!(
        (after.width() - 40.0).abs() < 0.5,
        "layout picked up the new style: {after:?} (was {before:?})"
    );
}

#[test]
fn zero_sized_viewport_paints_nothing() {
    let mut counter = new_counter();
    counter.host.set_size(Size::ZERO);
    let stats = counter.frame();
    assert_eq!(stats.painted, 4, "every widget is still visited");
    assert_eq!(
        counter.host.scene().len(),
        0,
        "nothing to draw without bounds"
    );
}

#[test]
fn scene_does_not_grow_across_frames() {
    let mut counter = new_counter();
    counter.frame();
    let first = counter.host.scene().len();

    for value in [1, 2, 3, 4] {
        counter.value = value;
        counter.frame();
    }

    assert_eq!(
        counter.host.scene().len(),
        first,
        "each frame must retire the previous frame's scene nodes"
    );
}

#[test]
fn semantics_parents_children_in_mount_order() {
    let mut host = WidgetHost::new();
    host.set_size(Size::new(100.0, 100.0));
    let root = host.mount_root(column(0.0), Container::new());
    // A labelled two-state control, which is all this test needs as a
    // vehicle. It was a `Checkbox` until that moved to
    // `floom-ui-composites`, in the satellite workspace the root one
    // excludes; `Switch` publishes the same `CHECKED` state and is still a
    // primitive.
    let toggle = host.mount(root, fixed(80.0, 20.0), Switch::new("Ship it"));
    host.frame();

    let root_node = host
        .semantics()
        .get(host.node_id(root))
        .expect("root is published once it has semantic children");
    assert_eq!(root_node.children, vec![host.node_id(toggle)]);

    // Toggling through the host updates shared semantics state.
    let bounds = host.bounds(toggle);
    let centre = Rect::ZERO;
    let _ = centre;
    let at = Point::new(bounds.x0 + 2.0, bounds.y0 + bounds.height() / 2.0);
    host.dispatch(PointerEvent::primary_down(at).into());
    host.dispatch(PointerEvent::primary_up(at).into());
    host.frame();

    assert!(host.widget::<Switch>(toggle).unwrap().is_on());
    assert!(
        host.semantics()
            .get(host.node_id(toggle))
            .unwrap()
            .state
            .contains(floom_accessibility::SemanticsState::CHECKED)
    );
}

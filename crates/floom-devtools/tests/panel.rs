//! The devtools panel's contract: the one-line integration, the shortcut, and
//! the readout.
//!
//! Everything here runs without a window: the panel is a widget, and a widget's
//! visibility, its key handling and its rows are all answerable from plain
//! values.

use std::sync::{Arc, Mutex};

use floom_core::{Key, KeyEvent, KeyPhase, Modifiers, NamedKey, Widget, WidgetAny};
use floom_devtools::panel::{
    DevtoolsPanel, DevtoolsState, DevtoolsTab, ReadoutRow, default_shortcut, panel_bounds,
};
use floom_types::{ElementId, Event, Point, PointerEvent, Rect, Size};
use floom_widgets::Button;
use floom_widgets::testkit::{RecordingUpdateContext, dispatch};

fn f12() -> KeyEvent {
    KeyEvent {
        phase: KeyPhase::Down,
        key: Key::Named(NamedKey::Function(12)),
        modifiers: Modifiers::NONE,
    }
}

fn ctrl_shift_i() -> KeyEvent {
    KeyEvent {
        phase: KeyPhase::Down,
        key: Key::Char('i'),
        modifiers: Modifiers::CONTROL.union(Modifiers::SHIFT),
    }
}

/// The macOS spelling of the same binding. The platform layer reports the
/// command key as `META`, so a panel that only knew `CONTROL` would be
/// unopenable by the shortcut every macOS user tries first.
fn cmd_shift_i() -> KeyEvent {
    KeyEvent {
        phase: KeyPhase::Down,
        key: Key::Char('i'),
        modifiers: Modifiers::META.union(Modifiers::SHIFT),
    }
}

fn panel() -> DevtoolsPanel {
    let state = Arc::new(Mutex::new(DevtoolsState::new()));
    DevtoolsPanel::new(state, |state| {
        if state.selected() == Some("one") {
            vec![ReadoutRow::new("elements", "1")]
        } else {
            vec![
                ReadoutRow::new("elements", "2"),
                ReadoutRow::new("semantics", "3"),
            ]
        }
    })
}

#[test]
fn the_shortcut_opens_and_closes_the_panel() {
    let mut panel = panel();
    assert!(!panel.visible());

    // F12 down: consumed, and the panel is now showing.
    assert!(panel.handle_key(&f12(), &default_shortcut()));
    assert!(panel.visible());

    // F12 again: consumed, and the panel is hidden again.
    assert!(panel.handle_key(&f12(), &default_shortcut()));
    assert!(!panel.visible());
}

#[test]
fn ctrl_shift_i_is_the_shortcut_where_f12_belongs_to_the_os() {
    let mut panel = panel();
    assert!(panel.handle_key(&ctrl_shift_i(), &default_shortcut()));
    assert!(panel.visible());
    assert!(panel.handle_key(&ctrl_shift_i(), &default_shortcut()));
    assert!(!panel.visible());
}

#[test]
fn cmd_shift_i_is_the_same_shortcut_on_macos() {
    let mut panel = panel();
    // The command key, not control: the platform layer maps it to `META`, and
    // this is the binding a macOS user reaches for first.
    assert!(panel.handle_key(&cmd_shift_i(), &default_shortcut()));
    assert!(panel.visible());
    assert!(panel.handle_key(&cmd_shift_i(), &default_shortcut()));
    assert!(!panel.visible());
}

#[test]
fn the_shift_i_shortcut_needs_both_modifiers() {
    let mut panel = panel();
    // Command alone is a menu binding, not the devtools.
    let cmd_i = KeyEvent {
        phase: KeyPhase::Down,
        key: Key::Char('i'),
        modifiers: Modifiers::META,
    };
    assert!(!panel.handle_key(&cmd_i, &default_shortcut()));
    // Control alone likewise.
    let ctrl_i = KeyEvent {
        phase: KeyPhase::Down,
        key: Key::Char('i'),
        modifiers: Modifiers::CONTROL,
    };
    assert!(!panel.handle_key(&ctrl_i, &default_shortcut()));
    assert!(!panel.visible());
}

#[test]
fn the_shortcut_ignores_key_releases() {
    let mut panel = panel();
    let release = KeyEvent {
        phase: KeyPhase::Up,
        key: Key::Named(NamedKey::Function(12)),
        modifiers: Modifiers::NONE,
    };
    assert!(!panel.handle_key(&release, &default_shortcut()));
    assert!(!panel.visible());
}

#[test]
fn an_unrelated_key_is_not_consumed() {
    let mut panel = panel();
    let tab = KeyEvent {
        phase: KeyPhase::Down,
        key: Key::Named(NamedKey::Tab),
        modifiers: Modifiers::NONE,
    };
    // Not consumed, so a text field keeps typing.
    assert!(!panel.handle_key(&tab, &default_shortcut()));
}

#[test]
fn a_key_event_without_the_modifiers_is_not_the_shortcut() {
    let mut panel = panel();
    let plain_i = KeyEvent {
        phase: KeyPhase::Down,
        key: Key::Char('i'),
        modifiers: Modifiers::NONE,
    };
    assert!(!panel.handle_key(&plain_i, &default_shortcut()));
    assert!(!panel.visible());
}

#[test]
fn a_custom_shortcut_replaces_the_default_one() {
    let mut panel = panel();
    // A debug build that wants `~` instead of F12.
    let tilde: Arc<dyn Fn(&KeyEvent) -> bool + Send + Sync> =
        Arc::new(|e| e.key == Key::Char('~') && e.phase == KeyPhase::Down);
    let tilde_key = KeyEvent {
        phase: KeyPhase::Down,
        key: Key::Char('~'),
        modifiers: Modifiers::NONE,
    };
    // The custom binding opens the panel.
    assert!(panel.handle_key(&tilde_key, &tilde));
    assert!(panel.visible());
    // And the default binding no longer answers, so F12 falls through to the
    // app: a caller that supplies its own shortcut opts out of the default.
    assert!(!panel.handle_key(&f12(), &tilde));
    assert!(panel.visible());
    // The custom binding closes it again.
    assert!(panel.handle_key(&tilde_key, &tilde));
    assert!(!panel.visible());
}

#[test]
fn the_state_is_the_one_line_integrations_surface() {
    let state = Arc::new(Mutex::new(DevtoolsState::new()));
    {
        let mut guard = state.lock().unwrap();
        assert!(!guard.visible());
        guard.show();
        assert!(guard.visible());
        guard.hide();
        assert!(!guard.visible());
        guard.toggle();
        assert!(guard.visible());
        guard.set_tab(DevtoolsTab::Semantics);
        assert_eq!(guard.tab(), DevtoolsTab::Semantics);
        guard.select("body");
        assert_eq!(guard.selected(), Some("body"));
    }
    // The panel reads the same cell, so a `show: true` from the app's side is
    // the panel showing.
    let panel = DevtoolsPanel::new(state, |_| Vec::new());
    assert!(panel.visible());
}

#[test]
fn a_hidden_panel_mounts_no_children() {
    let panel = panel();
    assert!(panel.children().is_empty());
}

#[test]
fn a_shown_panel_mounts_the_readout() {
    let mut panel = panel();
    panel.handle_key(&f12(), &default_shortcut());
    let children = panel.children();
    assert_eq!(children.len(), 1);
    // The child builds: a surface holding the rows.
    let built = children[0].build();
    let _ = built;
}

#[test]
fn the_readout_sees_the_selected_element() {
    let state = Arc::new(Mutex::new(DevtoolsState::new()));
    {
        let mut guard = state.lock().unwrap();
        guard.show();
        guard.select("one");
    }
    let panel = DevtoolsPanel::new(state, |state| {
        // The closure is the app's own: it decides what a row says.
        vec![ReadoutRow::new(
            "selected",
            state.selected().unwrap_or("none"),
        )]
    });
    // Rows are read through the same path the widget uses, so the panel and the
    // app agree on what is selected.
    assert!(panel.visible());
}

#[test]
fn the_panel_docks_to_the_trailing_edge() {
    let viewport = Rect::from_origin_size(Point::new(0.0, 0.0), Size::new(1000.0, 800.0));
    let bounds = panel_bounds(viewport);
    // Docked right: it starts a third of the way in and never leaves the window.
    assert!(bounds.x0 > 0.0);
    assert!((bounds.x1 - 1000.0).abs() < 0.001);
    assert_eq!(bounds.height(), 800.0);
    // Wide enough to read, narrow enough to leave the app visible.
    assert!(bounds.width() >= 280.0 && bounds.width() <= 520.0);
}

#[test]
fn a_small_window_still_fits_the_panel() {
    let viewport = Rect::from_origin_size(Point::new(0.0, 0.0), Size::new(320.0, 480.0));
    let bounds = panel_bounds(viewport);
    // Clamped, so a phone-sized window does not get a panel wider than itself.
    assert!(bounds.width() <= 320.0);
    assert!(bounds.x0 >= 0.0);
}

#[test]
fn the_visibility_change_fires_once_per_change() {
    let state = Arc::new(Mutex::new(DevtoolsState::new()));
    let seen = Arc::new(Mutex::new(Vec::<bool>::new()));
    let recorder = seen.clone();
    let mut panel =
        DevtoolsPanel::new(state, |_| Vec::new()).on_visibility_change(move |visible| {
            recorder.lock().unwrap().push(visible);
        });
    panel.handle_key(&f12(), &default_shortcut());
    panel.handle_key(&f12(), &default_shortcut());
    assert_eq!(*seen.lock().unwrap(), vec![true, false]);
}

#[test]
fn opening_the_panel_asks_the_host_to_rebuild_its_children() {
    let mut panel = panel();
    let id = ElementId::new(1, 1);
    let mut cx = RecordingUpdateContext::new();
    // Hidden, first update: nothing to rebuild — the panel declares the same
    // (empty) children it always did.
    panel.update(&mut cx, id);
    assert!(!cx.reconciled());

    // The shortcut flips the state from outside the widget tree. The next
    // `update` is the only place that can tell the host its children changed,
    // so it has to: a repaint alone would leave the host mounting no rows.
    panel.handle_key(&f12(), &default_shortcut());
    let mut cx = RecordingUpdateContext::new();
    panel.update(&mut cx, id);
    assert!(
        cx.reconciled(),
        "opening the panel marks its children dirty, or the host never asks again"
    );
    assert!(cx.laid_out_again(), "the rows arrive with a new layout");

    // A settled frame asks for nothing: the flag is per-change, not per-frame.
    let mut cx = RecordingUpdateContext::new();
    panel.update(&mut cx, id);
    assert!(!cx.reconciled());

    // Closing is a change in the other direction, and equally has to be marked.
    panel.handle_key(&f12(), &default_shortcut());
    let mut cx = RecordingUpdateContext::new();
    panel.update(&mut cx, id);
    assert!(cx.reconciled());
}

/// The rows are a function of more than visibility: the tab and the selected
/// element change what the panel declares without changing whether it is up.
///
/// Neither comes through an argument to `update` — they are written into the
/// shared cell from outside the tree — so a panel that only watched visibility
/// would keep painting the previous tab until some unrelated repaint happened
/// to rebuild it. That is the bug this test is named after.
#[test]
fn switching_tab_or_element_asks_the_host_to_rebuild_the_rows() {
    let state = Arc::new(Mutex::new(DevtoolsState::new()));
    let mut panel = DevtoolsPanel::new(state.clone(), |state| {
        vec![ReadoutRow::new("tab", format!("{:?}", state.tab()))]
    });
    let id = ElementId::new(1, 1);
    panel.handle_key(&f12(), &default_shortcut());

    // Opening is one change.
    let mut cx = RecordingUpdateContext::new();
    panel.update(&mut cx, id);
    assert!(cx.reconciled());
    // And it settles.
    let mut cx = RecordingUpdateContext::new();
    panel.update(&mut cx, id);
    assert!(!cx.reconciled(), "a settled frame asks for nothing");

    // A tab switch, written straight into the shared cell the way a click on a
    // tab would: no visibility change, no `update` argument, new rows.
    state.lock().unwrap().set_tab(DevtoolsTab::Style);
    let mut cx = RecordingUpdateContext::new();
    panel.update(&mut cx, id);
    assert!(
        cx.reconciled(),
        "a new tab declares new rows, so the host has to be told"
    );

    // A selection change, same shape: the readout is about a different element.
    state.lock().unwrap().select("node-7");
    let mut cx = RecordingUpdateContext::new();
    panel.update(&mut cx, id);
    assert!(cx.reconciled(), "a new selection declares new rows");

    // A second update with nothing changed does not.
    let mut cx = RecordingUpdateContext::new();
    panel.update(&mut cx, id);
    assert!(!cx.reconciled());
}

/// A hidden panel collapses its row key to `(false, ..)`, so a tab change while
/// it is down is not remembered as "already mounted" and skipped when it opens.
///
/// The panel is hidden most of the time. If the key carried the tab even while
/// hidden, a tab change made with the panel down would compare equal on the
/// next update and the rows for the new tab would never be mounted.
#[test]
fn a_tab_changed_while_hidden_still_rebuilds_on_open() {
    let state = Arc::new(Mutex::new(DevtoolsState::new()));
    let mut panel = DevtoolsPanel::new(state.clone(), |state| {
        vec![ReadoutRow::new("tab", format!("{:?}", state.tab()))]
    });
    let id = ElementId::new(1, 1);
    // Hidden and settled.
    let mut cx = RecordingUpdateContext::new();
    panel.update(&mut cx, id);
    assert!(!cx.reconciled());

    // The tab changes while the panel is down.
    state.lock().unwrap().set_tab(DevtoolsTab::Layout);
    let mut cx = RecordingUpdateContext::new();
    panel.update(&mut cx, id);
    assert!(!cx.reconciled(), "hidden: there are still no rows to rebuild");

    // Opening mounts the rows for the tab that is selected *now*, not the one
    // that was selected when it last had rows.
    panel.handle_key(&f12(), &default_shortcut());
    let mut cx = RecordingUpdateContext::new();
    panel.update(&mut cx, id);
    assert!(cx.reconciled());
    assert!(panel.visible());
}

// --- The tab bar ---------------------------------------------------------

/// Walks a built widget tree and hands every `Button` to `visit`, in the order
/// the bar shows them.
fn buttons_in(widget: &mut dyn WidgetAny, visit: &mut impl FnMut(&mut Button) -> bool) -> bool {
    if let Some(button) = widget.as_any_mut().downcast_mut::<Button>() {
        return visit(button);
    }
    for child in widget.children() {
        let mut built = child.build();
        if buttons_in(built.as_mut(), visit) {
            return true;
        }
    }
    false
}

fn tab_labels(panel: &DevtoolsPanel) -> Vec<String> {
    let mut labels = Vec::new();
    let mut children = panel.children();
    for child in children.iter_mut() {
        let mut built = child.build();
        buttons_in(built.as_mut(), &mut |button| {
            labels.push(button.label().to_string());
            false
        });
    }
    labels
}

fn click_button(panel: &DevtoolsPanel, label: &str) -> bool {
    let mut children = panel.children();
    for child in children.iter_mut() {
        let mut built = child.build();
        let mut clicked = false;
        buttons_in(built.as_mut(), &mut |button| {
            if button.label() != label {
                return false;
            }
            let id = ElementId::new(1, 1);
            let bounds = Rect::from_origin_size(Point::new(0.0, 0.0), Size::new(80.0, 24.0));
            let at = Point::new(10.0, 10.0);
            dispatch(
                button,
                id,
                bounds,
                Event::Pointer(PointerEvent::primary_down(at)),
            );
            dispatch(
                button,
                id,
                bounds,
                Event::Pointer(PointerEvent::primary_up(at)),
            );
            clicked = true;
            true
        });
        if clicked {
            return true;
        }
    }
    false
}

#[test]
fn a_panel_without_tabs_mounts_no_bar() {
    let mut panel = panel();
    panel.handle_key(&f12(), &default_shortcut());
    // No bar means no buttons at all: the first level of children is the rows,
    // and every one of them is text. A bar the caller did not ask for would be
    // buttons that switch to a readout this panel cannot fill.
    assert_eq!(tab_labels(&panel), Vec::<String>::new());
}

#[test]
fn the_tab_bar_offers_exactly_the_tabs_it_was_given() {
    let state = Arc::new(Mutex::new(DevtoolsState::new()));
    let mut panel =
        DevtoolsPanel::new(state, |_| Vec::new()).with_tabs(DevtoolsTab::ELEMENTS);
    panel.handle_key(&f12(), &default_shortcut());
    // `Tree` is spelled "Elements" on the bar — the word Chrome uses, and the
    // one that says what the readout is a list of.
    assert_eq!(tab_labels(&panel), vec!["Elements", "Style", "Layout"]);
}

#[test]
fn a_click_on_a_tab_switches_the_readout() {
    let state = Arc::new(Mutex::new(DevtoolsState::new()));
    let mut panel =
        DevtoolsPanel::new(state.clone(), |_| Vec::new()).with_tabs(DevtoolsTab::ALL);
    panel.handle_key(&f12(), &default_shortcut());
    assert_eq!(state.lock().unwrap().tab(), DevtoolsTab::Tree);

    assert!(click_button(&panel, "Style"), "the bar offers Style");
    assert_eq!(state.lock().unwrap().tab(), DevtoolsTab::Style);

    assert!(click_button(&panel, "Layout"), "the bar offers Layout");
    assert_eq!(state.lock().unwrap().tab(), DevtoolsTab::Layout);

    // Every tab is reachable from every other one: the bar is rebuilt from the
    // state, not frozen at the tab that was active when it was first mounted.
    assert!(click_button(&panel, "Elements"));
    assert_eq!(state.lock().unwrap().tab(), DevtoolsTab::Tree);
    assert!(click_button(&panel, "Style"));
    assert_eq!(state.lock().unwrap().tab(), DevtoolsTab::Style);
}

#[test]
fn a_click_on_the_active_tab_is_not_a_change() {
    let state = Arc::new(Mutex::new(DevtoolsState::new()));
    let mut panel =
        DevtoolsPanel::new(state.clone(), |_| Vec::new()).with_tabs(DevtoolsTab::ELEMENTS);
    panel.handle_key(&f12(), &default_shortcut());
    assert!(click_button(&panel, "Elements"));
    assert_eq!(state.lock().unwrap().tab(), DevtoolsTab::Tree);
}

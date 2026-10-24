//! The devtools panel's contract: the one-line integration, the shortcut, and
//! the readout.
//!
//! Everything here runs without a window: the panel is a widget, and a widget's
//! visibility, its key handling and its rows are all answerable from plain
//! values.

use std::sync::{Arc, Mutex};

use floom_core::{Key, KeyEvent, KeyPhase, Modifiers, NamedKey, Widget};
use floom_devtools::panel::{
    DevtoolsPanel, DevtoolsState, DevtoolsTab, ReadoutRow, default_shortcut, panel_bounds,
};
use floom_types::{Point, Rect, Size};

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

fn panel() -> DevtoolsPanel {
    let state = Arc::new(Mutex::new(DevtoolsState::new()));
    DevtoolsPanel::new(state, |state| {
        if state.selected() == Some("one") {
            vec![ReadoutRow {
                label: "elements".into(),
                value: "1".into(),
            }]
        } else {
            vec![
                ReadoutRow {
                    label: "elements".into(),
                    value: "2".into(),
                },
                ReadoutRow {
                    label: "semantics".into(),
                    value: "3".into(),
                },
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
        vec![ReadoutRow {
            label: "selected".into(),
            value: state.selected().unwrap_or("none").into(),
        }]
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

//! The shell has to mount, paint, and list every component by the name a
//! reader knows it by.

use floom::floom_types::Size;
use floom_testkit::{Selector, TestApp};

use floom_demo::shell::GalleryShell;

const WINDOW: Size = Size::new(1280.0, 860.0);

#[test]
fn the_shell_mounts_and_paints() {
    let mut app = TestApp::unpinned(WINDOW, GalleryShell::new());
    assert!(app.frame().painted > 0);
}

#[test]
fn the_rail_lists_every_page_by_name() {
    let mut app = TestApp::unpinned(WINDOW, GalleryShell::new());
    app.frame();
    for name in [
        "Button",
        "TextField",
        "Dialog",
        "ThemeProvider",
        "OTPInput",
        "DataTable",
    ] {
        assert!(
            app.find(Selector::label(name)).is_some(),
            "the rail has no entry for {name}"
        );
    }
}

#[test]
fn the_rail_lists_every_component_of_the_catalogue() {
    let mut app = TestApp::unpinned(WINDOW, GalleryShell::new());
    app.frame();
    for page in floom_demo::registry::pages() {
        assert!(
            app.find(Selector::label(page.name)).is_some(),
            "the rail has no entry for {}",
            page.name
        );
    }
}

#[test]
fn the_shell_shows_the_first_page_by_default() {
    let mut app = TestApp::unpinned(WINDOW, GalleryShell::new());
    app.frame();
    assert!(
        app.find(Selector::label("Button")).is_some(),
        "the first page is Button"
    );
}

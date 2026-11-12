//! The compare row is the gallery's headline: one component shown in all three
//! design systems at once. These tests hold that contract.

use floom::floom_types::Size;
use floom_testkit::TestApp;

use floom_demo::ds;
use floom_demo::macros::compare_children;
use floom_demo::registry;
use floom_demo::widgets::compare_row;

const WINDOW: Size = Size::new(1280.0, 860.0);

#[test]
fn the_button_page_shows_three_columns() {
    let page = registry::page_named("Button").expect("Button has a page");
    let mut app = TestApp::unpinned(WINDOW, page.build_shell());
    assert!(app.frame().painted > 0);
    for label in ["Material", "Cupertino", "Fluent"] {
        assert!(
            app.find(floom_testkit::Selector::label(label)).is_some(),
            "the compare row has no {label} column label"
        );
    }
}

#[test]
fn the_compare_macro_handles_an_argument_taking_terminal() {
    // Review Focus #2: `item_group`'s terminal takes two callbacks.
    let (m, c, f) = compare_children!(
        ds::material::ItemGroup,
        ds::cupertino::ItemGroup,
        ds::fluent::ItemGroup,
        |b, theme| b::new()
            .with_theme(theme)
            .view(None::<fn(usize)>, None::<fn(Vec<usize>)>)
    );
    let row = compare_row((m, c, f));
    let mut app = TestApp::unpinned(WINDOW, row);
    assert!(app.frame().painted > 0);
}

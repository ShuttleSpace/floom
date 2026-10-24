//! What this crate's API *is* now, replacing v1's neutral-contract test.
//!
//! v1's `api.rs` asserted five things about `FluentDialog`'s style struct. Four of them
//! are about surface this port does not have, and saying why is more useful than the
//! assertions were:
//!
//! | v1 field | here |
//! |---|---|
//! | `design_system: &'static str` | **dropped** |
//! | `scrim_color` | **dropped** — the host owns the scrim |
//! | `container_color` | `surface` |
//! | `content_color` | `on_surface` plus a separate `content_alpha` |
//! | `border_radius` | kept, but **not** from `DialogTokens` in this kit |
//!
//! ## Why each
//!
//! * `design_system` was a string nothing read. Dropping it is the same trade
//!   `md-treeview` and `md-combobox` made: a kit style struct that differs only by a name
//!   is a second thing to keep in step.
//! * **`scrim_color` is the interesting one.** v1's dialog drew its own dimmed backdrop
//!   over the window. 2.0's `OverlayKind::Dialog` is *modal*, and the host draws the
//!   scrim — which is also why clicking outside no longer closes the dialog, since the
//!   host deliberately does not light-dismiss a modal overlay. So this field is gone
//!   because the responsibility moved to the host, not because it was inconvenient.
//! * `content_color` was `with_alpha(on_surface, 222)` folded into one colour. Splitting
//!   the alpha out is what lets a test say the body text is *deliberately* lighter than
//!   the title, which a single colour cannot express.
//!
//! The two v1 tests' *intent* is kept: the builder chain is neutral, and the style
//! resolves from the kit's theme.

use floom_core_theme::theme::Theme;

use floom_widget_fluent_dialog::FluentDialog;

fn dialog() -> FluentDialog {
    FluentDialog::new().title("Confirm").content("Continue?")
}

/// The neutral contract: title, content, and open are settable and readable, and nothing
/// about the chain names a design system.
#[test]
fn the_builder_chain_is_neutral() {
    let dialog = dialog().open(true);
    let props = dialog.props();
    assert_eq!(props.title, "Confirm");
    assert_eq!(props.content, "Continue?");
    assert!(
        props.open,
        "and `.open(true)` writes the flag `props()` reports"
    );
}

/// The style resolves from the kit's theme.
///
/// `MultiTheme::new_for` ignores its argument and reads go through a process-global, so
/// naming `Fluent` here changes nothing — which is the same property correction 9 records
/// about every kit, and the reason this test cannot claim to be about this kit's theme.
#[test]
fn the_style_resolves_from_a_theme() {
    let theme = floom_core_theme::theme::MultiTheme::new_for(
        floom_core_theme::global_theme::DesignSystem::Fluent,
    );
    let style = dialog().with_theme(theme.clone()).resolved_style();

    assert_eq!(
        style.surface,
        floom_types::Color {
            r: theme.colors().surface.r,
            g: theme.colors().surface.g,
            b: theme.colors().surface.b,
            a: theme.colors().surface.a,
        },
        "`container_color` renamed to `surface`"
    );
    assert_eq!(
        style.content_alpha, 222,
        "and `content_color`'s alpha is now a field of its own, so the body can be \
         said to be lighter than the title rather than merely differently coloured"
    );
    // v1's `api.rs` asserted `max_width == 420.0` as a literal, and **it no longer
    // matches the token**, which is 560. So that assertion was stale in v1: the token
    // moved and the test did not. Reading the token here means this test goes stale the
    // same way a design system does rather than the other way round.
    let floom_core_theme::component_tokens::ComponentTokens::Dialog(tokens) =
        theme.component(floom_core_theme::component_tokens::ComponentId::Dialog)
    else {
        panic!("expected DialogTokens");
    };
    assert_eq!(style.max_width, tokens.max_width);
    assert_ne!(
        style.max_width, 420.0,
        "which is not the literal v1's own test asserted — that one had gone stale \
         against the token it was nominally checking"
    );
}

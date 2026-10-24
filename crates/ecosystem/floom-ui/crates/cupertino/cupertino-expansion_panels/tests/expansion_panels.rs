//! The ported Cupertino expansion panels — the *stateful* reference port.
//!
//! Where `md-card`'s test proved the theme→style→widget translation and
//! `md-button`'s pinned an interaction, this one pins what only a component that
//! *changes after mount* has: that the theme's tokens reach the engine widget,
//! that the panels and their bodies survive being shared into the tree, and that
//! the design system's single-panel rule is still in force after the trip.
//!
//! The state machine's own behaviour — mount, unmount, reconcile, and the two
//! things this port could not carry over — is pinned where it lives, in
//! `floom-widgets`' `compose::accordion_tests`. A kit test can only reach the
//! public surface of the engine widget, which is the right boundary: this file
//! proves the *translation*, not the engine.

use std::sync::atomic::{AtomicUsize, Ordering};
use std::sync::Arc;

use floom_core::{Axis, Widget};
use floom_types::Insets;
use floom_widgets::VectorIcon;

use floom_core_theme::component_tokens::{ComponentId, ComponentTokens, ExpansionPanelsTokens};
use floom_core_theme::global_theme::DesignSystem;
use floom_core_theme::props::expansion_panels::ExpansionPanelsProps;
use floom_core_theme::theme::{MultiTheme, Theme};
use floom_core_theme::tokens::color::Color;
use floom_widget_cupertino_expansion_panels::{
    CupertinoExpansionPanels, CupertinoExpansionPanelsStyle,
};

/// The Cupertino theme, with Cupertino selected.
///
/// `MultiTheme::new_for(ds)` does *not* select `ds` -- it builds all three and
/// `activate` is a documented no-op -- and a `MultiTheme` reads the *active*
/// design system at read time. So a test that wants Cupertino's own tokens has to
/// select Cupertino, and `set_design_system` writes a thread-local, so the
/// selection is this test's alone.
fn theme() -> MultiTheme {
    floom_core_theme::global_theme::set_design_system(DesignSystem::Cupertino);
    MultiTheme::new_for(DesignSystem::Cupertino)
}

/// The Cupertino panels, with Cupertino selected.
///
/// The selection has to happen before the kit resolves anything: the theme a kit
/// holds is a `MultiTheme`, and a `MultiTheme` reads the *active* design system
/// at read time -- so a kit built before the selection would report the default
/// system's tokens.
fn kit() -> CupertinoExpansionPanels {
    let _ = theme();
    CupertinoExpansionPanels::new()
}

/// The theme's own tokens, read through the same path the component reads them.
fn tokens() -> ExpansionPanelsTokens {
    let theme = theme();
    let ComponentTokens::ExpansionPanels(tokens) = theme.component(ComponentId::ExpansionPanels)
    else {
        unreachable!("ComponentId::ExpansionPanels must resolve to ExpansionPanelsTokens")
    };
    tokens
}

fn panels(count: usize) -> CupertinoExpansionPanels {
    let mut built = kit();
    for index in 0..count {
        built = built.panel(format!("Panel {index}"), || {
            Box::new(floom_widgets::Text::new("body"))
        });
    }
    built
}

// --- the tokens reach the engine widget ------------------------------------

/// The translation test: every metric the port resolves is the token it names.
/// `indent` is the one that could go wrong silently — v1 spells it
/// `content_padding + indent` for the body and nothing at all for the header, so
/// a mix-up shows up as content that is flush with the panel edge.
#[test]
fn the_resolved_style_is_the_theme_tokens() {
    let tokens = tokens();
    let style: CupertinoExpansionPanelsStyle = kit().resolved_style();
    assert_eq!(style.header_height, tokens.header_height);
    assert_eq!(style.header_padding, tokens.content_padding);
    assert_eq!(style.content_padding, tokens.content_padding);
    assert_eq!(style.content_indent, tokens.indent);
    assert_eq!(style.gap, tokens.gap);
    assert_eq!(style.radius, tokens.radius);
    assert_eq!(style.border_width, tokens.border);
}

/// v1 wrote `border_color(with_alpha(t.colors().outline, 30))`. Resolve the
/// outline at full alpha and the panel stops reading as a hairline and starts
/// reading as a box — a visible change nobody would catch in a diff.
#[test]
fn the_outline_is_the_theme_outline_at_30_alpha() {
    let style = kit().resolved_style();
    assert_eq!(style.border_color.a, 30, "v1's `with_alpha(outline, 30)`");

    let theme = MultiTheme::new_for(DesignSystem::Cupertino);
    assert_eq!(
        [
            style.border_color.r,
            style.border_color.g,
            style.border_color.b
        ],
        [
            theme.colors().outline.r,
            theme.colors().outline.g,
            theme.colors().outline.b
        ],
        "and the outline's own colour, not a constant"
    );
}

#[test]
fn the_engine_widget_declares_the_token_axis_and_gap() {
    let req = panels(2)
        .resolve()
        .layout_request()
        .expect("the panels are one vertical stack");
    assert_eq!(req.axis, Some(Axis::Vertical));
    assert_eq!(req.gap, Some(tokens().gap), "the token gap, not a default");
}

// --- the panels travel -----------------------------------------------------

#[test]
fn every_declared_panel_becomes_one_slot() {
    let accordion = panels(3).resolve();
    assert_eq!(accordion.panel_count(), 3);
    assert_eq!(accordion.titles(), vec!["Panel 0", "Panel 1", "Panel 2"]);
    assert_eq!(accordion.children().len(), 3);
    assert_eq!(kit().panel_count(), 0);
}

/// The single-panel rule is the design system's identity, and it has to survive
/// the trip into the engine widget: a default that silently became "multiple"
/// would let two panels sit open at once.
#[test]
fn the_single_panel_rule_survives_the_trip() {
    let mut accordion = panels(2).resolve();
    accordion.set_open(0, true);
    accordion.set_open(1, true);
    assert_eq!(
        accordion.open_state(),
        vec![false, true],
        "Material's expansion panels are an accordion by default"
    );

    let mut accordion = panels(2).multiple(true).resolve();
    accordion.set_open(0, true);
    accordion.set_open(1, true);
    assert_eq!(
        accordion.open_state(),
        vec![true, true],
        "unless the caller opts out"
    );
}

// --- the bodies ------------------------------------------------------------

/// A body is a *builder*, and 2.0's builder closure is `Send + Sync + 'static`.
///
/// That constraint is the whole reason this test exists as a compile-level
/// assertion: it does not build unless the kit shared the callback as an `Arc`
/// instead of keeping v1's `Box<dyn Fn() -> AnyView>`. v1's signature simply
/// cannot be expressed here.
#[test]
fn a_body_is_a_send_sync_builder() {
    let builds = Arc::new(AtomicUsize::new(0));
    let sink = builds.clone();
    let component = kit().panel("Only", move || {
        sink.fetch_add(1, Ordering::SeqCst);
        Box::new(floom_widgets::Text::new("body"))
    });

    assert_eq!(component.panel_count(), 1);
    assert_eq!(
        builds.load(Ordering::SeqCst),
        0,
        "declaring a panel must not build its body — a body is built when the \
         panel is opened, and again whenever the tree rebuilds that slot"
    );

    // And it *is* callable from the tree: mounting a panel slot runs no body
    // builder, which is what makes a closed panel cheap.
    let accordion = component.resolve();
    let _ = accordion.children();
    assert_eq!(
        builds.load(Ordering::SeqCst),
        0,
        "even declaring the panel's slot builds nothing"
    );
}

/// v1 wrote the body's inset as `padding_horiz(content_padding + indent)` and
/// `padding_vert(content_padding)`. 2.0's `Container` is a leaf, so the inset is a
/// margin on the body's slot — and the horizontal side has to be the *sum* or the
/// content loses the indent that makes it read as the panel's own.
#[test]
fn the_body_inset_carries_both_tokens() {
    let tokens = tokens();
    let margin = kit().content_layout().margin.expect("the body's inset");
    assert_eq!(
        margin.left,
        f64::from(tokens.content_padding + tokens.indent),
        "horizontal: padding plus indent, as v1 wrote it"
    );
    assert_eq!(
        margin.right,
        f64::from(tokens.content_padding + tokens.indent)
    );
}

// --- the indicator ---------------------------------------------------------

/// v1 swapped between two glyph *names* inside a `dyn_view` closure. 2.0 cannot
/// do that — a widget is built once — so the pair has to exist up front, and it
/// has to actually be two different outlines.
#[test]
fn the_indicator_is_a_pair_of_distinct_glyphs() {
    let indicator = kit().indicator();
    assert_ne!(
        indicator.collapsed.path(),
        indicator.expanded.path(),
        "a collapsed panel's chevron and an expanded one's are different shapes"
    );
}

#[test]
fn the_indicator_carries_the_token_size_and_the_muted_ink() {
    let indicator = kit().indicator();
    for icon in [&indicator.collapsed, &indicator.expanded] {
        assert_eq!(icon.edge(), tokens().icon_size);
        let req = icon
            .layout_request()
            .expect("an icon asks for its own size");
        assert_eq!(
            req.width.and_then(|l| l.px()),
            Some(tokens().icon_size),
            "the icon is sized in pixels, not as a percentage"
        );
        assert_eq!(
            req.height.and_then(|l| l.px()),
            Some(tokens().icon_size),
            "the icon is sized in pixels, not as a percentage"
        );
    }

    let theme = MultiTheme::new_for(DesignSystem::Cupertino);
    assert_eq!(
        indicator.expanded.fill().r,
        theme.colors().on_surface_variant.r,
        "v1 coloured the chevron `on_surface_variant`"
    );
    assert_eq!(
        indicator.expanded.fill(),
        indicator.collapsed.fill(),
        "and both states share the ink"
    );
}

/// The glyph swap is why the engine takes a *builder* rather than a widget: it
/// cannot know the state when the panel is declared. This is that seam, written
/// the way `resolve` writes it.
#[test]
fn the_kit_hands_the_engine_a_builder_that_is_told_the_state() {
    let indicator = kit().indicator();
    let (collapsed, expanded) = (indicator.collapsed, indicator.expanded);
    let choose = move |open: bool| -> Box<dyn floom_core::WidgetAny> {
        Box::new(if open {
            expanded.clone()
        } else {
            collapsed.clone()
        })
    };
    assert!(choose(false).as_any().is::<VectorIcon>());
    assert!(choose(true).as_any().is::<VectorIcon>());
    assert_ne!(
        choose(true)
            .as_any()
            .downcast_ref::<VectorIcon>()
            .expect("an icon")
            .path(),
        choose(false)
            .as_any()
            .downcast_ref::<VectorIcon>()
            .expect("an icon")
            .path(),
        "and the two states resolve to different glyphs"
    );
}

// --- the shared props shape ------------------------------------------------

/// `ExpansionPanelsProps` is the design-system-neutral carrier, and 2.0 cannot
/// use its `Box<dyn Fn(..)>` field — the kit holds an `Arc` of its own and says
/// so rather than reporting a callback it does not keep there.
#[test]
fn the_shared_props_report_the_multiple_rule_only() {
    let props: ExpansionPanelsProps = panels(1).multiple(true).props();
    assert!(props.multiple);
    assert!(
        props.on_change.is_none(),
        "the callback is held as an `Arc` of the kit's own, not in the v1 carrier"
    );
}

/// `on_change` has to be `Send + Sync` to reach a click that lands inside a
/// `Child` builder closure. Holding it as an `Arc` and cloning it into the
/// accordion is what makes this compile, so the test that exists is the one that
/// runs that construction.
#[test]
fn the_change_callback_survives_being_shared_into_the_tree() {
    let seen: Arc<std::sync::Mutex<Vec<(usize, bool)>>> =
        Arc::new(std::sync::Mutex::new(Vec::new()));
    let sink = seen.clone();
    let mut accordion = panels(2)
        .on_change(move |index, open| sink.lock().unwrap().push((index, open)))
        .resolve();

    // The imperative door does not report a person's click — the callback is for
    // the user, and a caller that just set the state knows already.
    accordion.set_open(1, true);
    assert!(seen.lock().unwrap().is_empty());
    assert!(
        accordion.is_open(1),
        "but the state did move, so the callback has something to report when a \
         click does happen"
    );
}

/// The kit resolves Cupertino's palette, not another system's.
///
/// The expansion-panel *tokens* are the same in all three design systems, so
/// the only thing that distinguishes the kits is the colour. A test that
/// compared the kit against whatever theme the thread happened to hold would
/// pass for any of the three, so the palette is pinned here.
#[test]
#[test]
fn the_colours_are_this_design_system_s_own() {
    let theme = theme();
    let style = panels(1).resolved_style();
    assert_eq!(
        style.title_color.r,
        theme.colors().on_surface.r,
        "the title is drawn in this design system's own ink"
    );
    assert_eq!(
        style.border_color.r,
        theme.colors().outline.r,
        "and the separator in its own outline"
    );
    // The control: the theme really is Cupertino's, so the two assertions above
    // are about this design system's palette and not about Material's.
    assert_eq!(
        theme.colors().primary,
        Color::rgb(0, 122, 255),
        "Cupertino's own primary"
    );
}

/// The header's own inset: v1 wrote `padding_horiz(content_padding)` on the
/// header's box.
///
/// The engine's `Accordion` carries the number in its style -- and used to
/// leave it unspent, so the title sat flush against the panel's edge while the
/// style said otherwise.
#[test]
fn the_header_is_padded_at_the_sides() {
    let style = CupertinoExpansionPanels::new().resolved_style();
    let accordion = panels(1).resolve();
    // The panel holds the header, the header holds a pressable, and the
    // pressable holds the header's row.
    let leaf = accordion.children()[0].build();
    let pressable = leaf.children().remove(0).build();
    let row = pressable.children().remove(0).build();
    let request = row
        .layout_request()
        .expect("the header's row asks for its own space");
    assert_eq!(
        request.padding,
        Some(Insets {
            left: f64::from(style.header_padding),
            right: f64::from(style.header_padding),
            top: 0.0,
            bottom: 0.0,
        }),
        "v1: `padding_horiz(content_padding)` on the header"
    );
}

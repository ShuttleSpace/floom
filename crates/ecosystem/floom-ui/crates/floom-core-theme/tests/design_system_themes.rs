use floom_core_theme::component_tokens::{ComponentId, ComponentTokens};
use floom_core_theme::global_theme::{DesignSystem, design_system_signal};
use floom_core_theme::theme::{CupertinoTheme, FluentTheme, MultiTheme, Theme};
use floom_core_theme::tokens::color::Color;
use floom_legacy_reactive::SignalUpdate;

#[test]
fn cupertino_theme_implements_theme_contract() {
    let theme = CupertinoTheme::new();

    assert_eq!(theme.colors().primary, Color::rgb(0, 122, 255));
    assert_eq!(theme.colors().surface, Color::rgb(255, 255, 255));
    assert_eq!(theme.typography().body.medium.font_size, 17.0);
    assert_eq!(theme.radius().md, 10.0);

    let ComponentTokens::Button(button) = theme.component(ComponentId::Button) else {
        panic!("button tokens expected")
    };
    assert_eq!(button.height, 44.0);
}

#[test]
fn fluent_theme_implements_theme_contract() {
    let theme = FluentTheme::new();

    assert_eq!(theme.colors().primary, Color::rgb(0, 120, 212));
    assert_eq!(theme.colors().surface, Color::rgb(255, 255, 255));
    assert_eq!(theme.typography().body.medium.font_size, 14.0);
    assert_eq!(theme.radius().md, 4.0);

    let ComponentTokens::Button(button) = theme.component(ComponentId::Button) else {
        panic!("button tokens expected")
    };
    assert_eq!(button.height, 32.0);
}

// --- what `MultiTheme` actually reads -------------------------------------
//
// The two tests above get real per-kit numbers by reading `CupertinoTheme` and
// `FluentTheme` *directly*. Every component in floem-ui does not: they hold a
// `RwSignal<MultiTheme>`, and this section is what that costs.
//
// Measured while porting the three counters, and it is sharper than "the `ds`
// argument is ignored": **`MultiTheme` does not read its own stored active
// variant either.** Every accessor dispatches on the process-global design
// system — `global_theme::set_design_system`'s own doc comment says as much
// ("the `MultiTheme` itself reads the active DS live"), which is the sentence
// that should have stopped this port from assuming otherwise.
//
// The tests below set the global through `design_system_signal()` rather than
// through `set_design_system`, because the latter is behind the very
// `#[cfg(any(feature = "material", ...))]` gate this crate's own test run does
// not enable — and a test that needs a design-system feature to observe a
// design-system bug is a test that will not run. So `new_for(Cupertino)` under a Material global reads Material's
// colours, and handing a component a `MultiTheme` through `with_theme` changes
// nothing at all.
//
// That has one sharp consequence for the port, and it is why the counter suites
// cannot do what most token-adapter suites would do: **there is no way to give a
// ported component a palette other than the process-global one.** A test that
// wants two different palettes in one process has to move the global between the
// two reads, and it has to do it in that order, because the global is the only
// lever there is.

/// The palette follows the global, and ignores the design system the theme was
/// built for.
#[test]
fn multi_theme_reads_follow_the_process_global_not_its_own_argument() {
    design_system_signal().set(DesignSystem::Material);
    let built_as_cupertino = MultiTheme::new_for(DesignSystem::Cupertino);

    design_system_signal().set(DesignSystem::Material);
    let material = built_as_cupertino.colors().surface_variant;

    design_system_signal().set(DesignSystem::Cupertino);
    let cupertino = built_as_cupertino.colors().surface_variant;

    assert_eq!(
        material,
        Color::rgb(231, 224, 236),
        "the same MultiTheme value, built as Cupertino, reads Material while the \
         global says Material"
    );
    assert_eq!(
        cupertino,
        Color::rgb(242, 242, 247),
        "and reads Cupertino the moment the global says Cupertino -- so it is not \
         frozen at construction either"
    );
}

/// Component tokens move the same way, which is the half a counter cares about.
#[test]
fn component_tokens_follow_the_process_global_too() {
    let gap_under = |ds: DesignSystem| {
        design_system_signal().set(ds);
        // `Button`, because it is a component whose tokens genuinely differ
        // between kits -- unlike `Counter`, whose six numbers come from one
        // shared table and so cannot move at all.
        match MultiTheme::new_for(ds).component(ComponentId::Button) {
            ComponentTokens::Button(b) => b.height,
            _ => unreachable!(),
        }
    };
    assert_eq!(gap_under(DesignSystem::Material), 40.0);
    assert_eq!(gap_under(DesignSystem::Cupertino), 44.0);
    assert_eq!(gap_under(DesignSystem::Fluent), 32.0);
}

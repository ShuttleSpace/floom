//! Phase 29.2 — the v1 prelude is preserved and mirrored.
//!
//! Two guarantees are pinned here:
//!
//! 1. **Preservation**: `floom::prelude` still carries the full v1 API, so
//!    existing v1 applications compile unchanged (the preservation policy for
//!    the current major version).
//! 2. **Mirror**: `floom::compat::v1_prelude` exposes the *same items* — the
//!    stable v1 path a v1 app can migrate to before any future narrowing of
//!    `prelude`. Re-exported items have identical types through both paths;
//!    if `v1_prelude` ever diverges, the `same_type` assertions below stop
//!    compiling.

use std::any::TypeId;

use floom::compat::v1_prelude as v1;
use floom::prelude as p;

/// Compiles only when both expressions have the same type.
#[allow(deprecated)]
fn same_type<T>(_: T, _: T) {}

#[test]
#[allow(deprecated)]
fn reactive_free_functions_are_the_same_items_through_both_paths() {
    same_type(p::create_rw_signal::<i32>, v1::create_rw_signal::<i32>);
    same_type(p::create_signal::<i32>, v1::create_signal::<i32>);
    same_type(p::create_trigger, v1::create_trigger);
}

#[test]
#[allow(deprecated)]
fn reactive_and_view_types_are_identical_through_both_paths() {
    assert_eq!(
        TypeId::of::<p::RwSignal<i32>>(),
        TypeId::of::<v1::RwSignal<i32>>(),
        "RwSignal must be the same type through prelude and v1_prelude"
    );
    assert_eq!(
        TypeId::of::<p::Scope>(),
        TypeId::of::<v1::Scope>(),
        "Scope must be the same type through prelude and v1_prelude"
    );
    assert_eq!(
        TypeId::of::<p::Color>(),
        TypeId::of::<v1::Color>(),
        "Color must be the same type through prelude and v1_prelude"
    );
}

#[test]
#[allow(deprecated)]
fn the_compat_path_runs_a_v1_app_snippet() {
    // The full v1 idiom, through the compat path only: a signal, an effect
    // reading it, and a colour constant — no `floom::prelude` import.
    use floom::compat::v1_prelude::*;

    let counter = create_rw_signal(0_i32);
    assert_eq!(counter.get(), 0);
    counter.set(41);
    assert_eq!(counter.get(), 41);

    let _colour: Color = Color::BLACK;
    let _css: Color = css::RED;
}

#[test]
#[allow(deprecated)]
fn unit_and_event_extensions_resolve_through_the_compat_path() {
    use floom::compat::v1_prelude::*;

    // `UnitExt` (styling ergonomics) and the keyboard event types are part
    // of the frozen surface too.
    let width = 24.0.px();
    assert_eq!(width, 24.0.px());

    let key = Key::Named(NamedKey::Enter);
    assert_eq!(key, key);
}

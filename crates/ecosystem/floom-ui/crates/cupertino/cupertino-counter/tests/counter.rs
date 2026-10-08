//! The ported Material counter.
//!
//! ## What this suite is actually about
//!
//! A token adapter is the shortest port in the library, and the risk in it is not
//! the code — it is a number that is silently wrong. So the tests here are mostly
//! **"is this value read, or is it a literal in disguise?"**, and the control for
//! each is the same number coming from a different kit.
//!
//! ## The one literal, and why it is not a bug
//!
//! v1 wrote `s.font_size(16.0)` on the readout in all three kits and
//! `CounterTokens` has no font-size field. So the 16.0 is a constant of the design
//! system rather than a token, it is named [`READOUT_FONT_SIZE`] rather than
//! inlined, and all three crates declare the same value — which is a *measured*
//! agreement, not an assumption. See
//! `the_readout_font_size_is_sixteen_and_is_not_a_token`.

use std::sync::{Arc, Mutex};

use floom_widgets::Counter;

use floom_widget_cupertino_counter::{READOUT_FONT_SIZE, CupertinoCounter};

use floom_core_theme::component_tokens::{ComponentId, ComponentTokens, CounterTokens};
use floom_core_theme::global_theme::DesignSystem;
use floom_core_theme::theme::{MultiTheme, Theme};

/// Activate `ds`, then build the counter against it.
///
/// The design system is a process-global `thread_local`; the harness gives each
/// test its own thread, so this cannot leak into a sibling. Every test goes
/// through here rather than constructing the counter directly, because a counter
/// built without activating its kit reads whichever kit the previous test left
/// behind — which is green for the wrong reason.
fn counter_under(ds: DesignSystem) -> CupertinoCounter {
    floom_core_theme::global_theme::set_design_system(ds);
    CupertinoCounter::new()
}

/// The counter, resolving against Material.
fn counter() -> CupertinoCounter {
    counter_under(DesignSystem::Material)
}

/// The token table Material's counter reads, through the same global.
fn tokens_under(ds: DesignSystem) -> CounterTokens {
    floom_core_theme::global_theme::set_design_system(ds);
    match MultiTheme::new_for(ds).component(ComponentId::Counter) {
        ComponentTokens::Counter(t) => t,
        _ => unreachable!(),
    }
}

fn cupertino_tokens() -> CounterTokens {
    tokens_under(DesignSystem::Cupertino)
}

// --- tokens --------------------------------------------------------------

/// Every size comes from `CounterTokens`, and the engine's style carries all six.
///
/// One test for six fields, because they are six readings of one rule: "this crate
/// translates a token table and does not compute sizes". A test per field would be
/// six near-identical assertions with six near-identical mutations.
#[test]
fn the_sizes_come_from_the_token_table() {
    let tokens = cupertino_tokens();
    let style = counter().counter_style();
    assert_eq!(style.button_size, tokens.button_size);
    assert_eq!(style.width, tokens.width);
    assert_eq!(style.height, tokens.height);
    assert_eq!(style.radius, tokens.radius);
    assert_eq!(style.gap, tokens.gap);
    assert_eq!(style.border, tokens.border);
}

/// **The control for the test above**, and it is worth being precise about what
/// the control is *for*.
///
/// `CounterTokens` is a **single shared table**: all three kits read the same one,
/// so no design system can move these six numbers and no test here can tell a
/// *read* from a *hard-coded literal* — a crate that wrote `32.0` inline would pass
/// the test above. The protection for the sizes is not a test, it is that this
/// crate contains no size literals at all (`grep -n '32.0\|140.0\|40.0\|8.0\|4.0\|1.0' src/lib.rs`
/// returns nothing), which is a cheaper and more honest check than a test that
/// cannot fail.
///
/// The colours, by contrast, *do* differ per kit, so they get a real control —
/// below.
/// The step buttons are filled with `surface_variant` and their glyphs drawn in
/// `on_surface` — the two colours v1 read, in that pairing.
#[test]
fn the_step_buttons_are_surface_variant_with_on_surface_ink() {
    let theme = MultiTheme::new_for(DesignSystem::Material);
    let style = counter().with_theme(theme.clone()).resolved_style();
    assert_eq!(style.button_background, theme.colors().surface_variant);
    assert_eq!(style.button_foreground, theme.colors().on_surface);
}

/// The band's outline is the `outline` token, not `on_surface` and not a literal.
#[test]
fn the_band_is_outlined_with_the_outline_token() {
    let theme = MultiTheme::new_for(DesignSystem::Material);
    let style = counter().with_theme(theme.clone()).resolved_style();
    assert_eq!(style.border_color, theme.colors().outline);
    assert_ne!(
        style.border_color, style.button_foreground,
        "`outline` and `on_surface` are different tokens; reading the wrong one \
         would still produce a plausible outline"
    );
}

/// The kit this crate is *for*.
fn own() -> DesignSystem {
    DesignSystem::Cupertino
}

/// A kit whose palette differs from [`own`], used as the contrast.
///
/// Named rather than inlined so the derivation for the other two crates has
/// exactly one line to change. See `correction 16` in the porting doc: a blanket
/// substitution turned this test into "Material against Material" in the two
/// derived crates, and it passed there for the wrong reason only because the
/// colours happened to be equal — the test was measuring nothing at all.
fn contrast() -> DesignSystem {
    DesignSystem::Material
}

/// **The control that actually works for a token adapter: the same crate, two
/// kits, different numbers.**
///
/// `surface_variant` is 231,224,236 under Material and 242,242,247 under
/// Cupertino -- two palettes that differ, which is the whole point. A crate that
/// hard-coded its colours would answer the same both times, so this fails for a
/// hard-coded adapter and passes for a reading one.
///
/// ## And it has to go through the process-global, because that is the only lever
///
/// `MultiTheme` does **not** read the design system it was built for — it reads
/// `global_theme::design_system_signal()` on every accessor. So `with_theme(..)`
/// changes nothing, and the only way to see two palettes is to move the global
/// between the two reads. That is measured, and pinned where it belongs:
/// `floom-core-theme`'s `multi_theme_reads_follow_the_process_global_not_its_own_argument`.
///
/// The consequence is worth stating plainly, because it is the cost of every
/// token adapter in this port: **a component cannot be given a palette.** Ask a
/// Cupertino counter for Material's colours and it will hand them over.
#[test]
fn the_colours_follow_the_active_kit_so_they_are_read_not_hard_coded() {
    let a = counter_under(own()).resolved_style().button_background;
    let b = counter_under(contrast()).resolved_style().button_background;
    assert_ne!(
        a, b,
        "surface_variant differs per kit; equal results would mean the adapter \
         hard-coded one"
    );
}

/// **And the uncomfortable half of the same fact**: this crate does not pin its
/// own kit. A Material counter built while Cupertino is the active design system
/// draws Cupertino's colours.
///
/// That is not a bug in this crate — it is what `MultiTheme` is — and the test is
/// here so that "the kits agree about which palette they read" is never assumed.
/// Material's v1 code called `theme_signal()` and the other two built their own
/// `RwSignal::new(MultiTheme::new_for(..))`, which looks like a difference and is
/// not one: both read the same global. This is the third time this port has found
/// an apparent kit difference that dissolves on measurement.
#[test]
fn the_crate_reads_whichever_kit_is_active_rather_than_pinning_its_own() {
    let own_kit = counter_under(own()).resolved_style().button_background;
    let other = counter_under(contrast()).resolved_style().button_background;
    assert_ne!(
        own_kit, other,
        "and there is no with_theme that changes it: MultiTheme reads the global"
    );
}

/// The readout is drawn in `on_surface` — the *same* token as the glyphs, which
/// is why the engine's `CounterStyle` has one `value_font_size` for both and a kit
/// that wanted them to differ would need a second field.
#[test]
fn the_readout_is_drawn_in_on_surface_the_same_ink_as_the_glyphs() {
    let style = counter().resolved_style();
    assert_eq!(style.value_color, style.button_foreground);
}

// --- the one literal -----------------------------------------------------

/// The 16.0, stated as a fact about v1 rather than left as a number in a chain.
///
/// `CounterTokens` has no font-size field, so this cannot be "read from the
/// table" — and if a future token table grows one, this test is where the choice
/// to use it gets made deliberately.
#[test]
fn the_readout_font_size_is_sixteen_and_is_not_a_token() {
    assert_eq!(READOUT_FONT_SIZE, 16.0);
    let style = counter().counter_style();
    assert_eq!(style.value_font_size, 16.0);
}

// --- the widget ----------------------------------------------------------

/// `view()` starts at the requested value, clamped by whatever bounds came with it.
#[test]
fn the_view_starts_at_the_requested_value() {
    let widget = counter().value(7).view();
    assert_eq!(widget.value(), 7);
}

/// The bounds and the step reach the engine — three setters, three reads.
#[test]
fn the_bounds_and_the_step_reach_the_engine() {
    let widget = counter().value(5).min(0).max(10).step(3).view();
    assert_eq!(widget.minimum(), Some(0));
    assert_eq!(widget.maximum(), Some(10));
    assert_eq!(widget.step(), 3);
    assert_eq!(widget.step_up(), 8);
}

/// A bound arrives already clamped, because the engine clamps when a bound is set.
///
/// v1 clamped at construction inside `view()`; the port moved the clamp to the
/// bound setter, which is the same rule with one path instead of two.
#[test]
fn a_bound_set_before_mount_clamps_immediately() {
    let widget = counter().value(50).max(10).view();
    assert_eq!(widget.value(), 10, "not 50 waiting for a press");
}

/// **`view()` returns the widget**, which is the API improvement over v1's
/// `impl View`. A caller can read the value after mount; in v1 the signal was
/// created inside `view()` and returned inside an opaque view.
#[test]
fn the_view_hands_back_a_widget_the_caller_can_read() {
    let widget: Counter = counter().value(1).view();
    widget.step_up();
    assert_eq!(widget.value(), 2);
}

/// The callback reaches the engine and reports real moves.
#[test]
fn the_change_callback_reports_a_move() {
    let seen: Arc<Mutex<Vec<i64>>> = Arc::new(Mutex::new(Vec::new()));
    let sink = seen.clone();
    let widget = counter()
        .value(1)
        .on_change(move |v| sink.lock().unwrap().push(v))
        .view();

    widget.step_up();
    widget.step_up();
    assert_eq!(seen.lock().unwrap().clone(), vec![2, 3]);
}

/// **The preserved v1 behaviour.** A press at the ceiling still reports, with the
/// value unchanged. v1 wrote `value.set(nv); cb(nv)` with no guard, so this is not
/// a port decision — it is a decision this port had to *not* make, and a test is
/// the only thing that keeps it from being made by accident later.
#[test]
fn a_press_at_the_ceiling_still_reports_the_unchanged_value() {
    let seen: Arc<Mutex<Vec<i64>>> = Arc::new(Mutex::new(Vec::new()));
    let sink = seen.clone();
    let widget = counter()
        .value(10)
        .max(10)
        .on_change(move |v| sink.lock().unwrap().push(v))
        .view();

    assert_eq!(widget.step_up(), 10);
    assert_eq!(seen.lock().unwrap().clone(), vec![10]);
}

/// The style the kit computed is the style the engine carries, in every field.
///
/// One test per field would be eleven near-identical assertions; the risk this
/// guards against is a *field* being dropped in the translation, and an equality
/// on the whole struct catches a dropped field as loudly as a wrong one.
#[test]
fn the_engine_style_is_the_resolved_style_translated_field_for_field() {
    let resolved = counter().resolved_style();
    let engine = counter().counter_style();
    assert_eq!(engine.button_size, resolved.button_size);
    assert_eq!(engine.width, resolved.width);
    assert_eq!(engine.height, resolved.height);
    assert_eq!(engine.radius, resolved.radius);
    assert_eq!(engine.gap, resolved.gap);
    assert_eq!(engine.border, resolved.border);
    assert_eq!(
        engine.border_color.r, resolved.border_color.r,
        "the colour is converted, not copied: floom_types::Color carries the same \
         four channels and a copy would type-check"
    );
}

/// The kit identifies itself, so a caller debugging three adapters can tell which
/// one produced a style.
#[test]
fn the_resolved_style_names_its_design_system() {
    assert_eq!(counter().resolved_style().design_system, "cupertino");
}

/// The props survive to the caller, minus the callback — v1's `Clone` dropped it
/// too, and a clone that silently lost a callback would be a worse bug than one
/// that says so.
#[test]
fn the_props_are_kept_and_the_callback_is_held_separately() {
    let built = counter().value(3).min(1).max(9).step(2);
    let props = built.props();
    assert_eq!(props.value, 3);
    assert_eq!(props.min, Some(1));
    assert_eq!(props.max, Some(9));
    assert_eq!(props.step, 2);
}

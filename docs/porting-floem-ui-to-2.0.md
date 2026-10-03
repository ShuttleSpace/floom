# Porting `floem-ui` from the v1 API to the Floom 2.0 API

`floem-ui` is the design-system component library: **298 crates, 66,729 LOC**
(98 distinct components × 3 kits — Material, Cupertino, Fluent — plus the shared
base crates). It is written against the **v1** API throughout. Because the plan
is to delete the v1 monolith, this library must be ported to the 2.0 widget
model first.

This document is the porting plan, with the numbers measured from the tree.

## Why this is not a rename

v1 and 2.0 are different widget models:

| | v1 (`floom`) | 2.0 (`floom` after the rename) |
|---|---|---|
| Widget | `View` trait, `IntoView`, `dyn_view` | `WidgetAny`, concrete widget structs |
| Composition | `(a, b).into_view()`, decorators | `Child::new(key, request, \|\| Box::new(..))` |
| Host | implicit global runtime | explicit `WidgetHost` owns the tree |
| Update | reactive `RwSignal` read during render | mutate the node, then `host.frame()` |

So components are rebuilt, not edited. The reusable part is the **design logic**
— tokens, spacing, state machines, variants — which is why this is tractable but
still large.

## The gap that gates everything — corrected

My first estimate here was wrong, and it mattered, so this section records the
correction rather than quietly replacing it.

The original claim was "2.0 ships 39 widget types covering only 19 of floem-ui's
98 components, so **79 widgets must be built** before the kits can be ported".
That conflated a *component* with a *widget type*. Measured against the tree:

- **`impl View for` appears zero times across all three kits.** floem-ui contains
  no custom widgets at all.
- Every one of the 79 components is a **declarative composition** of four v1
  built-ins — `Container::new` (165 uses), `Label::new` (95), `Stack::new` (11),
  `Button::new` (1).
- Even the components that look like they need new machinery are compositions:
  `md-calendar` is `Label::new("‹")` plus nested containers; `md-datatable` is
  `Label::new(..)` inside `Container::new(..)`.

So **Stage B requires zero new widget types to port floom-ui.** 2.0's
`Container` / `Text` / `Stack` plus `Child::new(ChildKey, LayoutRequest, ..)`
composition can express everything floem-ui expresses today.

What is left is therefore *volume and translation*, not architecture:

- **The real translation cost** is the v1 style closure
  (`.style(|s| s.padding(..).background(..).rounded(..))`) becoming 2.0's
  explicit builder calls (`Container::new().background(..).radius(..)`).
  Mechanical, per component, but it is the bulk of the work.
- **The three kits are not duplicates** — material/cupertino/fluent differ by
  ~60% of their lines (e.g. `card` differs on 71–75 of 115 lines), so this is
  genuinely 3× work despite a shared shape.
- **The one genuinely new capability is interaction animation**, needed for
  `Ripple` (see below). Everything else is composition.

Scale: 98 components × 3 kits ≈ 50,000 LOC of composition to rewrite.

Already covered by an existing 2.0 widget (19): `button`, `checkbox`,
`container`, `dialog`, `divider`, `grid`, `icon`, `image`, `list`,
`navigation_drawer`, `overlay`, `radio`, `select`, `slider`, `spacer`, `switch`,
`tabs`, `text`, `text-field`.

**Started:** `badge` (`crates/floom-widgets/src/badge.rs`, 10 tests). Note this
one genuinely *was* worth a widget type — it is small, self-contained, and sets
the pattern; the remaining 78 do not each need one.

**Started:** `badge` (done — `crates/floom-widgets/src/badge.rs`, 10 tests).

### The pattern a Stage B widget follows

`Badge` is the reference implementation for the other 78, because it fixes the
seam between 2.0 and floom-ui:

- **A 2.0 widget is theme-agnostic.** It takes explicit values
  (`background`, `foreground`, `font_size`, `min_size`, `padding`,
  `dot_size`, `radius`). Resolving "which design system, which palette" is the
  *kit's* job — so the v1 pattern of a widget reading a global
  `theme_signal()` does not carry over, and the kits get thinner as they port.
- **Impl `floom_core::Widget`** with the defaults doing the work: `create` is
  `cx.alloc_child()`, `update` is `self.revision.sync(cx, id)`, and a leaf only
  adds `layout_request` (intrinsic size), `paint`, and `semantics`.
- **Paint respects layout.** `paint` fills `cx.bounds()`, not its own intrinsic
  size — `dot_size` and friends only feed `layout_request`, and the host wins
  if a parent stretches the box.
- **Semantics go through the crate's vocabulary.** `floom-widgets` has its own
  `primitives::SemanticsHint` (naming `floom-accessibility` types) and
  `widget_any.rs` bridges it into the engine-neutral `floom_core::SemanticsHint`.
  Register the widget in `impl_widget_any!` rather than hand-writing
  `WidgetAny`.
- **Geometry types are `f64`.** `Rect::width()/height()`, `Point::new` and
  `LayoutRequest.width/height` are `f64` / `Option<f32>` respectively — the
  mismatch is the most common first-compile error.

Two behaviours worth keeping, both covered by tests: a pill's radius defaults to
half its height (so it reads as a stadium, not a rectangle), and an unchanged
widget must mark **nothing** dirty on re-sync, or every frame would invalidate
the whole subtree.

None of the 79 is missing a *widget type* — see the correction above. The list
is retained only as the porting backlog, ordered so the pieces other components
depend on land first.

### Stage B — the kits

Shake the widget API out with one component per *shape* before translating in
bulk — a bulk port on top of an unexercised widget API just relocates the
mistakes.

| Shape | Example | Status |
|---|---|---|
| surface | `md-card` | ported, 7 tests |
| interactive | `md-button` | ported, 10 tests |
| stateful | (`carousel`, `expansion_panels`, …) | **next** |

What porting `md-button` turned up, all of it a real gap rather than a
translation nuisance:

* **`Ripple` had no mutators.** v1 passed `radius_fn` and `color` closures that
  re-read the theme on every use. 2.0 mounts once, so a kit must push new values
  in — `set_radius` / `set_ink` / `set_disabled` were added for exactly this, and
  they are idempotent so a theme signal firing on unrelated changes does not
  repaint every frame.
* **A click is a press *and* a release.** `Button` activates only on a matched
  pair, so a test that sends one half is not testing a weaker click, it is not
  testing a click.
* **2.0's `Child` builder closure must be `Send + Sync + 'static`.** A callback
  therefore cannot be `Rc<dyn Fn()>`; it has to be `Arc<dyn Fn() + Send + Sync>`,
  which is a visible API constraint on `MdButton::on_click` and is documented
  there. `Button` is correspondingly not `Clone`, so the tree rebuilds a fresh
  one per call from captured plain data.
* **Geometry types disagree.** `Ripple::radius` takes `f64` while button tokens
  are `f32` — the source of the kit's `f32`/`f64` mismatch.

Then:

1. **Batch the surface components** (`footer`, `toolbar`, `item_group`) — these
   should now be uneventful.
2. **Interaction animation** for `Ripple` — the only new capability needed.
3. **Port one kit end to end** (material, 98 components), then cupertino and
   fluent, which follow the same shape with their own tokens.
4. **`floom-elements`** last: its 800 lines of re-exports and 6,220 lines of v1
   `multi` dispatch can only be rewritten once the underlying components exist.

## Stages

Dependency-ordered. Each stage is independently verifiable by compiling.

### Stage A — shared base crates (16,377 LOC)

The four crates every kit depends on. Must land first: the kits cannot be
meaningful until the base exists on 2.0.

| Crate | LOC | v1 coupling | Status |
|---|---|---|---|
| `floom-core-theme` | 7,711 | trivial — tokens only | **ported** |
| `floom-testing` | 22 | none (zero dependencies) | **no port needed** |
| `floom-widget-runtime` | 351 | theme plumbing ported; `Ripple` not | **ported, minus `Ripple`** |
| `floom-elements` | 8,293 | heavy — `View`, `IntoView`, reactive reads | todo |

`floom-core-theme` turned out to need almost nothing from v1: theme state is
just `RwSignal` + `create_global_rw_signal` (which live in `floom-legacy-reactive`
and are already re-exported by the 2.0 prelude), plus two value conversions.
It now depends on `floom-legacy-reactive`, `peniko` and `parley` directly — no
widget layer, no `floom`.

`floom-widget-runtime` is the crate 99 others depend on, and **91 of them use
only `theme_signal`**, so porting it was high leverage. Its theme plumbing is now
2.0-native. Two of its helpers changed shape because 2.0 has no wrapper widget:
`theme_provider` and `layout_provider` used to take a child and return
`impl View`; 2.0's `Container` is a leaf (paint, background, padding, filters)
with no `.child()` builder, because composition moved to the parent via
`Child::new(ChildKey, LayoutRequest, || Box::new(..))`. So both now only install
their signal, and wrapping is the caller's job.

#### `Ripple` — no engine gap at all

`Ripple` (the Material press ripple) was described here twice as blocked by an
engine limitation. **It is not.** `crates/floom-app/tests/event_animation.rs`
proves the two-phase pointer-driven case works today, and I twice misread the
code before checking; recording the proof so it does not get re-litigated.

A two-phase ripple is: a press starts an *expand*, a release starts a *fade*.
All three links work:

1. **A widget can change its declared animation.** `WidgetHost::sync_declared_animation`
   compares the newly declared `AnimationSpec` with the previous one via
   `declared.matches(&spec)` and, on a difference, cancels and restarts.
2. **One animated scalar is enough.** `AnimationSpec::new` is a tween that
   drives nothing by itself, which means "the widget reads the value" via
   `cx.animation_value()` — `Spinner` already does. The phases are *sequential*,
   so one scalar at a time covers radius-then-alpha.
3. **A state change made during `event()` does reach the reconciler.** This is
   the link I misread twice. `EventContext` has no `mark_dirty`, and the routing
   loop never marks the routed node — but that does not matter, because the
   update pass visits **every** node every frame and dirtiness is *reported by*
   `update()` (`Revision::sync` compares against the last synced revision) rather
   than gating which nodes run. So a widget that bumps its own revision in
   `event()` is reported dirty on the next frame, joins the reconcile set, and
   its new spec restarts the animation.

**The one requirement: bump the revision when the phase changes.** Without it the
spec change is never noticed. The test asserts this negatively — deleting the
`self.revision += 1` line fails `the_new_phase_starts_a_running_animation` and
nothing else, which is what makes it a real control rather than a passing
vacuum.

**Ported.** `floom-widgets/src/ripple.rs` (11 tests) and re-exported from
`floom-widget-runtime`, which is where the 99 kit crates reach it. Two things
got simpler in the port: 2.0 hands `paint` and `event` `cx.bounds()` directly,
so v1's `LayoutChangedListener` (which existed only to learn its own box before
it could size the ink) is gone; and clipping is a declaration
(`Widget::clip` -> `ClipShape::RoundedBox`) rather than a style closure.

The ripple **never consumes the event** — wrapping a button in one must not
disable the button — and a release with no press in flight starts nothing, or a
stray release would conjure ink from nowhere.

Remaining callers still on the v1 call shape (`Ripple::new(x).view()`): the kit
crates, which is ordinary port work — `Ripple` *is* the widget now, so the
`.view()` terminal call goes away.

### Stage B — superseded

The original plan here was to build 79 widgets in `floom-widgets`. Measurement showed
that is unnecessary (see the correction above); the ordered widget list that
followed has been removed in favour of the kit-porting order.

1. *Composites over existing widgets* — `card`, `footer`, `item_group`,
   `list_group`, `list_item`, `list_subheader`, `toolbar`, `app`, `main`,
   `counter`, `empty_state`, `checkbox_btn`, `icon_button`, `button_group`.
2. *Feedback / decoration* — `alert`, `badge`, `banner`, `chip`, `chip_group`,
   `code`, `kbd`, `tooltip`, `spinner`-adjacent `skeleton_loader`,
   `progress_circular`, `progress_linear`, `stepper`, `rating`, `sparkline`,
   `speed_dial`.
3. *Navigation & layout* — `bottom_navigation`, `breadcrumbs`, `pagination`,
   `parallax`, `responsive`, `timeline`, `navigation_*` remainder,
   `infinite_scroller`, `virtual_scroller`, `pull_to_refresh`, `expansion_panels`.
4. *Input* — `autocomplete`, `combobox`, `number_input`, `otp_input`,
   `range_slider`, `textarea`, `file_input`, `color_picker`, `date_picker`,
   `time_picker`, `calendar`, `validation_messages`.
5. *Data-heavy* — `table`, `datatable`, `data_iterator`, `treeview`,
   `display_provider`, `menu`, `sheet`, `floating_action_button`, `system_bar`,
   `hover`, `confirm_edit`, `*_provider` traits (`defaults`, `locale`,
   `theme`, `icons`, `date_adapter`).

The `*_provider` traits first: they are interfaces the others plug into, so
doing them early avoids rework.

### Stage C — port the 294 kit crates

Once the base and widgets exist, the kits are mostly *theme + composition*:
tokens already ported in Stage A, so a component is a widget wired to
`floom-core-theme` tokens. Expect this stage to be repetitive and to shrink as
automation helps. The three kits stay structurally identical — one kit ported
gives the pattern for the other two.

### Stage D — delete v1 and rename

Only after `floom-ui` no longer references it:

1. Delete `crates/floom` (70,196 LOC), `floom-legacy-renderer` + 4 backends
   (6,469), `floom-legacy-test`, and the 33 v1 examples.
2. Rename `floom2` → `floom`.
3. **Keep `floom-legacy-reactive`** — 2.0 depends on it
   (`floom-runtime`, `floom-runtime-core`, `floom-store`, `floom-task`,
   `floom-widgets`, `floom2`), so it is not part of the v1 teardown despite the
   name. Renaming it to drop "legacy" is a separate, later cleanup.

## Note on `floem-editor`

`ecosystem/floom-editor` (20,593 LOC) is also v1-based and is **not** covered by
this plan. It needs its own porting assessment once the widget set is in place.

## Rules for the port

- One crate at a time, each compiling before moving on.
- Never reintroduce a `floom` (v1) dependency into a ported crate — the point is
  to remove the edge.
- Port design logic first, markup second: the tokens and state machines are the
  valuable part and are the part that survives.
- Prefer promoting a component that appears in all three kits into
  `floom-widgets` over porting it three times.
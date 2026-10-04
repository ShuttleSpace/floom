# Porting `floem-ui` from the v1 API to the Floom 2.0 API

## Resume here

Branch `floem2.0`. The three component *shapes* are now ported — one per shape,
so the widget API gets shaken out before anything is translated in bulk:

| Shape | Example | Status |
|---|---|---|
| surface | `md-card` | ported, 7 tests |
| interactive | `md-button` | ported, 10 tests |
| stateful | `md-expansion_panels` | ported, 12 tests |

Ported kits: `md-card`, `md-button`, `md-icon`, `md-expansion_panels`.
Engine widgets: `Badge`, `Ripple`, `VectorIcon`, `Pressable`, `Accordion`,
**`Surface`**.

**The next task is a component again.** The capability work is done — a clickable
subtree, a padding/border vocabulary, and a decorated box that can hold content
all landed, and between them they unblock the 146 / 172 / 94 files listed below:

1. **Batch the surface components** — `footer`, `toolbar`, `item_group`,
   `list_subheader`, `banner`. These are now genuinely uneventful: a `Surface`, a
   `Text`, maybe a `Stack`.
2. **`floom-elements` needs repairing, not just rewriting** (correction 6). Eight
   material crates are broken by the ports already done; they are the *callers* of
   `MdIcon` and `Ripple`, so each is a small edit rather than a port.
3. Then port one kit end to end, and the other two follow its shape.

Verification commands that must stay green:

    # the whole core workspace
    cargo check --workspace --exclude calculator --exclude floem2-capabilities \
          --exclude floem-legacy-renderer-skia --all-targets

    # the engine widget crate
    cargo test --manifest-path crates/floom-widgets/Cargo.toml

    # the layout crate (the padding / margin vocabulary lives here)
    cargo test --manifest-path crates/floom-layout/Cargo.toml

    # the nested floem-ui workspace (a separate workspace)
    cd crates/ecosystem/floom-ui
    cargo test --manifest-path crates/material/md-expansion_panels/Cargo.toml

The three `--exclude`d packages are **pre-existing** breakages, proved against a
clean `origin/floem-2.0` worktree, not regressions: `examples/calculator` (manifest
points at `src/lib.rs`, the file is `src/librs`), `floom2-capabilities` (7
Android/iOS errors `cfg`'d out on macOS), and `floom-legacy-renderer-skia` (its
`skia-bindings` build script needs network). `--exclude` does not actually
suppress the last two — they come back in through other packages' targets — so the
gate is "8 error lines, all in `floom2-capabilities`", counted before and after
every change. The same discipline applies to `floom-elements`, where the count is
20 and the cause is *not* pre-existing (correction 6).

**`floom-elements --features material` is not green, and this document was wrong
to say it was** (correction 6).

Six claims this document has had to correct about itself. Each is recorded with
its evidence rather than quietly overwritten, because every one was a confident
assertion made from reading code instead of compiling it:

1. "79 components need new widgets" — wrong: `impl View for` appears **zero**
   times across all three kits. All 98 are declarative compositions.
2. "`Ripple` needs an interaction-animation capability in the engine" — wrong,
   twice. `floom-app/tests/event_animation.rs` proves it works today.
3. "the v1 monolith can be deleted without porting `floem-ui`" — wrong: 298 of
   its 299 crates depend on v1.
4. "Stage B requires zero new widget types" — right about *components*, wrong
   about *capabilities*: 2.0 had no way at all to make an arbitrary subtree
   clickable, and floem-ui asks for that 360 times.
5. "the port is a mechanical style-closure translation" — wrong: 2.0 had **no
   padding and no border**. 288 padding calls across 172 files; 119 `.border(`
   across 94.
6. "**`floom-elements` now builds clean with `--features material`**" — wrong,
   and stated without having run it. At this commit it is
   **20 errors across 8 material crates** (`--keep-going`):
   `md-alert`, `md-autocomplete`, `md-combobox`, `md-dialog`, `md-icon_button`,
   `md-list_item`, `md-select`, `md-treeview`. All of them are *callers* of the
   two ports that already landed: `MdIcon::view()` and `Ripple::new(x).view()`
   are gone, and the `.view()` terminal call was the whole point of removing them.
   Porting a shared base therefore breaks its callers, and the plan had no step
   for that; `md-icon`'s note about "six material crates still on v1" named the
   right crates and then did not go and fix them.

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

## The gap that gates everything — corrected twice more

My first estimate here was wrong, and it mattered, so this section records the
corrections rather than quietly replacing them.

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

So **Stage B requires no new *component-shaped* widgets.** 2.0's `Container` /
`Text` / `Stack` plus `Child::new(ChildKey, LayoutRequest, ..)` composition can
express every one of them as a *structure*.

What the second measurement missed is that those four v1 built-ins carry
**capabilities**, not just structure, and 2.0 was missing three of them. Counting
call sites rather than component names:

| v1 capability | call sites in floem-ui | 2.0 at the time | now |
|---|---|---|---|
| `.on_event(listener::…, …)` on any view | **360** (146 files; 209 are `Click`) | nothing — `Button` is a label, not a wrapper | `Pressable` |
| `Container::new(child).style(..)` | **165** | nothing — `Container` is a leaf | `Surface` |
| `.padding*()` | **288** (172 files) | `Container::padding` stored, never read | `LayoutRequest::padding` / `margin` |
| `.border()` / `.border_color()` | **119** / **145** (94 files) | nothing — no stroke primitive on a box | `Container::border` |
| a widget whose own layout moves after mount | every theme change | `mark_dirty` repainted yesterday's geometry | `UpdateContext::mark_layout_dirty` |
| height transition on mount | `md-expand_transition` | nothing | still nothing (see below) |

So the honest statement is: **Stage B needs no new component shapes, and did need
five new capabilities — four of which are now in.** Every one of them was found by
compiling a component, not by reading the tree, which is the argument for shaking
out one component per shape before translating in bulk.

- **The real translation cost** is the v1 style closure
  (`.style(|s| s.padding(..).background(..).rounded(..))`) becoming 2.0's
  explicit builder calls (`Container::new().background(..).radius(..)`) plus
  `LayoutRequest::margin(..)` on the child's slot. Mechanical per component, and
  it is the bulk of the work.
- **The three kits are not duplicates** — material/cupertino/fluent differ by
  ~60% of their lines (e.g. `card` differs on 71–75 of 115 lines), so this is
  genuinely 3× work despite a shared shape.
- **The one genuinely new capability left is interaction animation** for
  `Ripple` — which turned out not to be missing (see Stage A).

Scale: 98 components × 3 kits ≈ 50,000 LOC of composition to rewrite.

Already covered by an existing 2.0 widget (19): `button`, `checkbox`,
`container`, `dialog`, `divider`, `grid`, `icon`, `image`, `list`,
`navigation_drawer`, `overlay`, `radio`, `select`, `slider`, `spacer`, `switch`,
`tabs`, `text`, `text-field`.

**Done:** `badge` (`crates/floom-widgets/src/badge.rs`, 10 tests) — genuinely
worth a widget type, because it is small, self-contained, and sets the pattern.

### The pattern a Stage B widget follows

`Badge` is the reference implementation, because it fixes the seam between 2.0 and
floem-ui:

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

### Stage B — the kits

Shake the widget API out with one component per *shape* before translating in
bulk — a bulk port on top of an unexercised widget API just relocates the
mistakes.

| Shape | Example | Status |
|---|---|---|
| surface | `md-card` | ported, 7 tests |
| interactive | `md-button` | ported, 10 tests |
| stateful | `md-expansion_panels` | ported, 12 tests |
| *(the box they all draw in)* | `Surface` | ported, 17 tests |

### What `md-button` turned up

All of it a real gap rather than a translation nuisance:

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

### What `md-expansion_panels` turned up — the stateful shape

The stateful shape is the one that is *not* a translation at all. v1 held the
state in an `RwSignal<Vec<bool>>` that each panel read inside a `dyn_view`
closure and closed over its own index. 2.0 has no reactive read during render, so
the state machine had to move into a widget. Three things came out of that, and
the first two are now engine surface:

* **`Pressable` — the missing half of `.on_event`.** `Button` *is* its label: it
  paints a run of text and nothing else. An expansion-panel header is a title plus
  a chevron; a list row is an icon, two labels and a trailing icon. Those need the
  interaction without the drawing. `Pressable` wraps one child, consumes the whole
  surface, activates on a matched press/release *or* on an activation key while
  focused, and reports `EXPANDED`/`COLLAPSED` from a **reader** rather than a
  value captured at build time (see the focus note below).

* **`Accordion` — shared state, because a click lands far from what it changes.**
  The press lands on a `Pressable` deep inside a header subtree; the answer is
  needed by that panel's body, by its indicator, and in accordion mode by every
  *other* panel. None of those owns the accordion's node. So the open set is a
  shared `Arc<Mutex<Vec<bool>>>` — exactly the shape `Tabs` uses for its selection
  — and every widget that cares reads it in its own `update` and marks itself.
  The host visits every node each frame, so no notification has to travel, and
  `Accordion::update` is deliberately empty (`Crossfade::update` says why, in the
  same words).

* **A header must keep its identity across its own toggle.** A rebuilt header is a
  new node, so the keyboard focus the press requested is thrown away by the very
  act of activating it. That constrains the tree in a way nothing else did: every
  `Child` inside the header needs a *stable* key, which means the chevron cannot
  be a plain `Child` (the host only rebuilds a child whose key changed). It is a
  small widget that re-declares its own single child keyed by the open flag, which
  is the whole trick. `Pressable::expanded_from` is the matching half on the
  semantics side.

* **Padding had to become a margin, and that is a design decision, not a rename.**
  v1 wrote `Container::new(content).style(|s| s.padding_horiz(…))`. 2.0's
  `Container` is a leaf, so the inset is `LayoutRequest::margin(..)` on the
  content's own slot. It renders the same (the parent's fill still covers the
  inset) and it composes with the layout engine, which had no padding concept at
  all.

### What the stateful shape could not do: a height reveal

**This document previously said the stateful shape "needs only a height
transition". That was wrong, and it is the one gap here that is not fixed.**

v1 has a real one. `md-expand_transition` wraps the body in
`.animation(|a| a.keyframe(0, height(0.0)).keyframe(100, computed_style()))`, so
the box grows from nothing and the space it occupies grows with it. 2.0 cannot
express that today, for two independent reasons:

1. **No compositor channel carries a size.** `CompositorProp` is
   `Opacity | TranslateX | TranslateY | Scale | Rotate` and
   `PresenceTransition`'s channel is `Fade | Grow | Slide` — every one of them a
   paint-time transform, none of them a re-layout. This is already recorded as
   unfinished work in `docs/2.0/MOTION-GAP.md` (B4, the "size" entry).
2. **The host does not re-lay out per frame.** `WidgetHost::frame` runs
   `if self.layout_dirty { self.run_layout(); }`, and nothing in the frame loop
   marks layout dirty while a tween runs. So even a widget that *wanted* to report
   a growing `layout_request` has no clock to read it from: `animation_value()`
   exists on `PaintContext`, not on `UpdateContext`.

What the port offers instead is `Accordion::content_transition`, which is
compositor-only — a fade, a grow or a slide over the revealed subtree, with the
box already at its final height — and it is **off by default**, because v1's
`md-expansion-panels` swapped its body with a bare `dyn_view` and adding a default
fade would be a behaviour change dressed as a translation. The negative test is
`the_reveal_does_not_resize_the_panel_over_time`.

Then:

1. **Batch the surface components** (`footer`, `toolbar`, `item_group`,
   `list_subheader`, `banner`) — these should now be uneventful: a `Surface`, a
   `Text`, maybe a `Stack`.
2. **Repair `floom-elements`' callers** — the eight material crates correction 6
   names. Each is a small edit, not a port: they call `MdIcon::view()` and
   `Ripple::new(x).view()`, both of which are gone on purpose.
3. **Port one kit end to end** (material, 98 components), then cupertino and
   fluent, which follow the same shape with their own tokens.
4. **`floom-elements`** last: its 800 lines of re-exports and 6,220 lines of v1
   `multi` dispatch can only be rewritten once the underlying components exist.

### What building the decorated box turned up — `Surface`

The last of the three shapes turned out not to be a component but the *box* every
component draws in, and it produced two findings that no amount of reading would
have.

**`Container` cannot hold children, and that is not a detail.** 2.0's `Container`
is a leaf by design — composition is declared by the parent — which leaves the
most common v1 shape in the whole library with no spelling at all:
`Container::new(child).style(..)`, 165 uses. `Surface` is the missing third kind
of widget: `Container` is the decoration description, `Stack` is the structure
description, and a card is both. It holds a `Container` whole rather than copying
its fields, so there is one implementation of "fill, then outline, then shadow,
then the backdrop filter" and the two cannot drift apart about how either draws.
It also keeps **one** revision of its own rather than forwarding the decoration's,
so a frame reports dirty at most once per node however many fields moved — and
`decoration()` hands out a shared reference, because handing out the fields
mutably is a way to change a box with no way to invalidate it.

**Padding had to become real, and it is two different things.** `LayoutRequest`
now carries both `padding` (inside the box, between it and its children) and
`margin` (outside the box, inside its parent). The first attempt at this reached
for a margin and was half right — with a *single* child the two are geometrically
identical, so every test written against one child passes for either
implementation. The measurable difference needs two children: a margin is per
child and lands *between* them as well, while a padding is one inset around the
group. `floom-layout`'s `a_margin_is_per_child_and_a_padding_is_around_the_group`
is that measurement, and it is why a card is a padded box rather than a margin on
its content.

**The third door: a widget could not say "my layout moved".** `UpdateContext`
offered `mark_dirty` and `mark_children_dirty`, and a padding change is neither —
it needs the *layout pass*, which `WidgetHost::frame` runs only when layout is
dirty. So a `Surface` whose padding moved after mount reported dirty, repainted,
and left the content exactly where it was, because the geometry it repainted into
was the previous frame's. `UpdateContext::mark_layout_dirty` is the missing
request, and it is deliberately **not** defaulted: there is no honest fallback,
because a host that ignored it would mis-lay-out every widget that used it,
silently. `FrameStats::layout_requested` counts the requests so a test can assert
the ask rather than infer it.

The cost control matters as much as the door and has its own test: a theme toggle
moves colours and radii on every surface in a tree, so `mark_layout_dirty` fires
only when the *padding* moved
(`a_moved_colour_does_not_ask_for_a_layout_pass`).

**And `md-card`'s padding was wrong three separate times**, which is the clearest
argument in this document for the last rule:

1. `Container::new().background(..).radius(..).padding(..).shadow(..)` — a
   `padding` field with a setter and no reader. The card rendered with its text
   against the edge, and its test passed, because it asserted the token was
   *positive*.
2. `content_layout()` — a margin on the content's slot. Applied, at least, but the
   content still was not *inside* the card.
3. `Surface` — a filled, rounded, elevated box that holds its content and pads it.
   The test now asserts the inset is found on the surface **and** leaves as
   `padding`, which is the only assertion that would have failed for 1 and 2.

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
| `floom-elements` | 8,293 | heavy — `View`, `IntoView`, reactive reads | todo, **and already broken** |

`floom-core-theme` turned out to need almost nothing from v1: theme state is
just `RwSignal` + `create_global_rw_signal` (which live in `floom-legacy-reactive`
and are already re-exported by the 2.0 prelude), plus two value conversions.
It now depends on `floom-legacy-reactive`, `peniko` and `parley` directly — no
widget layer, no `floom`.

`floom-widget-runtime` is the crate 99 others depend on, and **91 of them use
only `theme_signal`**, so porting it was high leverage. Its theme plumbing is now
2.0-native. Two of its helpers changed shape because 2.0 has no wrapper widget:
`theme_provider` and `layout_provider` used to take a child and return
`impl View`; 2.0's `Container` is a leaf (paint, background, filters) with no
`.child()` builder, because composition moved to the parent via
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
`.view()` terminal call goes away. `floom-widget-md-list-item` is the only crate
in the material set that breaks on it today, and it is one of the eight in
correction 6.

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

Only after `floem-ui` no longer references it:

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
- **Check that a value you hand a widget is actually read by it.** `Container` had
  a `padding` field with a setter and no reader for the whole life of the 2.0
  port, and `md-card`'s test passed the whole time because it only asserted the
  token was positive. `grep -n <field> crates/floom-widgets/src/` is the check,
  and it is cheaper than the bug.
- **A widget that hands out its own fields is handing out a way to change them
  without invalidating them.** `Surface` therefore exposes `decoration()` shared
  and `set_*` in place, never a `&mut`. The same reasoning as the rule above,
  turned into an API shape instead of a habit.
- **A test written against one child cannot tell a margin from a padding.** The
  two are geometrically identical for a single child; the difference only appears
  with two, where a margin also lands between them. Write the two-child test.
# Porting `floem-ui` from the v1 API to the Floom 2.0 API

## Resume here

Branch `floem2.0`. The three component *shapes* are now ported — one per shape,
so the widget API gets shaken out before anything is translated in bulk:

| Shape | Example | Status |
|---|---|---|
| surface | `md-card` | ported, 7 tests |
| interactive | `md-button` | ported, 10 tests |
| stateful | `md-expansion_panels` | ported, 12 tests |
| self-composing | `md-toolbar` | ported, 13 tests |
| separator | `md-footer` | ported, 14 tests |
| selectable | `md-item_group` | ported, 11 tests (state machine promoted) |
| surface + press | `fluent-card` | ported, 14 tests (first fluent kit) |

Ported kits: `md-card`, `md-button`, `md-icon`, `md-expansion_panels`,
`md-toolbar`, `md-footer`, `md-item_group`, **`fluent-card`**.
Engine widgets: `Badge`, `Ripple`, `VectorIcon`, `Pressable`, `Accordion`,
**`Surface`**, **`SelectGroup`**.
Text: **`FontWeight`**, end to end — `floom_types` enum, `TextLayoutOptions`,
both glyph caches, the measure comparison, the paint op.

**The next task is `floom-elements`.** The surface batch is done — `md-toolbar`,
`md-footer` and `md-item_group` are ported, and the last of them turned out to be a
70-line wrapper because its state machine belongs in the engine.

**The next task is a port, not a repair.** Correction 7 established that the 14
crates broken by the ports already done are **not small edits** — they are ports,
and no repair exists that keeps this plan's rules. So:

1. ~~**Port the cheapest of the 14 first**~~ — **done**: `fluent-card`, 14 tests.
   The floem-ui workspace is now at **24 errors across 13 crates** (was 26 across
   14; `fluent-card`'s two `Ripple` errors are gone), so the arithmetic is the
   check that the count in correction 7 was right.
2. **`cupertino-card` next** (109 lines, the same shape with `radius().md` where
   Fluent uses `radius().xl`), then the two `icon_button`s (117/117/118) and the
   three `list_item`s (172 each) — the last of those is one shape three times and
   wants the same promotion `md-item_group` got.
3. Then `md-alert`, `md-dialog`, `md-select`, `md-treeview`, `md-combobox`,
   `md-autocomplete` — and with them the floem-ui workspace goes green, at which
   point `floom-elements`' own 6,220 lines of v1 `multi` dispatch can be rewritten
   (correction 6).

Verification commands that must stay green:

    # the whole core workspace
    cargo check --workspace --exclude calculator --exclude floem2-capabilities \
          --exclude floem-legacy-renderer-skia --all-targets

    # the engine widget crate
    cargo test --manifest-path crates/floom-widgets/Cargo.toml

    # the shaping crate, where the weight and both caches' keys are pinned
    cargo test --manifest-path crates/floom-text/Cargo.toml

    # **every ported kit**, from the nested workspace. This line was missing until
    # the `Length` change broke two of these suites and nothing noticed for two
    # commits — see the correction below.
    cd crates/ecosystem/floom-ui
    for c in md-card md-button md-icon md-expansion_panels md-toolbar md-footer md-item_group; do
        cargo test --manifest-path "crates/material/$c/Cargo.toml"
    done
    for c in fluent/fluent-card; do
        cargo test --manifest-path "crates/$c/Cargo.toml"
    done

    # the layout crate (the padding / margin / percentage vocabulary lives here)
    cargo test --manifest-path crates/floom-layout/Cargo.toml

    # the host, including the padding and percentage suites that close the loop
    # from a widget's request through to a painted box
    cargo test --manifest-path crates/floom-app/Cargo.toml

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

7. **Correction 6's own remedy was wrong**, twice over: the count, and the claim
   that each break is "a small edit rather than a port". Both were assertions from
   reading code, and one experiment each is what settled them.

   **The count.** `cargo check --workspace --keep-going` in the nested workspace
   reports **26 errors across 14 files in all three kits**, not 20 across 8
   material ones — because the breakage grew as more ports landed (`Ripple`'s
   re-export is what reached cupertino and fluent), and because the earlier count
   only ever looked at material.

   The count was re-measured after `fluent-card` was ported, and came back
   **24 errors across 13 crates** — exactly `26 - 2`, which is the two `Ripple`
   errors in `fluent-card`. A count that moves by precisely the amount a change
   should move it is the cheapest available check that the original number was
   real rather than remembered.

   | crate | lines | `floom::` references |
   |---|---|---|
   | `fluent-card`, `cupertino-card` | 107, 109 | 3, 4 |
   | `fluent-icon_button`, `cupertino-icon_button`, `md-icon_button` | 117, 117, 118 | 3, 3, 3 |
   | `fluent-list_item`, `cupertino-list_item`, `md-list_item` | 172, 172, 174 | 5, 5, 5 |
   | `md-alert`, `md-dialog` | 170, 271 | 4, 4 |
   | `md-select`, `md-treeview`, `md-combobox`, `md-autocomplete` | 263, 259, 284, 315 | 7, 7, 7, 7 |

   **Not a small edit.** Two experiments, both reverted:

   * *Can `.view()` just be dropped?* No. Deleting it from `md-alert` moves the
     error down one line rather than removing it: `the method style exists for
     struct MdIcon, but its trait bounds were not satisfied`. `MdIcon` is a 2.0
     `Widget`; its callers are v1 crates that still depend on the `floom`
     monolith, and a v1 `Decorators` chain needs `impl View`. **There is no v1
     view to return**, so the repair would be to port the click onto
     [`Pressable`] — which is a port.
   * *Can the v1 `Ripple` wrapper be restored to `floom-widget-runtime`?* No. It
     is still in git history (234 lines) and restoring it fails immediately:
     `cannot find module or crate floem`. The wrapper needs
     `floom::context::LayoutChangedListener` and `floom::animate::Animation`,
     because a v1 *view wrapper* has to learn its own box before it can size the
     ink. 2.0 hands `paint` and `event` `cx.bounds()` directly, which is why the
     engine version needs no listener. So restoring it would put a v1 `floom`
     dependency back into a ported crate — the one thing this plan forbids.

   **What this changes about the plan.** `floom-widget-runtime`'s note claimed
   that re-exporting the 2.0 `Ripple` was right because "91 of 99 crates reach it
   through this path". It reached them, and it broke all nine that were still v1,
   because a v1 `Container` cannot be handed to a 2.0 widget's builder. So:

   * there is **no repair** that keeps the plan's own rules — every option is
     either 14 ports, or a new v1-only compatibility crate to be deleted in Stage D;
   * the floem-ui workspace **cannot be made to compile before those 14 crates are
     ported**, and the honest thing is to say so rather than carry them as a
     backlog of edits;
   * so they are no longer a repair step. They are the first 14 entries of "port
     one kit end to end", and `cupertino-card` / `fluent-card` are the cheapest
     way in, because `md-card`'s shape is already proven.

8. **"The kit test suites are green" meant "they were never run."** Porting
   `fluent-card` surfaced this: the crate's own `tests/api.rs` contained
   `assert_eq!(style.border_radius, 4.0)`, and no design system in the table says
   `4.0` — `CardTokens::from_spacing`, which is what Fluent resolves, says `12.0`,
   and Material's card token says `10.0`. The assertion was wrong before the port.

   It had never been caught because **the crate did not compile**: its v1 `lib.rs`
   still called `Ripple::new(inner)` and `.radius_fn(..)`, both removed by the
   ports, so every run stopped at the build and the assertion sat behind that error
   for the life of the crate. This is the same omission as the `Length` correction
   further down — a `floem-ui` suite that no root-workspace command reaches —
   except that this one was not merely skipped by the verification list, it was
   unreachable.

   The lesson is narrower than "write more tests". A test suite behind a compile
   error is not a safety net; it is a comment. **Every unported crate's test suite
   should be assumed to contain a stale assertion, and none of them can be
   believed until its crate builds.** That is an argument for porting in
   dependency order and re-running each suite the moment its crate goes green —
   not for reading ahead and judging the tests.

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
| `.border()` / `.border_color()` | **119** / **145** (94 files) | nothing — no stroke primitive on a box | `Container::border` (**all four sides only**) |
| `width_pct` / `height_pct` | **231** / **79** | nothing — `LayoutRequest` had no percentage | `Length::Pct` |
| `justify_between()` | **38** | the layout layer had `SpaceBetween`; the vocabulary did not | `Align::SpaceBetween` |
| a widget whose own layout moves after mount | every theme change | `mark_dirty` repainted yesterday's geometry | `UpdateContext::mark_layout_dirty` |
| `font_weight(…)` | **73** | `Text` has size and colour, no weight | `FontWeight` end to end |
| `border_top` / `border_bottom` / `border_left` / `border_right` | **7 / 14 / 3 / 3** | nothing | `Edges` + `border_edges(..)` |
| height transition on mount | `md-expand_transition` | nothing | still nothing (see below) |

So the honest statement is: **Stage B needs no new component shapes, and has
needed nine new capabilities — eight of which are now in.** Every one was found by
compiling a component, not by reading the tree, which is the argument for shaking
out one component per shape before translating in bulk. The two still open, in
the order their call counts justify:

1. **A height transition — see below.** Not cheap, and not a component problem.
   It is the last item on this list, and the only one whose cost is design work
   rather than plumbing.

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
2. **Port `floom-elements`' callers** — the 14 crates correction 7 names, across
   all three kits. Correction 6 called each "a small edit"; two experiments showed
   otherwise, because `.view()` has no v1 view to return and the v1 `Ripple`
   wrapper cannot be restored without putting v1 back into a ported crate.
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

### Percentage sizing, and a bug it ran into

Reading the three surface components (`footer`, `toolbar`, `item_group`) to batch
them turned up the largest remaining gap and one genuine engine bug.

**`width_pct` / `height_pct` — 310 call sites, no spelling at all.** v1 had
`width(..)` and `width_pct(..)` over one field; 2.0 had only the first, so every
full-width row in the library — a footer, a toolbar, a banner — was inexpressible.
`LayoutRequest.width`/`height` are now `Option<Length>` with
`Length::{Px, Pct}`, which is a breaking change to a small, well-contained type:
the compiler found all 30-odd sites, which is exactly why it was done as a type
rather than as a second `width_pct: Option<f32>` field. Two parallel fields would
have left "both set" representable, and a percentage beats a length or loses to it
depending on which line you read.

`Align::SpaceBetween` came in the same pass and is nearly free: the layout layer
has carried `JustifyContent::SpaceBetween` since the beginning, so only the
engine-neutral vocabulary was missing it. On the cross axis it lowers to `Start`,
because flexbox has no cross-axis "distribute between" and inventing one would be
worse than saying so.

**The bug: a definite child width loses to the cross-axis stretch.** Writing the
control for the percentage — "a fixed 120px width must not also come out 300" —
failed, and it is not a `Length` problem: `width: Some(120.0)` lowered to the same
`Dimension::length(120.0)` before this work. What is known:

* `WidgetHost::reconcile` **does** write the width first. A temporary probe in the
  reconciler printed `slot_w=Some(Px(120.0))` and `final_w=Some(Px(120.0))` for the
  node whose box then painted at 300.
* `floom-layout` does **not** reproduce it. A column with the default
  `align_items: Stretch` and a child of `width: 120, height: auto` lays out at 120
  in every combination probed, and so does one with both dimensions definite.

So the width is lost between `set_style` and the painted box, in the host. It is
recorded as `a_definite_child_width_is_stretched_by_the_default_cross_axis` in
`floom-app/tests/surface_padding.rs` — a test that **passes** by asserting the
current behaviour and whose doc comment says to delete it when the behaviour
changes — because the honest response to a bug of unknown cause is to write down
what is known rather than to route around it. Two consequences worth carrying:

* A "full width" and a "fixed width" are indistinguishable through the host, both
  coming out as the parent's width. The *value* distinction is pinned at the
  layout layer instead (`a_fixed_length_and_a_percentage_diverge_as_the_parent_grows`).
* The kits want a fixed-width row inside a full-width container more often than
  one might hope — a chip, a badge, a button. Until this is understood, every one
  of those will silently come out stretched.

### Font weight, and two things it turned up

`font_weight` is the last big vocabulary gap (73 call sites, 68 of them `Medium`
or `SemiBold`). It is not a widget field, and the reason is worth writing down
because the shape of the mistake is invisible: **two caches return shaped glyph
runs on a hit, so a key without the weight answers a `Bold` request with the
`Normal` glyphs for the same string.** Same text, same size, wrong face. Nothing
in the result says so, and nothing in a code review would either.

So the weight went into four places, each of which has a test that fails without
it:

* `floom_types::FontWeight` — the nine CSS weights, named as CSS names them
  because floem-ui already writes `FontWeight::Medium` at 50 call sites and a
  port should not also be a rename.
* `floom_text::TextLayoutOptions.weight` and `LayoutCache`'s key, which mirrors
  that struct field for field *on purpose*.
* `PaintContext::fill_text`, which gained the parameter rather than gaining a
  sibling method. 20 call sites, all mechanical.
* the host's glyph cache (`glyph_hash` **and** the retained entry) and
  `sync_text_measure`'s comparison, so a weight that changes *after mount*
  re-measures and re-shapes.

**How much of that is load-bearing, measured rather than assumed.** The host keeps
the weight in two places in the glyph cache. Three mutations, three answers:

| Mutation | Result |
|---|---|
| drop it from `glyph_hash` only | **all 8 tests green** — correct output, one re-shape per frame per text node |
| drop it from the retained entry's comparison only | **all 8 green** — redundant; the hash already separates the buckets |
| drop both | **3 tests red** |

One line is load-bearing and the other is defence in depth, and it is not obvious
from reading either which is which. The hash-only variant's cost is real and
invisible to every assertion in the suite, which is the kind of thing worth
knowing before someone "simplifies" it.

**And a correction to this document's own first draft.** The first version of
`a_heavier_weight_measures_wider` asserted `Medium` measures strictly wider than
`Normal`. It does not, and cannot be made to: the default font stack's faces
generally ship a regular and a bold and nothing between, so font matching resolves
`500` to the same face as `400` — correctly, per CSS. `Medium` measures
*identically*; `Bold` measures wider. Asserting the former would have been
asserting something about the installed fonts rather than about the code, and it
would pass on a machine with a variable font installed and fail on one without. The
test now asserts `Medium >= Normal` plus `Bold > Normal`, and the same reasoning is
carried into the widget and host suites so the trap is not re-set there.

**Then a bug that has nothing to do with weight.** `md-toolbar`'s `Medium` title
is 79.078px wide. The host's measure produced that, the node's box came back
**79.0**, and the paint path re-shaped the same string against the box — so the
text wrapped to two lines inside a box that had just measured it as one. Chasing
it found the real rule:

* A measure leaf's box comes back from `floom-layout` **rounded to whole pixels**
  (`79.078125` in, `x1: 79.0` out — pinned by
  `a_measure_leafs_box_is_rounded_to_whole_pixels`, with
  `the_rounding_goes_both_ways` for the other direction).
* The paint path wrapped against that rounded box. A string whose natural width
  lands in the **upper half-pixel** of its own box therefore measured in one line,
  laid out in one line, and painted in two.
* Over 2997 random regular-weight strings, **498 of them — one in six — wrapped at
  their own box.** This is a pre-existing bug, not a weight bug: weight only moved
  one string (`Toolbar title`, 71.59 regular → 79.08 bold) across the boundary.

The fix is to wrap against the width the text was actually wrapped against, which
the measure closure already knows: `TextMeasure::wrap_width` records the
`avail.width` it was handed, and the paint prefers it over the rounded box. The
subtlety, which cost one wrong implementation to find, is that it must be the
*available* width and **not** the measured result. Recording what the measure
returned looks equivalent and is not: for a string wider than its box that result
is the wrapped width (fine), but for one measured against a wide available space
it is the string's natural width, and feeding that back as the paint wrap width
stops the label wrapping at all. `a_label_that_needs_two_lines_still_wraps` is the
test for that half.

### Per-side borders, and a shape question worth answering out loud

The last capability gap: 27 call sites (`border_bottom` 14, `border_top` 7,
`border_left` 3, `border_right` 3) that `Container::border` could not express,
because a `Stroke` traces a whole path and the only path a box had was the full
outline.

`floom_types::Edges` is a four-bit set whose `Default` is `ALL`, so every existing
`border(..)` call, test and kit crate means exactly what it meant, and
`border_edges(w, colour, Edges::TOP)` is additive. `Border` became a spec struct
(width, colour, edges) because `Surface::set_border` was comparing the lossy
`(width, colour)` pair, and a change of *edges alone* compared equal — which is the
change a selected row in `md-item_group` makes. `border_style()` is kept and
documented as lossy; `border_spec()` is the honest one.

**The interesting decision is what one edge looks like.** A band, or a partial
rounded outline? The paint does bands — filled rects — and the reasons are worth
stating rather than leaving in the diff:

* *Why not curved.* CSS would curve it. A footer's rule and a toolbar's divider
  are straight lines, and curving them raises a question this API has no opinion
  about: what happens where two drawn edges meet. Choosing straight means the
  answer is visible in the drawing rather than hidden in a corner rule.
* *Why a fill and not a stroke.* A stroke straddles its path, so half of a 1px
  band would land outside the box and be clipped away by any clipping ancestor. The
  existing whole-outline border already has to inset by half its width for the same
  reason; a band starts at the edge instead, and is exactly `width` thick.
* *`Edges::ALL` keeps the rounded stroke*, so `border(1.0, c)` and
  `border_edges(1.0, c, Edges::ALL)` are the same drawing — asserted, because two
  spellings that merely look alike are how a footer silently loses its corners.

And the fill keeps its own radius either way: only the outline ignores `radius`.
Dropping it from both would turn a rounded box square the moment a rule was added,
which is `a_straight_band_does_not_flatten_the_fill`.

Two mutations, checked rather than assumed: routing single edges through the
rounded stroke turns 3 tests red, and losing the second of two edges (`else if`
instead of a second `if`) turns exactly `each_edge_band_sits_on_that_edge` and
`two_edges_draw_two_bands` red.

### What `md-toolbar` turned up — the first self-composing component

The first ported component that brings its own content, and so the first one whose
tests are about composition as much as decoration. Three things came out of it.

**1. A slot is a replayed `Fn`, so a component cannot close over itself.** v1's
`actions(impl IntoView)` stored a *value*; 2.0's `push` takes
`impl Fn() -> Box<dyn WidgetAny> + Send + Sync`, and the compiler says so in the
least helpful way:

* a closure that moved a built `Icon`/`Text` out of the capture fails with
  "cannot move out of a captured variable in an `Fn` closure" — the widget has to
  be *rebuilt* on every replay;
* a closure that captured `self` fails with "`Rc<()>` cannot be sent between
  threads", because `MdToolbar` holds an `RwSignal` and 2.0's signal is
  `Rc`-based.

So the shape is: **resolve the theme to plain data first, then close over
`String`s, `f32`s and colours.** `bar_of` and `leading_group_of` are free functions
over exactly that, and `view()` is the composition of them. The actions slot is an
`Arc<dyn Fn ..>` rather than a `Box` for the same reason one level down — the
surface's slot is itself replayed, so every rebuild needs its own copy.

**2. Two engine capabilities this component was the first to need**, both of which
the previous commits happened to add:

* `FontWeight`, because v1's title is `FontWeight::Medium`. It is a literal in v1,
  and it stayed a literal — but it is now a *field on the resolved style*, so the
  style struct states the weight instead of the widget builder hard-coding it.
* `Surface::request`, because v1 styled the bar `.width_pct(100.0).height(h)`. A
  decorated box could only shrink-wrap: its `layout_request` carried padding and
  nothing else, so the only way to say "64px and full width" was to put the request
  on the *parent's* slot for the child, which spreads one box's styling across two
  widgets. The request now merges with the padding rather than replacing it.

**3. A record, not a change: the theme's own weight table is wrong.**
`floem_core_theme::tokens::typography::FontWeight::to_floom` maps both `ExtraBold`
and `Black` onto parley's `EXTRA_BOLD` (800), so the two are indistinguishable
there. `FontWeight::Black` has **no call site** in material, cupertino or fluent,
so the mapping is left alone — a fix there would change nothing observable and
could not be verified. The kit's own conversion maps all nine weights distinctly,
and a test says so.

Two v1 oddities kept deliberately, each with a comment saying why: the leading
icon's 22px and the title's `Medium` are literals rather than tokens, and the bar's
height comes from `ToolbarProps::height` while `ToolbarTokens::height` — which v1
also declared — stays unread. Reading the token instead would be a behaviour change
wearing a cleanup's clothes.

### A correction: the verification list did not include the kits

`Length` landed two commits ago and broke two ported kits' *test* suites
(`md-expansion_panels`, `md-icon` — `assert_eq!(req.width, Some(120.0))` against a
field that is now `Option<Length>`). Nothing noticed, because the porting plan's
verification list named the core workspace and five core crates and **not one kit
crate**, and `floem-ui` is a separate nested workspace with its own `Cargo.lock`,
so a root `cargo test` never reaches it.

That is the same failure this document keeps recording in a different form: a check
that was not in the list did not happen. The list now ends with a loop over every
ported kit, and the comment on it says why it is there.

### What `md-footer` turned up — the component the separator was for

The footer is what the `Edges` work was for: v1 styled it
`.border_color(outline_variant).border_top(tokens.border)`, and a four-sided stroke
cannot say "top". 14 tests, three mutations checked rather than assumed:

| Mutation | Tests red |
|---|---|
| `border_edges` returns `Edges::ALL` | exactly the two separator tests |
| `view` stops calling `publish_layout` | exactly the two layout-registry tests |
| the content is dropped instead of pushed | exactly the two content tests |

Three things it settled:

**`items_center` is not a property of the fill.** v1 put it on the box. In 2.0 the
*child* is what gets centred, so it went into `Surface::request(..)`'s
`main_align`/`cross_align` — which is the payoff of letting a decorated box
declare its own extent. The whole of v1's styling now lives on one widget: the
extent, the padding, the separator, the elevation, and the content inside.

**The layout registry is a real dependency, not bookkeeping.** v1 called
`register_layout(LayoutSlot::Footer, height)` while building the view, and
`md-main` reads that process-wide signal to inset the content region. A port that
dropped the call would look finished and overlap every footer's content with the
bar. It is kept as a build-time side effect — not reactive — and
`MdFooter::publish_layout` says why making it reactive is not available: it would
mean holding the theme signal inside a `Send + Sync` slot closure, which 2.0's
slots cannot do. Two tests pin the publication, including the token default, since
a caller who never sets a height still has to publish something.

**`color(..)` on a box has nowhere to live.** v1 set the footer's content colour
on the box. 2.0's `Surface` paints a fill and only the child knows it is text, so
the colour is a field on the resolved style and a method,
`MdFooter::content_color`, for the child to use. Recorded because it is the third
component in a row where a v1 style closure property has no 2.0 home, and the
pattern is worth naming: *the box owns the fill, the child owns the ink.*

`Edges` is now re-exported from `floom-widgets`' root beside `Align` and `Axis`,
for the same reason those two are: a kit author writing `border_edges` should not
have to reach past this crate for the type.

### What `fluent-card` turned up — the first Fluent kit, and a red test nobody ran

`fluent-card` is 107 lines of v1 and the second card in the library. It exists in
this document for two reasons: it is the cheapest of the 14 crates correction 7
identified, and it **inherited a failing test**.

**The inherited test was wrong, and had never been run.**
`tests/api.rs::card_style_resolves_from_fluent_theme` asserted
`style.border_radius == 4.0`. No design system in the table says `4.0`:
`CardTokens::from_spacing` — which is what Fluent gets — says `12.0`, and
Material's own card token says `10.0`. The literal was simply stale.

It could not have been caught earlier, and the reason is worth writing down: **the
crate did not compile before this port.** Its v1 `lib.rs` called
`Ripple::new(inner)` and `.radius_fn(..)`, both of which the ports removed, so
every run stopped at the build. The assertion sat behind a compile error for the
life of the crate. This is the same failure as the `Length` correction below — a
kit test suite that no root-workspace command reaches — except that this one was
never reachable at all.

Rewritten to assert against `theme.component(ComponentId::Card)` instead of a
literal, which is what the test's own name claims: the point is that the style
*resolves from* the theme, so a change to the token table should move the number
rather than break it.

**The manifest was wrong in a way that compiled.** `fluent-card`'s `Cargo.toml`
started as a copy of `md-card`'s with the package name changed, and
`floom-widget-runtime = { features = ["material"] }` came along with it. It built
and every test passed, because all three design-system features are *the same
gate*: `#[cfg(any(feature = "material", feature = "cupertino", feature = "fluent"))]`
appears on the same items in both `floom-widget-runtime` and `floom-core-theme`, so
enabling the wrong one satisfies it exactly as well as the right one.

Corrected to `["fluent"]`, and recorded because the failure mode is the interesting
part: **these features are presence flags, not data selectors.** Nothing in the
tree can currently tell you which design system a crate asked for, so a Fluent
crate asking for Material is undetectable by compiling or by testing — it is only
detectable by reading the manifest. That stops being true the moment the features
start selecting data instead of gating items, so the copy-a-manifest shortcut needs
its own check: **when a ported kit manifest is derived from another kit's, the
design-system feature is the field that must not be inherited.**

**A card you can press is still a card that holds its content.** This is the one
genuine shape difference from `md-card`, and it is why `Surface` had to be a
widget with children rather than a leaf. The composition is `Ripple(Surface(content))`
— not `Surface(Ripple(content))`, and not a replacement. The test that holds it
honest is a downcast: the ripple's one child must *be* a `Surface`. A
`Container`-based port, or a ripple that had displaced the box, fails there.

**Two slot facts the port turned up, both about replay rather than storage.**

* `Ripple::child` takes the child's `LayoutRequest` as well as its builder, like
  every other slot. Found by compiling, not by reading.
* The surface cannot be *captured* by the ripple's slot, because a 2.0 slot is a
  replayed `Fn` and cannot hand out the same widget twice —
  `cannot move out of surface, a captured variable in an Fn closure`. So the slot
  captures plain data (a colour, three lengths, a `Shadow`) and composes the box
  on every rebuild. `the_slot_replays_rather_than_hands_out_one_widget` replays
  twice and asserts both rounds still have their content, which is what a captured
  widget would lose.

**The radius v1 read but never used, now kept apart on purpose.** v1 read
`CardTokens::border_radius` into its style struct and drew the box at
`t.radius().xl`. For Material the two are close enough to hide this; for Fluent
they are `12.0` against `24.0`, which is the only reason the port noticed. Both
are kept — `resolved_style().border_radius` reports what v1 reported,
`corner_radius()` returns what v1 drew — and
`the_drawn_radius_is_the_themes_not_the_cards_token` asserts they differ.

**One thing the port gives up, stated rather than buried:** the ripple's radius is
resolved once, here, instead of through v1's `radius_fn(|t| t.radius().xl)`
re-read on every render. `floom_widgets::Ripple` takes a fixed radius for the same
reason it takes a fixed ink — it is a value in a builder. Box and ripple resolve
from the same theme value at the same moment, so they cannot disagree, but neither
re-reads the theme afterwards. This is a property of the whole porting approach
rather than of this crate: **every ported kit resolves the theme when it is built,
so a runtime theme switch restyles nothing until the tree is rebuilt.** Worth
deciding on purpose before 290 more crates inherit it.

### What `md-item_group` turned up — the promotion that paid for itself

209 lines of v1 became a 70-line wrapper, and the measurement that justified it is
the whole argument for promoting a component rather than porting it three times:

| | lines | substantive lines shared with the other two |
|---|---|---|
| `md-item_group` | 209 | **123 of 149** |
| `cupertino-item_group` | 208 | |
| `fluent-item_group` | 208 | |

Everything the three share — the shared selection state, the
single/multiple/mandatory click rules, the row's two appearances, the click target,
and the padding that is `pad` across and `pad / 2` down — is now
[`floom_widgets::SelectGroup`], promoted for the same reason `Accordion` was. What
is left in `md-item_group` is the part that genuinely differs per design system:
which tokens, and which colours mean "chosen". Its 11 tests are about the
translation; the state machine's 21 live in `floom-widgets`' `select_group_tests`,
which is the right boundary for them.

**The state is shared because a row's appearance depends on *other* rows.** This is
the one place where the `Arc<Mutex<Vec<bool>>>` pattern earns its keep rather than
merely following precedent: choosing row 1 has to restyle rows 0 and 2 as well, so
no row can own the answer. Each row reads the shared set in its own `update`,
compares against what it last acted on, and marks itself dirty. Four mutations,
checked rather than assumed:

| Mutation | Tests red |
|---|---|
| single mode stops clearing the other rows | 3 — the shared-state tests |
| a row's `update` stops comparing | 3 — the self-report tests |
| `selected_from(..)` reads a constant | 1 — the reader test |
| `mandatory` in multiple mode refuses at two rows | 1 — the rule test |

**A correction to this document's own reading of v1.** The first version of the
mandatory test asserted that with two rows chosen, clicking one *keeps* it. It does
not: v1 refuses to remove a row only when it is the **only** chosen one
(`cur.len() == 1`). So `mandatory` keeps the selection non-empty; it does not
freeze it. The implementation was right and the test was wrong — but only because
the test disagreed loudly. Recorded because getting this backwards is invisible
until a user tries to deselect one of two chosen rows and nothing happens.

**`Pressable::selected_from` is the missing half of `expanded_from`.** The row has
to report `SELECTED` through a *reader*, because a row is not rebuilt when the
selection moves: a captured value would go stale the instant another row was
chosen, and would be the opposite of what the user sees. `SemanticsState::SELECTED`
already existed and had no way to be set.

**Two additions, both deliberate and both called out rather than slipped in:**

* **Semantics.** v1's rows were a `Container` with a click handler and a pointer
  cursor — invisible to a screen reader, which could neither name a row nor learn
  which was chosen. Rows here are `SemanticsRole::Radio` in single mode and
  `CheckBox` in multiple mode. That is an addition, not a translation.
* **`Pressable` where v1 had a raw listener.** The pointer cursor is *not* carried
  over: 2.0's `PaintContext` has no cursor vocabulary. The row is still keyboard
  activatable, which the pointer was not.

**What could not be carried over, recorded rather than worked around:** v1 wrote
`apply_if(n == 0, |s| s.hide())` — an empty group hides its *node*. 2.0's
`LayoutRequest` carries no visibility, and a hidden node and an absent one differ
in layout: one takes no space, the other takes whatever the parent gives it. An
empty group here is a stack with no children.

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
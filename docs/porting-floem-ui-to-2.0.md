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
| surface + press | `cupertino-card` | ported, 14 tests (same shape, `radius().md`) |
| icon button | `md-icon_button` + 2 kits | ported, 43 tests (shape promoted to the engine) |
| list row | `md-list_item` + 2 kits | ported, 48 tests (shape promoted; needed a new capability) |
| alert banner | `md-alert` | ported, 12 tests (**needed nothing** from the engine) |
| tree view | `md-treeview` + 2 kits | ported, 53 tests (shared expansion state promoted) |
| text input | `floom_widgets::TextInput` | **new primitive**, 32 tests — nothing in 2.0 had one |
| popup placement | `PopupPlacement` + `OverlayHost` | the geometry and the embeddable host — 24 tests |
| combobox | `floom_widgets::Combobox` | decision layer (24 tests) + widget (20 tests) |
| combobox (Material) | `md-combobox` | ported, 14 tests — 91% overlap, so it promoted |
| icon tables | `cupertino-icon`, `fluent-icon` | 2.0 surface **added** to two still-v1 crates, 11 tests each |
| combobox (Cupertino, Fluent) | `cupertino-combobox`, `fluent-combobox` | ported, 15 tests each — **no panel gap**, against Material's 4.0 |

Ported kits: `md-card`, `md-button`, `md-icon`, `md-expansion_panels`,
`md-toolbar`, `md-footer`, `md-item_group`, **`fluent-card`**,
**`cupertino-card`, the three `icon_button`s** and **the three `list_item`s**.
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
2. ~~**`cupertino-card`**~~ — **done**, 14 tests. The floem-ui workspace is now at
   **22 errors across 12 crates** (26/14 → 24/13 → 22/12, each step minus exactly
   the two `Ripple` errors of the card just ported).
3. ~~**The three `icon_button`s**~~ — **done**, 43 tests. The shape was promoted to
   the engine rather than ported three times; see the section below for the
   measurement that decided it and the four mutations run against it.
4. ~~**The three `list_item`s**~~ — **done**, 48 tests.
5. **A decision before `dialog`.** Correction 11: a v1 dialog owns its `open`
   signal and a 2.0 one cannot, because `RwSignal` is `Rc`-based and a slot closure
   must be `Send + Sync`. The host already owns presence (`OverlayKind`,
   `OverlayHandle`, `PresenceChannel`), so the answer is "the host owns it" — but that
   touches the overlay API and 285 crates will inherit the answer. Everything else
   about `dialog` was checked and is *not* blocked: the scale animation exists
   (`CompositorProp::Scale`), and clicks-inside-already-do-not-dismiss falls out of
   2.0's outward-from-the-hit dispatch. `max_width`, which it did need, is now in.

6. **Then the last three — in dependency order, not crate order** (correction 12),
   and then `floom-elements`. The floem-ui workspace is now at **4 errors across 4
   crates** — `md-autocomplete`, `md-combobox`, `md-dialog`, `md-select` — so
   `floom-elements` needs all four, and nothing else:

   | | what blocks it |
   |---|---|
   | ~~**`treeview`**~~ | **done**, 53 tests. The state is the engine's `floom_widgets::TreeView` and each kit is a token translation. |
   | ~~a **text-input primitive**~~ | **done** — `floom_widgets::TextInput`, 32 tests. `combobox` and `autocomplete` can now start, and each needs a filtered popup on top of it. |
   | `combobox`, `autocomplete` | that primitive; 93% overlap, so whichever is second is cheaper |
   | the **dialog decision** | correction 11 — who owns a dialog's `open` state |
   | `select` | a popup widget *and* an answer about three kits that disagree (60% overlap) |

   Correction 7's two experiments still stand for all of them: `.view()` cannot simply
   be dropped, because the callers are v1 crates and a 2.0 widget has no v1 view to
   return.

   Measured, so the order is chosen rather than guessed:

   | component | cross-kit overlap | md lines | what it will need |
   |---|---|---|---|
   | `alert` | 83% | 170 | **nothing** — done |
   | `dialog` | 90% | 271 | absolute positioning, a scrim, two slots |
   | `treeview` | 87% | 259 | shared expansion state (as `SelectGroup` had) + a viewport |
   | `select` | **60%** | 263 | a popup anchor — and the kits genuinely differ |
   | `combobox` | 93% | 284 | a text input + a filtered list popup |
   | `autocomplete` | 93% | 315 | the same, plus focus |

   `treeview` next once the dialog question is settled: 87% overlap, and its state
   (which nodes are expanded) is shared across rows exactly as `SelectGroup`'s was,
   so it wants the same promotion. `select` last despite being no larger, because at
   60% it is the one component where the three kits are **not** the same component.
   With them the floem-ui workspace goes green, at which point `floom-elements`'
   own 6,220 lines of v1 `multi` dispatch can be rewritten (correction 6).

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
    for c in fluent/fluent-card fluent/fluent-icon_button fluent/fluent-list_item \
             cupertino/cupertino-card cupertino/cupertino-icon_button \
             cupertino/cupertino-list_item \
             material/md-icon_button material/md-list_item; do
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

13. **The first *new* widget in the port is a text input, and two of its four
    obligations had nowhere to go.**

    `combobox` and `autocomplete` both build on `TextInput::new(query)`, and 2.0 had
    no text input at all — so this is the first primitive here that is not a port of
    something, and its shape is worth stating rather than leaving in the diff.

    **The buffer is `Arc<Mutex<String>>`, not a signal.** v1's
    `TextInput::new(buffer: RwSignal<String>)` had the *caller* own the buffer, which
    cannot cross a `Send + Sync` slot boundary — the same wall `md-dialog` hits
    (correction 11), and for the same reason. An `Arc<Mutex<String>>` is `Send + Sync`
    and shared, so a caller still holds a handle and can read or write it. The
    consequence is that the **external write** path becomes a first-class case: v1 got
    it free from the signal, and here the widget has to notice for itself by comparing
    against what it last read on every frame. Two of the six obligations that adds are
    `on_change` firing for both directions, and the caret being clamped when an external
    write shrinks the buffer under it.

    **Two obligations had nowhere to go, and are recorded rather than faked:**

    * **A click cannot place the caret.** `EventContext` carries `cursor_position` and
      `bounds` but no text measurement, so an x coordinate cannot become a byte offset.
      A click focuses and puts the caret at the end. Everything else — typing,
      backspace, delete, arrows, Home, End, Enter, Escape, IME — works from the caret's
      current position.
    * **The text is not clipped to the box.** `PaintContext` has `stroke_path` but no
      `stroke_rect` and no clip op — *although the host already has the scene
      operation* (`write_clip_push`, and `resolve_clip` in `floom-types`). So this is
      a missing method rather than a missing capability, which makes it worth doing
      before a text input grows selection handles or scrolls.

    Both are the kind of limit that should be fixed before the widget is built on
    further, and neither is worked around: a field narrower than its text paints past
    its own edge, and that is visible.

13b. **The popup was already there. I wrote a second one, and it was worse.**

    `combobox`, `autocomplete` and `select` all need a panel below a control, and 2.0
    has: `OverlayRequest`, `OverlayHandle`, `OverlayKind::Popup`, the state-driven
    `UpdateContext::open_overlay`/`close_overlay` (whose doc names a `Dialog` as its
    intended caller), `Widget::overlay_opened`/`overlay_closed`, and in the host
    `dismiss_top_overlay_on_press` — the outside-click rule, correctly a *host* rule.
    `floem-app/tests/overlay.rs` covers all of it against the real host.

    What is missing is the geometry: a `Popup` asks for **its own box** and fills it,
    which is right for a menu pinned to a toolbar button and wrong for every dropdown,
    because a combobox's list belongs *below* its field.

    I did not find that first. I wrote a new `floom_widgets::Popup` — 320 lines, a
    three-state `open`/`requested`/`handle` machine, `panel_rect`, the whole state
    machine — and it was **worse than what it duplicated** in the one way that
    mattered: on a host that refuses the request, `OverlaySurface` falls back to
    mounting the panel in-tree, while mine cleared `open` and showed nothing at all.
    285 lines of duplicate machinery and a regression, found by grepping the module I
    was extending *before* checking what it already did.

    The real gap turned out to be one enum and one function: `PopupPlacement`
    (`Fill` / `Below` / `Above`) and `popup_placement_rect`. `Fill` is the default,
    so every existing popup is unchanged — which is the control group that proves the
    new field changed nothing.

    Two things survived from the attempt and are worth having:

    * **`RecordingUpdateContext` had no overlay door.** `open_overlay` is *defaulted*
      on `UpdateContext`, so a composite that mounts its panel from `update` — which
      is what a popup, a dialog and a drawer all do — was untestable: the door
      returned `false` and a test could not tell a deliberate fallback from a missing
      implementation. It now records `OverlayCall`s and runs the content factory
      exactly as the host does. The placement tests drive a real `Popup` through it.
    * **`bounds()` was unsettable.** `UpdateContext::bounds` is documented as the
      *previous* frame's box and as "exactly right for anchoring an overlay to a
      control that already exists"; without `with_bounds` a test could not check that
      a popup put itself below its trigger.

13c. **A popup cannot be a child, and the reason is a host fact I had to go read.**
    `combobox` cannot hand its `view()` a `Popup` and flip its presence — and the
    first reason I reached for ("a child whose presence flag changes must change
    key") is only the symptom. The cause is in the host: **`OverlaySlot` is removed
    only by `close_overlay`**, so *unmounting a node does not close the overlays it
    opened*. A popup rebuilt with a new key remounts, and the panel it had open is
    orphaned on the overlay stack with nothing left able to name it.

    So the owner keeps the overlay bookkeeping as a **field**, flips `present` on it,
    and forwards `overlay_opened`/`overlay_closed`. Which means
    `compose::OverlaySurface` — previously `pub(crate)`, correct while `Popup` and
    `Dialog` *were* the composite — has to become the public `OverlayHost`, and it
    has to hand out the in-tree fallback it was writing out for itself.

    Two test doubles came with it, and one of them had a bug worth recording:

    * **`RecordingUpdateContext` refused every overlay**, because
      `#[derive(Default)]` set `overlays_supported` to `bool::default()` — `false`.
      So a context built the obvious way silently shut the door, and the first
      symptom was a state-machine test reporting that nothing had been opened, which
      read like a bug in the widget. It now has a hand-written `Default` in which the
      door is **open**, and `without_overlay_support()` for the headless embedder.
      Without that second constructor the refusal branch would have been unreachable
      in tests, which is how a fallback rots.
    * The first draft of `nothing_to_show_mounts_nothing_anywhere` asserted that a
      `build` returning `None` leaves the in-tree fallback *unarmed*. That is wrong,
      and the code is right: a refusal is a refusal whatever caused it, so the
      fallback **is** armed — the panel is absent because there was no content. The
      test now says both halves, because "no content" and "the fallback declined" are
      different statements.

    And one test of mine was simply wrong about the contract: `closing_closes_the_
    handle_the_host_issued` forgot that the host answers *later*, through
    `overlay_opened`. Without that call the widget has no handle to name, so there
    was nothing to close. That is not a nit — it is the reason the grant is a
    separate callback and not a return value.

13d. **`md-combobox` is ported — and the other two are blocked on a crate that is not
    a component at all.**

    Material's combobox is a 14-test token translation, exactly as the overlap argued
    it would be. The port removed: three `RwSignal`s (v1's `selected`, `open`, `query`)
    in favour of the engine's shared state; v1's `Overlay::new(panel)` plus a
    `LayoutChangedListener` reading the trigger's window origin and height, in favour
    of `OverlayHost` with `PopupPlacement::Below` — which is the same arithmetic said
    the short way, and the listener is gone because the widget already has the box
    whose bottom edge it needs; v1's `List` of `AnyView`s; and v1's pointer cursor, for
    the fifth time in this port.

    Two losses are recorded rather than faked:

    * **v1's row hover fill has no 2.0 equivalent.** `Pressable`'s interaction state is
      not readable from outside, so a row paints its selected fill or nothing. The
      alternative — a fill that is always on — would be a permanent hover.
    * **The list opens on the first keystroke, not on focus**, because 2.0 has no way
      for a parent to observe a child's focus.

    Then Cupertino and Fluent turned out to need `cupertino_icon_svg` /
    `fluent_icon_svg` for their chevron, and **both live in `cupertino-icon` /
    `fluent-icon`, which are still v1 crates** — `floom = { workspace = true }`. So the
    blocker for the other two comboboxes is not a component but an *unported
    dependency*, which is a category this plan had not tracked.

    The good news is that those crates are nearly free to port: `icons.rs` is a 45-line
    pure data table with no v1 dependency at all — a `match` of SVG path strings — and
    only `lib.rs` (the v1 view wrapper, 93 lines) needs dropping. What the kit then
    needs is `VectorIcon::new(parse_path_data(path), 0,0,24,24)`, both of which the
    engine already has.

13e. **I turned a crate into a 2.0-only crate and broke five crates that were not part
    of the port.**

    Porting `cupertino-icon` and `fluent-icon` looked trivial — `icons.rs` is a 45-line
    pure data table with no v1 dependency — so I *replaced* each `lib.rs` with a 2.0
    version that dropped the v1 widget. The floem-ui workspace went from **three error
    crates to eight**, because `cupertino-alert`, `cupertino-dialog`, `fluent-alert`,
    `fluent-dialog` and `fluent-stepper` all call `CupertinoIcon::new(..).view()` and
    are themselves still v1.

    The mistake was reading "this crate is being ported" as "this crate's consumers have
    been ported". They have not — the port proceeds one shape at a time and the whole v1
    tree is still underneath. The rule says *never reintroduce a v1 `floom` dependency
    into a **ported** crate*; it does not say *strip v1 out of a crate that is still
    v1*. So the fix is to **add** a 2.0 surface and leave the v1 one alone, which is
    what the two crates do now: `cupertino_icon(name) -> VectorIcon`, plus a public
    `*_icon_path` and `VIEW_BOX`, alongside the untouched v1 widget.

    Worth noting what the added surface deliberately does **not** do: it resolves no
    theme. v1's `CupertinoIcon` produced a `resolved_style()` from a `MultiTheme`; the
    2.0 icon is a widget the caller holds and restyles with `set_color`/`set_size`, and a
    caller wanting the design system's `on_surface` reads it from the theme itself. So
    v1's `tests/api.rs` had nothing left to assert, and it was rewritten to say *why*
    rather than deleted.

13f. **Two of the three comboboxes had no gap, and copying a manifest would have given
    them Material's.**

    The kit-to-kit copy is where this plan has already been bitten twice, and the
    combobox was no different — twice:

    * `cupertino-combobox`'s manifest was generated from `md-combobox`'s and came out
      asking for `features = ["material"]`. It compiled. That is correction 8 exactly:
      the three design-system features are the *same* gate
      (`#[cfg(any(feature = "material", ...))]`), so nothing but reading the manifest
      catches a kit asking for the wrong one.
    * `fluent-combobox` was *derived* from `cupertino-combobox` by explicit
      substitution, and the diff came back showing only names, the icon crate and the
      design system. But the **manifest had been written by a separate pass**, so it kept
      `features = ["cupertino"]` — and the diff of the manifests did not show it, because
      I diffed the two manifests *after* substituting and they now differed only in the
      lines I had changed. The wrong feature was invisible to the check that was supposed
      to catch exactly that.

      The fix that actually works: **grep the manifests for the feature after deriving**,
      not diff them. A diff shows what differs between two files; the question here was
      whether a value was right, and those are different questions.

    Both were caught before committing, by reading the manifest rather than trusting the
    derivation. The derivation itself was sound — 20 changed lines, every one of them a
    name, an icon crate, a design system, or a doc sentence — and one claim in the copied
    prose needed checking rather than assuming: the note says `fluent-treeview`'s v1
    source genuinely used `MdIcon` (unlike `fluent-combobox`), and it does.

    **The disagreement itself is the substance.** Material wrote `.margin_top(4.0)`;
    Cupertino and Fluent wrote nothing, so their panels sit flush against the field.
    `ComboboxStyle::panel_gap` is a parameter and the engine default is Material's 4.0 —
    which means *a kit that says nothing gets the wrong gap*, and the kits that want 0.0
    have to say so out loud. Three tests per kit pin that, and mutating `0.0` to `4.0`
    turns three of them red.

    Also derived, deliberately: **the kits' call-site constants are identical.** The
    26-line diff between the three v1 crates gives the impression that they disagree
    about everything, and they do not — they agree on `gap(8.0)`, `padding_horiz(12.0)`,
    `size(14.0)` and all seven alphas, and differ on exactly one number. A test says so,
    because "these two design systems agree here" is the surprising claim and deserves
    the same protection as the disagreement.

12. **The last four components are not four ports; they are four prerequisites,
    and only one of them is self-contained.**

    Measured while assessing `md-dialog`, and worth stating as its own finding
    because the plan has been counting *crates* rather than ordering *work*:

    | component | overlap | what it actually needs first |
    |---|---|---|
    | `treeview` | 87% | a new `floom_widgets::TreeView` — shared expansion state, row flattening, hover. **Self-contained: no host change.** |
    | `dialog` | 90% | the open-state decision (correction 11). Its *layout* needs are already met: `OverlayKind::Dialog` exists in `floom-types`, `CompositorProp::Scale` covers the pop-in, 2.0's outward-from-the-hit dispatch covers click-inside, and `max_width` landed with correction 11. |
    | `select` | 60% | a **popup widget**. `OverlayKind::Popup` exists as a type, but no widget uses it — there is nothing to anchor to. And at 60% the three kits are not the same component, so it is a decision even once the popup exists. |
    | `combobox`, `autocomplete` | 93%, 93% | **a text-input primitive that does not exist.** `ImeUpdate` and `InputMode` are in `floom-types` and the host handles them, but there is no `TextInput` widget in `floom-widgets`. Both crates build on `TextInput::new(query)` in v1, so neither can start. |

    Two consequences the crate count hid:

    * **`combobox` and `autocomplete` cannot be ported in either order relative to
      each other** — both need the same missing primitive, and neither is a
      prerequisite for the other. Whichever is done second is cheaper.
    * **`select` is the only one whose blocker is *also* its own design question.**
      The other three are blocked by something external (a decision, a missing
      primitive, a missing widget), which is the good kind of blocked: someone else
      can decide it. `select` needs a popup *and* an answer about how three kits that
      disagree should each present one.

    So the honest order is `treeview` (self-contained), then the text-input primitive,
    then `combobox`/`autocomplete` (cheapest whichever way round), then the dialog
    decision, then `select`. **`floom-elements` still gates on all four**, because it
    re-exports them — which is why it stays last and why the floem-ui workspace
    cannot go green before them.

11. **A v1 dialog owns its own open state, and a 2.0 one cannot.** `md-dialog` is the
    first component in this batch whose blocker is not a missing capability but a
    modelling difference, so it is assessed here rather than half-ported.

    v1's dialog holds `open: RwSignal<bool>` and closes by `open.set(false)` from its
    own click handlers. In 2.0 a slot builder is `Fn() -> Box<dyn WidgetAny> +
    Send + Sync`, and `RwSignal<T, S = UnsyncStorage>` is `Rc`-based — so it **cannot
    be captured by a slot closure**, which is the same fact that makes
    `Pressable::on_click` non-`Send`. A 2.0 dialog therefore cannot own its own
    visibility: the host owns presence (`OverlayKind`, `OverlayHandle`,
    `PresenceChannel` all live in `floom-core` and `floem-app`), the widget reports
    "I was dismissed" through a `Send + Sync` callback, and something above has to
    re-render it.

    That is a design decision about who owns state, it touches the host's overlay
    API, and 285 unported crates will inherit the answer. It is not a port and should
    not be decided inside one.

    **What the assessment did settle**, by reading rather than guessing:

    * **The scale animation is *not* a gap.** v1 keyframes `scale(95.pct())` to
      `scale(100.pct())`, and `CompositorProp::Scale(f64)` already exists as an
      `AnimationSpec` channel. `MOTION-GAP.md`'s B4 is the *height* transition, which
      this component does not use.
    * **"Clicks inside the card must not dismiss" is *not* a gap either.** 2.0's
      dispatch builds a route from the hit node outward and stops at the first widget
      that consumes, so anything inside the card that consumes the press already
      prevents the scrim from seeing it. v1 needed an explicit
      `EventPropagation::Stop` because v1 had no such chain. The idiom here would be
      a `Pressable` with no callback — it consumes, and publishes none — which is
      slightly odd but needs no engine change.
    * **`max_width` *was* a gap**, and is now closed (see below). v1's card is
      `width_pct(100.0).max_width(..)`, and with only `width` one of the two had to
      be dropped.

10. **Correction 9 has a consequence no ported crate can currently test for, and
    the attempt to test for it produced a control group that could not fail.**

    The three `icon_button` suites each end with a test reading the crate's own
    design system's colours. Two of those tests are *incapable* of failing for the
    reason they were written: they read the global design system, which the test
    itself sets on the previous line, while the crate's `new()` seeds a
    `MultiTheme` that — per correction 9 — ignores its own argument.

    Measured, not argued: changing `DesignSystem::Fluent` to `DesignSystem::Material`
    in `fluent-icon_button/src/lib.rs` leaves **all 15 of its tests green**.

    So the annotation now in those two files says what is and is not established:

    * **is** pinned — the crate's translations read `on_surface_variant` and
      `primary` from whatever theme is active;
    * **is not** pinned — that the crate activates *its own* kit.

    Nothing can pin the second thing today, because `MultiTheme` does not carry a
    design system: it holds all three and dispatches on a global at every read. The
    engine fix is small and belongs before the remaining crates — give `MultiTheme`
    a design system (or make `new_for` actually call `activate`) — but it changes
    every kit at once, so it is a decision rather than a port.

    **The general lesson, which is the one worth keeping:** a control group has to be
    checked by mutating the thing it is a control *for*. Mine asserted a property
    that no mutation could disturb, so it was decoration. This document has now been
    wrong twice in the same way — correction 8 assumed a stale assertion would be
    caught, and this assumed a control group would fail — and both times the failure
    was a test that could not fail rather than a test that failed.

9. **Every kit was resolving Material's tokens, and one inherited test passed for
   exactly that reason.** This is the largest correction in the document, and it was
   found only because `cupertino-card`'s inherited test disagreed with
   `fluent-card`'s about the same field.

   `MultiTheme::new_for(ds)` **does not select `ds`.** `activate` is a documented
   no-op placeholder, and `MultiTheme` holds all three themes and dispatches on
   `current_design_system()` — a **process-global** — at *every read*:

   ```rust
   pub fn activate(self, _ds: DesignSystem) -> Self { self }   // the argument is unused
   fn ds(&self) -> DesignSystem { current_design_system() }
   ```

   `floom_core_theme` therefore has exactly one design system in effect per thread,
   defaulting to Material, and **nothing in this 299-crate workspace ever calls
   `set_design_system`** — the only two definitions are the function itself and its
   re-export in `floom-widget-runtime`. So `MultiTheme::new_for(Cupertino)` answers
   with Material.

   Measured, by activating each in turn:

   | kit | `radius().md` | `radius().xl` | card token |
   |---|---|---|---|
   | Material | 12.0 | 24.0 | 12.0 |
   | Cupertino | 10.0 | 20.0 | 10.0 |
   | Fluent | 4.0 | 8.0 | 8.0 |

   **The two inherited card tests, and what they prove.** `fluent-card`'s asserted
   `border_radius == 4.0` and **failed** — `4.0` is nobody's. `cupertino-card`'s
   asserted `12.0` and **passed** — because `12.0` is *Material's*. Two tests, one
   field, two different beliefs about their own kit's radius, and the one that was
   wrong in the direction of the default went green. A passing test that is right for
   the wrong reason is the failure mode correction 8 did not anticipate: 8 assumed a
   stale assertion would be *caught*, and here it was not, because the bug supplied
   the wrong number.

   **What this document got wrong as a result, all of it mine, one commit ago.**
   `fluent-card`'s port claimed "Fluent's two radii differ: token 12.0, drawn 24.0,
   which is the only reason the port noticed" and built a test asserting they differ.
   Those are Material's numbers, read while believing they were Fluent's. Under real
   Fluent both are `8.0`. The test passed *because* of the bug it was written to
   document, and it has been rewritten to assert equality.

   The follow-on claim was worse: that Material was "the kit where the token and the
   drawn radius disagree". It is not. v1's `md-card` draws at
   `tokens.border_radius` — the token — so Material agrees with itself. All three
   kits land on the same number by three *different* expressions (Material reads the
   token, Cupertino reads `radius().md`, Fluent reads `radius().xl`), and the
   agreement is a fact about today's token table rather than something a port may
   rely on. Hence the control: `the_drawn_radius_follows_the_theme_rather_than_a_literal`
   switches the global to Material and would go red against a hard-coded radius.
   Verified by mutation — replacing `corner_radius()`'s body with `10.0` turns that
   one test red and leaves the other eleven green.

   **Blast radius, measured rather than assumed.** All seven previously ported kits
   were checked for hard-coded design-system numbers: **zero**. Their tests compare a
   widget against a token read from the same theme, which is correct under any design
   system, so Material being the default did not mislead them. The exposure is
   confined to tests that assert a *literal* — the two inherited `api.rs` files and
   the ones written this session, all now fixed. The seven kits' claims of the form
   "resolves from its own theme" are nonetheless now known to be true by construction
   rather than by measurement, and each should activate its design system when it is
   next touched.

   **A `MultiTheme` is not a snapshot.** The pin that documents this got the ordering
   wrong first: reading a theme built while the global said Material, *after*
   switching the global to Fluent, returns Fluent's numbers. The switch has to happen
   before the read. Two of this document's own test helpers also fought each other —
   `card()` re-activated the kit on every call, so a control test that switched to
   Material had its switch undone one line later and failed against a correct
   implementation. One helper, `card_under(ds)`, now takes the design system as an
   argument for that reason.

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
`t.radius().xl`. Both are kept — `resolved_style().border_radius` reports what v1
reported, `corner_radius()` returns what v1 drew.

*This paragraph was wrong and correction 9 fixes it.* It claimed the two differ for
Fluent (`12.0` against `24.0`) and that this was how the port noticed. Those are
Material's numbers, read while the global design system was Material. Under real
Fluent both are `8.0`, and the test written to that claim was asserting a difference
that does not exist — passing only because of the bug. It now asserts equality, and
the reason the two are still separate methods is that **Material** is the kit that
would break if they were merged into one.

**One thing the port gives up, stated rather than buried:** the ripple's radius is
resolved once, here, instead of through v1's `radius_fn(|t| t.radius().xl)`
re-read on every render. `floom_widgets::Ripple` takes a fixed radius for the same
reason it takes a fixed ink — it is a value in a builder. Box and ripple resolve
from the same theme value at the same moment, so they cannot disagree, but neither
re-reads the theme afterwards. This is a property of the whole porting approach
rather than of this crate: **every ported kit resolves the theme when it is built,
so a runtime theme switch restyles nothing until the tree is rebuilt.** Worth
deciding on purpose before 290 more crates inherit it.

### What `cupertino-card` turned up — the same shape, and the test that agreed with the bug

`cupertino-card` is 109 lines of v1 and the third card in the library. As a *port* it
is nearly free: `fluent-card` had already established the composition
`Ripple(Surface(content))`, and the only difference is the radius — Cupertino draws
at `t.radius().md` where Fluent draws at `radius().xl`. That is the whole change,
and it is why a shape that appears in three kits is worth porting once.

**It is here because of what it proved about the other two.** Cupertino's inherited
`tests/api.rs` asserted `style.border_radius == 12.0` and **passed**. Cupertino's card
token is `10.0`; `12.0` is Material's. Its sibling in `fluent-card` asserted `4.0`
and failed, because `4.0` is nobody's. So the two inherited card tests held two
different beliefs about the same field of the same shape, and the one that was wrong
*in the direction of the default* went green — which is what correction 9 is about.

**Three kits, one shape, three different ways to name the radius.** This is the part
worth keeping:

| kit | v1 drew the radius from | value | card token | agree? |
|---|---|---|---|---|
| Material | `tokens.border_radius` | 12.0 | 12.0 | yes |
| Cupertino | `t.radius().md` | 10.0 | 10.0 | yes |
| Fluent | `t.radius().xl` | 8.0 | 8.0 | yes |

All three agree, and that agreement is a coincidence of today's token table rather
than a property any of them relies on. Two of the three read the *radius scale* and
one reads the *token*, and a port that unified them would break two kits the moment a
radius scale moved. So `corner_radius()` is a method that reads the scale, and
`resolved_style().border_radius` is a field that reports the token, in all three
crates — and the control test proves the method is a read rather than a literal:

    the_drawn_radius_follows_the_theme_rather_than_a_literal

Verified by mutation: replacing `corner_radius()`'s body with `10.0` turns that one
test red and leaves the other eleven green.

**`BLACK` is a shared token, not a per-crate constant.** `fluent-card`'s port defined
its own `const BLACK` and this one inherited the mistake from the copy; v1 got it
from the glob `use floom_core_theme::tokens::color_ext::*`, where it is already
`pub const BLACK: Color`. Both now import it. Worth a line because a locally
redefined token is invisible: it compiles, it is almost certainly the same value, and
nothing in the type system says two crates must agree.

**One thing this port did not decide.** Cupertino's card, like Fluent's, resolves its
theme when it is built. Combined with correction 9 — a theme resolves the *active*
design system at every read, from a global — a card that is built once will follow a
later `set_design_system` for its colours but not for its radius, padding or shadow,
because those were resolved into plain data at build time. That is the general
build-time-resolution limitation already noted, now with a sharper edge: it is not
only "a theme switch restyles nothing", it is "a theme switch restyles a kit card
*partially*", which is worse than either consistent behaviour. It wants a decision
before 290 more crates inherit it.

### What the `icon_button` turned up — the promotion that needed three engine additions

Three crates, 118/117/117 lines of v1, and the measurement that settled the
approach: **69 of 73 substantive lines are identical across all three**, and the
four that differ are the type name, the `DesignSystem` handed to `MultiTheme`, and
whether the theme came from `theme_signal()` or a fresh `MultiTheme`.

That is a *higher* overlap than `SelectGroup`'s 123 of 149, and the same reasoning
applies — port it once. So the shape became `floom_widgets::IconButton` and each kit
crate is now ~40 lines of token translation. What the promotion took is worth
spelling out, because it is not "most of it" but **all of the decisions**:

* the composition and its *order*: `Ripple(Pressable(Surface(icon)))`;
* the disabled rule, which replaces the variant's fill **and** outline with two
  different alphas of `on_surface` (12 for the fill, 38 for the outline and glyph);
* the ripple-ink rule — `on_primary` for contained, `on_surface` for the other two;
* that the glyph's colour follows `disabled` while its *size* does not;
* and that `disabled` gates the click in **one** place. v1 wrote
  `if !disabled { .. }` inside the action closure, which guarded the pointer but
  not the keyboard; 2.0's gate is in `Pressable`, which guards both. That is an
  improvement rather than a translation, and the engine suite is what keeps it.

**A 2.0 slot is replayed once per key, so the icon could not be a built value.**
This is the finding that forced the node. v1 built the icon inside the button's
style closure; in 2.0, a subtree built from a `Child` builder keeps the widget it
first built, so toggling `disabled` would repaint the box and leave the glyph stale
— and a test that looked only at the box would pass. Hence `IconButtonIcon`: a node
with its own `update`, which is what needed the engine additions below.

**Three engine additions, each found by composing rather than by reading.**

* `VectorIcon::set_color` / `set_size`. Every widget here that owns a decoration has
  the pair (`Surface::set_background`, `Ripple::set_ink`, `Text::set_weight`), and
  the reasoning is theirs: a theme signal fires on unrelated changes too, so a
  setter that always bumped would repaint the tree on every one. Without these the
  icon is frozen at build time — the ninth capability gap, and the first one a
  *promotion* rather than a port turned up.
* `Ripple::ink_color()`, a reader for the same reason `clip_radius()` exists.
* **`Ripple` was never registered as a `WidgetAny`.** It could be constructed,
  returned from a public method, and then not mounted: `Box::new(Ripple::new()..)`
  failed with *"`Ripple` is not a Floom 2.0 `Widget`"*. Nobody had tried to mount
  one, because `md-button` is the only kit that used the ripple and it composes it
  by hand. Found on the first attempt to build the icon button's subtree. A public
  widget that cannot be mounted is a gap the type system does not report.

**Four mutations, because this shape has three orderings that all compile.**

| Mutation | Tests red |
|---|---|
| the icon stops restyling itself in `update` | 1 — `the_icon_is_a_node_so_a_disabled_toggle_can_restyle_it` |
| `disabled` stops replacing the variant's paint | 1 — `disabled_replaces_the_variants_paint_rather_than_blending` |
| the `Pressable` is dropped from between ripple and surface | **0 — the compiler caught it** (`E0507`: cannot move out of a captured variable in an `Fn` closure) |
| the ripple and the pressable are swapped | **6** |

The third row is the honest one: that mutation *did not compile*, so the type
system is what holds that invariant, not the test. Recorded because a suite that
claims to check an ordering should say which orderings it can actually check.

**One control group that could not fail, and was left in place with that said
out loud.** `the_crate_really_reads_fluent_tokens` reads the *global* design system,
which the test sets itself one line above — so it passes no matter what the crate's
`new()` seeds its signal with. Mutating `DesignSystem::Fluent` to `Material` in
`fluent-icon_button` leaves all 15 tests green. That is correction 9 eating a
control group written to catch a different bug, and a test that cannot fail is
worse than none because it reads as coverage. So it stays, with a doc comment
stating exactly what it does and does not establish: it pins that the crate's
translations read `on_surface_variant` and `primary` from the active theme; it
**cannot** pin that the crate activates its own kit, because `MultiTheme::new_for`
ignores its argument and nothing in `floom-core-theme` lets a theme carry its design
system. **A kit can be wired to the wrong theme and no assertion in any ported
crate will notice.** That is the sharpest form of correction 9's consequence, and
it wants an engine fix before the remaining 285 crates inherit it.

**One thing recorded rather than fixed.** `IconButtonProps::on_click` is
`Option<Box<dyn Fn()>>` — a v1 type, shared by all three kits — and the engine's
slot requires `Send + Sync`. So `MdIconButton` stores the callback **twice**: the
v1-shaped `Box` so `props()` reports what v1 reported, and an `Arc<dyn Fn() +
Send + Sync>` for the engine, from one closure. It is the last v1-shaped field in
the crate and a duplicate of something already stored; the honest fix is to change
the shared props type, which is a three-kit change and therefore its own decision.

### What the `list_item` turned up — a capability the shape needed, and an inconsistency it preserved

90 of 107 substantive lines are identical across the three v1 `list_item`s, so the
row's structure went to the engine as `floom_widgets::ListItemRow`. What the port
turned up is one **missing capability** and one **deliberate refusal to fix a bug**.

**The tenth capability gap: a minimum size.** v1 wrote `.min_height(..)`. 2.0's
`LayoutRequest` had `width` and `height`, both of which are *definite* sizes — a row
that must be **at least** a touch target tall but may grow with its content has
nothing to say, because `height` is the content's height now. The engine had no
minimum anywhere: not in `LayoutRequest`, not in `LayoutStyle`, and taffy's
`min_size` was never populated.

So `min_width` / `min_height` were added, and the interesting part is proving they
are a **floor and not a ceiling** — which is the whole reason they are separate
fields. `floom-app/tests/min_size.rs` closes the loop through a real host:

| Mutation | Tests red |
|---|---|
| a widget that asks only for a floor is *sized* to its floor | **3** — including `a_min_height_is_a_floor_and_not_a_ceiling` |

Two of the nine tests in that suite had to be rewritten before they could fail at
all, and both failures were mine rather than the engine's:

* **the control was mounted as a root.** A root is force-filled to the viewport and
  cannot shrink, so "without a floor the box hugs its content" asserted `400 < 180`
  against a 400pt-wide box. It has to be a *child* of a non-stretching stack — the
  same two-shape trick `surface_padding.rs` already uses for the same reason.
* **the "mount style wins" test pinned the style on a wrapper.** The style applied
  to the wrapper, the surface was its child with a request of its own, and nothing
  was ever in conflict. The box came out 180pt and the failure message read "the
  mount style's 30pt stands: 180 wide" — a sentence that says the opposite of what
  it means.

**The inconsistency this port refused to fix.** Two kits read their own token table
for the row's height and font sizes; the third does not:

| | `min_height` | `title_font_size` | `subtitle_font_size` |
|---|---|---|---|
| Material | `tokens.*` | `tokens.*` | `tokens.*` |
| Cupertino | **44.0** | **17.0** | **15.0** |
| Fluent | **40.0** | **14.0** | **12.0** |

`ListItemTokens::default()` says `56.0 / 16.0 / 14.0` for **every** kit, so Cupertino
and Fluent disagree with tokens they have, and the disagreement is real: Cupertino's
rows are 44pt tall and the table says 56.

A port that "helpfully" routed all three through the token table would change
Cupertino's rows from 44pt to 56pt tall — a design change wearing a refactor's
clothes. So each kit passes its own numbers, and the disagreement is pinned by a test
whose doc comment says it **must fail** if someone later makes the kits consistent.
Verified by mutation: replacing the literals with token reads turns 2 tests red in
each of the two suites.

**A third `FontWeight`, and a silent behaviour change.** v1's
`typography().title.medium.font_weight.to_floom()` returns a
**`parley::style::FontWeight`** — a third type with the same name as
`floom_types::FontWeight`, which is what 2.0's `Text::weight` takes. The two have to
meet somewhere. Going through parley's numeric weight would mean reaching into a
dependency's internals from a kit crate, so each kit names the nine variants
instead, which makes the mapping checkable.

That surfaced a silent difference: **v1's `to_floom()` sends `Black` to `EXTRA_BOLD`
(800)**, because parley's table has no 900, while the direct mapping gives
`floom_types::Black` (900). This port applies the *fixed* answer — so `Black` is the
one weight where this crate and v1 disagree. It is unverifiable from here: no kit
reads `Black` at any call site, which is the same fact the plan already records about
`to_floom()`. Applied here rather than in `to_floom()` on purpose, so the two answers
differ in exactly one place and that place is written down.

**Two props that are carried and never read.** `ListItemProps::nav`'s own doc claims
"extra trailing affordance space + hover affordance", and v1 read it nowhere in
`view()` — neither exists. Kept unread, with a comment saying that reading it would
mean inventing the behaviour the doc promises. `ListItemTokens::padding_horizontal`
*is* read; `text_gap` is v1's literal `.gap(4.0)` written at the call site, so it is
a style field here rather than buried in a call site, and a test asserts the column's
gap is not the row's gap.

**Three mutations, and the ripple's radius is asserted to be zero.** v1 passed no
radius to the row's ripple, and a full-width row with rounded ink reads as a card, so
`the_ripple_is_not_rounded` pins `clip_radius() == 0.0` — the line to change if a kit
ever wants a rounded row, to be changed together with the test.

| Mutation | Tests red |
|---|---|
| the band's `min_height` becomes `height` | 1 — `the_band_has_a_height_floor_and_fills_its_width` |
| the text column stops growing | 1 — `the_text_column_is_the_only_part_that_grows` |
| the row inside the band stops growing | 1 — the same test, its `CONTROL` assertion |

### What `md-dialog` turned up before it was ported — one capability, and one decision

`md-dialog` is 271 lines at 90% cross-kit overlap, and it is the first component in
this batch whose blocker is **not** a missing capability. Correction 11 has the
detail; the short version is that a v1 dialog owns its `open: RwSignal<bool>` and a
2.0 one cannot, because `RwSignal` is `Rc`-based and a slot builder must be
`Send + Sync`. That is a decision about state ownership, and it is recorded rather
than half-ported.

The assessment was still worth doing, because two of the three things it appeared to
need turned out not to be gaps — and that is the outcome worth having.

**The scale animation exists.** v1 keyframes `scale(95.pct())` → `scale(100.pct())`
over 220ms on both create and remove, and `CompositorProp::Scale(f64)` is already an
`AnimationSpec` channel. `MOTION-GAP.md`'s B4 is the *height* transition, which this
component does not use — so the two were easy to confuse, and only reading the spec
settled it.

**"Clicks inside the card must not dismiss" needs no propagation API.** v1 wrote
`.on_event(listener::Click, |_, _| EventPropagation::Stop)` on the card. 2.0's host
builds a route from the hit node *outward* and stops at the first widget that
consumes, so anything inside the card consuming the press already stops the scrim
from seeing it. The 2.0 idiom would be a `Pressable` with no callback — it consumes
and publishes none — which is slightly odd to read and needs nothing new.

**`max_width` was a real gap, and is now closed** — the eleventh capability, and the
other half of the `min_width` added for `md-list_item`. v1's card is
`width_pct(100.0).max_width(..)`, and with only `width` one of the two had to be
dropped. floem-ui asks for `max_width` **22 times across 8 crates**, so it is part of
the vocabulary rather than one component's quirk; `max_height` came with it because
half the call sites are "cap this thing that is otherwise unbounded".

**And a lesson about testing a ceiling, which cost a whole rewrite.** The first
version of the cap test put one character in a 180pt-capped box and asserted
`<= 180`. The mutation — making the engine *size* a box to its ceiling instead of
capping it — **passed**, because a one-character box is already narrower than 180, so
pinning it to 180 still satisfies `<= 180`. A ceiling that cannot be violated cannot
be tested.

The fix was not one test but two, because the two cases are distinguishable in
opposite directions and neither alone suffices:

| content | ceiling | "size me to the cap" | what only the ceiling gets right |
|---|---|---|---|
| wider than 180 | 180 | 180 | **identical** — so this case cannot detect the mutation |
| narrower than 180 | 180 | 180 | the box shrinks to its content |

So `a_max_width_caps_content_that_wants_to_be_wider` proves the ceiling applies, and
`content_narrower_than_the_cap_still_shrinks_the_box` proves it is not a size. With
both in place the mutation turns exactly the second red. The floor suite needed no
such pair, because a floor and a height differ even when the content is *shorter* —
which is the asymmetry that made me write one test where two were required.

### What `md-alert` turned up — the first of the six that needed nothing

The six crates correction 7 identified were treated as one batch, and this one
breaks the batch in half.

`md-alert` is a tinted rounded `Surface` holding a `Stack` of two more stacks and
some `Text`, plus a bare pressable icon for the dismiss affordance. **Every one of
those pieces already existed.** No new engine capability, no new shape, no promotion
warranted either: at 83% cross-kit overlap the shape is shared, but unlike the
`icon_button` (95%) or `list_item` (84%) there is no *decision* in the shared part
worth centralising — it is three nested stacks and a `Surface`, and the engine's
`Stack`/`Surface` already say all of it. The right answer was to write it once in a
kit crate and be done.

That is worth recording because it is the outcome a port *should* have most often,
and this plan had been carrying "14 crates, all ports" as if they were all alike.

**One deliberate behaviour change, pinned.** v1 wrote:

```ignore
let dismiss = if dismissible { MdIcon::new("close")... } else { Label::new("") };
```

So the row **always** had two children, and the 8px gap after the content was spent
on an empty label. A non-dismissible alert was 8px narrower in its content than it
needed to be. Here the slot is simply absent, so the row has one child. Reproducing
v1 exactly would mean reproducing its bug; dropping it is a *change*, so it is
pinned by `the_dismiss_slot_is_absent_unless_the_alert_is_dismissible`, whose
mutation — making the slot unconditional — turns 2 tests red.

**The `MdIcon::view()` site, and what replaced it.** v1 chained
`.style(|s| s.padding(2.0).cursor(Pointer)).on_event(listener::Click, ..)` onto the
icon's view. In 2.0 the icon *is* a widget, so the chain is gone and a clickable icon
is that widget inside a [`Pressable`] — which also gains a label, so a screen reader
can name the dismiss button. The pointer cursor is the fourth time in this port that
`PaintContext`'s lack of a cursor vocabulary has cost something; recorded each time
rather than worked around.

**The third `FontWeight` crossing, again.** `FontWeight::Bold.to_floom()` returns a
`parley` weight, so the same nine-variant mapping `md-list_item` needed appears here.
This is now the third kit to need it, which says the mapping belongs somewhere shared
rather than in three crates — see the open item at the end of this section.

**An engine change that turned out not to be one.** Reaching for `Text`'s string
added `content()`, `edge()` and `font_weight()` to `primitives::Text` — and then the
compiler reported all three as duplicates: `content()`, `font_size()` and
`font_weight()` were **already there**. The readers existed; I had not looked. Three
lines added, three removed, and the crate needed nothing — which is the second time in
this batch that the honest answer turned out to be "the engine is fine", after the
`min_width`/`min_height` that turned out not to be.

| Mutation | Tests red |
|---|---|
| the dismiss slot becomes unconditional (i.e. v1's bug) | **2** |
| the container tint drops its 26/255 alpha | 1 |

### What the `tree_view` turned up — a shared answer, and two ways to lose it

87% cross-kit overlap, so the shape went to the engine as `floom_widgets::TreeView`
and each kit became a token translation. 53 tests across the three.

**The state is shared for a stronger reason than `SelectGroup`'s was.** Selecting a
row restyles other rows but never adds or removes one. Expanding a tree node
**changes which rows exist** — the node's own row stays, its descendants' rows go,
and every row below shifts up a slot. So a row that remembered "I am expanded" would
be right about itself and wrong about everything after it, and a host that rebuilt
the subtree on each toggle would throw away the hover and focus state of every row
underneath. The shape is the same `Arc<Mutex<..>>` with a re-flatten, and the same
"every node is visited each frame, so nobody has to send a notification" reasoning.

**Two v1 details that are bugs if reproduced and invisible if dropped.**

* v1 rendered `Empty::new().style(|s| s.width(expander_size)…)` for a childless
  node's expander, and `Empty::new().style(|s| s.width(icon_size))` for a node with
  no icon — so it kept every row's geometry stable and the labels lined up. Here the
  **boxes stay and only the widgets inside them are absent**, which is the same
  geometry and a different child count. A tree where a missing icon shifts every
  label below it is a different component, and `a_childless_nodes_expander_box_is_empty_but_the_same_size`
  is what holds that.
* `TreeviewTokens::item_gap` is declared and **never read by v1** — `flatten` built
  one flat list and the gap lived on the container. Carried in the style and left
  unused, for the same reason `ToolbarTokens::height` is, with a test that says so.

**`Stack` could not declare its own extent.** v1 set `height(row_height)` and
`padding_horiz(8.0)` on the row, and the row *is* a `Stack`. Its siblings all had a
request — `Surface::request`, `Button::layout_request` — and a stack did not, so the
obvious translation was to wrap every row in a `Surface` and add a node per row to
carry one number. `Stack::request` now exists, and `Stack::layout_request` merges the
caller's request with the stack's own nature (axis, gap, alignment) laid over it.

**Adding that reader cost a real behaviour, and the compiler said nothing.** The first
version added an *inherent* `Stack::layout_request`, which **shadows** the `Widget`
trait method — so a row's `gap` and its alignment silently vanished from the answer,
because the stored request does not carry them and only the trait method does the
merge. The symptom was an assertion reading `gap: None` on a row built with
`.gap(4.0)`. It is now `declared_request`, and the doc comment says why it is not
named `layout_request`. That is the **second** time in this port that adding a
convenience reader cost behaviour; both times the compiler was silent, and both times
the test that noticed was one that asserted a field the new method did not carry.

**`node_at` type-checked and was wrong.** Its first version walked the path and
returned `level.last()` of the final node's *children* — so it returned a slice, not a
node, and every label in the tree would have been another node's. `node_at_resolves_a_path_to_that_node`
is the walk, asserted per level, because a wrong-but-plausible descent is the kind of
bug a compiler cannot see and a type cannot express.

| Mutation | Tests red |
|---|---|
| `flatten` descends into collapsed nodes | **4** |
| a toggle only ever expands, never collapses | **4** |

Two more expectations in the suite were mine and wrong, and the tests were right:

* the fixture left the root collapsed, so three tests correctly reported one row —
  a node's children show when **that** node is open, and the root being visible says
  nothing about it;
* and collapsing a node removes its *children's* rows, not its own — one row, not two.

### What mounting the combobox turned up — three wirings, all silent

The `Widget` impl, 20 tests. Three of the failures below are **wiring**, and they share
a shape: nothing errored, nothing looked wrong in review, and each produced a combobox
that was quietly not a combobox.

**`pick.open` and `host.present` are two sources of truth, and nothing wired them.**
`pick.open` is what the user asked for — the chevron writes it, a keystroke writes it,
the host's own dismissal writes it. `OverlayHost::present` is what the overlay
bookkeeping acts on. A widget that owns both has to say so once per frame, in **both**
directions:

* `update` copies `pick.open` into `host.present`, or the panel is never requested;
* `overlay_closed` copies the host's dismissal back into `pick.open`, or the next
  `update` re-opens a panel the user just dismissed.

Both were missing at first, in opposite directions. The mutation table's first row is
the first one; the second is the second.

**Keeping a selection's raw index across an option-list replacement makes the widget
contradict itself.** `selected` is an `Option<usize>` — that is what v1 stored and what
`on_change` reports — and an index stops meaning anything the moment the list behind it
is replaced. So the selection is re-pointed **by text**: the option that was chosen is
looked up in the new list and becomes the new index. Keeping the raw index leaves a
combobox whose field reads "banana" while `selected()` says `Some(1)` and index 1 is
now "apple" — two of the three disagree and the user sees the one that is wrong.

If the chosen option is **gone**, the selection is dropped *and the field cleared*: a
field still reading "banana" above a list with no banana looks right, which is worse
than one reading nothing. My test for this was wrong at first in an instructive way — I
replaced the list with `["apple", "Cherry"]` and index 1 *was* `Cherry`, so the
selection did still resolve and the code was right.

**The panel is `None` when nothing matches, not an empty box.** A dropdown with nothing
in it should not be a bordered, shadowed rectangle around nothing — and returning `None`
also means the host is never asked, which is cheaper than asking and being refused.

Two smaller things the port settles rather than guesses:

* **The list opens on the first keystroke, not on focus.** v1 opened on
  `FocusGained`; 2.0 has no way for a parent to observe a child's focus, so the same
  user-visible moment is used minus the "tap the field, see nothing happen" one.
  Recorded, not worked around.
* **A row's selected fill is read when the row is built**, not when it is painted.
  Child keys do the work: choosing changes `selected`, the reconciler is asked for
  children again, every row is rebuilt, and a now-chosen row paints its fill.

And a test of mine that would have passed for the wrong reason: the disabled/enabled
comparison painted the `Combobox`, whose `paint` deliberately draws nothing — the
trigger is a child — so it compared two empty scenes. It now paints the trigger.

    Mutation                                        Tests red
    `update` never tells the host the user opened    4
    a host dismissal does not clear `pick.open`      1
    the selection keeps its raw index                2

### What the combobox turned up — three decisions, and two of them are disagreements

`floom_widgets::combobox`, 24 tests. 91% cross-kit overlap — the three v1 crates differ
by 26 lines, all of it type names, the icon source, the theme source, and one
`.margin_top(4.0)`. So this promotes rather than being ported three times, the same
call as `icon_button` and `list_item`.

**It is a decision layer with no `Widget` impl yet**, on purpose. 2.0 had neither a
popup placement nor a shared buffer, so both had to be built first (`PopupPlacement`,
`TextInput`, correction 13). This file settles what can be settled without mounting
anything — the filter, the selection state machine, the placement, and the token
translation — which is the ordering this port adopted at the start: design logic first,
markup second.

**The filter is v1's, and two parts of it are easy to "improve" into a behaviour
change.** `q.is_empty() || opt.to_lowercase().contains(&q)`: an empty query matches
everything, so an untouched combobox opens showing its options rather than an empty
list; and it is a *substring* match, so `"ban"` finds `"urban planning"`. A prefix
filter would look like a refinement and would silently stop finding things — the
mutation table has that one turning two tests red.

**Choosing is one decision, not four statements.** v1's row click set `selected`,
`query`, `open`, and the callback. Three of those are one decision — *the user chose
this, so the field reads it and the list goes away* — and a decision written at four
call sites is written three and a half somewhere. Hence `select`. It also returns
whether anything changed, so a second click on the already-chosen row (still there
behind the list) does not repaint.

**Two disagreements are preserved, not resolved:**

* **The panel gap.** Material wrote `.margin_top(4.0)`. Cupertino and Fluent wrote
  nothing, so their panels sit flush against the field. That is a design disagreement
  between three design systems; `panel_gap` is a parameter and `ComboboxStyle`'s
  default is Material's 4.0 — the only one of the three that wrote a number. A default
  invented here would have been a fourth opinion.
* **The panel's shadow is black**, at alpha 20: `with_alpha(rgba(0,0,0,255), 20)`,
  while everything else in this design system tints its shadow from the theme's own
  colour. Recorded by `panel_shadow_color`, and pinned by a test so that harmonising it
  later is a decision somebody makes on purpose.

And one ordering that reads like a typo: `border_color` checks **disabled before
open**, so a disabled combobox that somehow has its list up still draws the disabled
border. Reversed, it produces a combobox that looks live while it is not. Same for
`chevron_color`, which fades `on_surface` when disabled but shows plain
`on_surface_variant` when not — so the two states are not one colour at two opacities,
and the test says so explicitly.

**Mounted rows, not mounted-and-hidden.** v1 mounted *every* option and hid the
non-matching ones, so a keystroke rebuilt the whole list. Here only matching rows are
mounted, keyed by option index so a row that stays visible is reused rather than
rebuilt. Invisible to a user; visible in a profile.

Two of my own expectations were wrong and the code was right both times:
`"urban planning"` really does contain `"ning"` (it ends in *plan**ning***), and
`"banana"` has no `e` — I put it in an expected list and the filter correctly left it
out.

`select` also failed its first test because it did not do what its own doc comment
promised: it returned `true` for a re-selection. Fixed, and the test now pins it.

| Mutation | Tests red |
|---|---|
| choosing leaves the list up | 1 |
| the border checks `open` before `disabled` | 1 |
| the substring filter becomes a prefix filter | 2 |
| choosing leaves the field reading what was typed | 1 |

### What building the text input turned up — a buffer, and four obligations

`floom_widgets::TextInput`, 32 tests. Not a port: the first *new* widget in this
project, written because `combobox` and `autocomplete` both need one and 2.0 had
none. Correction 13 has the full account; what is here is what the tests had to
settle.

**The buffer is shared, and that makes the external write a first-class case.** v1's
input took an `RwSignal<String>` from the caller, which cannot cross a `Send + Sync`
slot boundary — the same wall as `md-dialog`. `Arc<Mutex<String>>` can, and a caller
keeps a handle. So the widget compares what it last read on every frame and adopts a
difference, which is where three of the tests live:

* `on_change` fires for a keystroke **and** for an external write, so a caller can
  treat the handle as a signal;
* an update with no external change fires **nothing**, or a model that rewrites the
  same value on every change would loop;
* and two fields over one handle see each other, proved with `Arc::ptr_eq` — two
  copies would both work and then drift apart, which is a silent bug rather than a
  wrong answer.

**Every offset is a byte index, and the caret must never land inside a character.**
This is the part that bites. A byte index that is not on a character boundary panics
on the next `String::slice`, and three things can produce one: an external write that
shrank the buffer, a caret set by a caller, and an IME `Preedit` whose offsets are
documented as needing exactly this clamping. So `clamp_to_char_boundary` is public, and
the test that earns its keep walks the caret left through `"héllo"` and shows the
positions:

    6, 5, 4, 3, 1        — and never 2

`2` is the second byte of `é`. A caret stepping by bytes would land there and the next
slice would panic. A four-byte emoji gets the same treatment
(`a_four_byte_character_survives_backspace`).

Two of my own expectations were wrong here, and the code was right both times: from
the end of a six-byte `"héllo"` one arrow-back is **5**, not 4 — I mis-added the
byte offsets twice before writing the sequence out properly, which is why the test
now prints every position instead of asserting one.

**An IME composition is held but not committed.** `Preedit` sets a field the paint
code reads and the buffer never sees, because a half-typed syllable must not be
readable as text; closing the panel commits it at the caret. `KeyPhase::Up` does
nothing, or every character would arrive twice. Escape fires a callback and does
**not** clear the field — v1's input had no escape behaviour, and inventing "escape
reverts" would be a decision a text field should not make on its own.

**The border is a filled band, not a stroke.** `PaintContext` has `stroke_path` but
no `stroke_rect`, so the outline is four thin rects inset by the border width — the
same choice the plan already made for single-edge borders, and for the same reason: a
fill lands inside the box, a stroke straddles its edge.

| Mutation | Tests red |
|---|---|
| backspace steps one byte, ignoring character boundaries | 2 — **and it panics** |
| the widget never notices an external write to the buffer | 2 |
| an IME composition is committed straight into the buffer | 2 |

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
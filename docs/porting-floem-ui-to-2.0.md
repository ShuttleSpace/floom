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

## The gap that gates everything

`floom-ui` needs 98 distinct components. 2.0 currently ships **39 widget types**,
of which only **19** cover a `floom-ui` component:

> **19 components can be ported now. 79 need a widget that does not exist yet.**

Already covered: `button`, `checkbox`, `container`, `dialog`, `divider`, `grid`,
`icon`, `image`, `list`, `navigation_drawer`, `overlay`, `radio`, `select`,
`slider`, `spacer`, `switch`, `tabs`, `text`, `text-field`.

Missing (79): `alert`, `app_bar`, `autocomplete`, `avatar`, `badge`, `banner`,
`bottom_navigation`, `breadcrumbs`, `button_group`, `calendar`, `card`,
`carousel`, `chip`, `chip_group`, `code`, `color_picker`, `combobox`,
`confirm_edit`, `counter`, `data_iterator`, `datatable`, `date_adapter_provider`,
`date_picker`, `defaults_provider`, `display_provider`, `empty_state`,
`expand_transition`, `expansion_panels`, `fade_transition`, `file_input`,
`floating_action_button`, `footer`, `form`, `hover`, `icon_button`,
`icons_provider`, `infinite_scroller`, `item_group`, `kbd`, `list_group`,
`list_item`, `list_subheader`, `locale_provider`, `main`, `menu`,
`number_input`, `otp_input`, `pagination`, `parallax`, `progress_circular`,
`progress_linear`, `pull_to_refresh`, `radio_group`, `range_slider`, `rating`,
`responsive`, `scale_transition`, `sheet`, `skeleton_loader`, `slide_group`,
`slide_transition`, `snackbar`, `sparkline`, `speed_dial`, `stepper`,
`system_bar`, `table`, `textarea`, `theme_provider`, `time_picker`, `timeline`,
`toolbar`, `tooltip`, `treeview`, `validation_messages`, `virtual_scroller`,
`window`.

Many are trivial compositions of what exists (`card`, `footer`, `item_group`,
`toolbar`). Several are substantial and carry native dependencies
(`file_input`, `color_picker`, `autocomplete`, `combobox`, `calendar`,
`date_picker`, `time_picker`).

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

#### `Ripple` is the first Stage B item

`Ripple` (the Material press ripple) is **not** ported. It is not a find-and-
replace problem, and the reason is a genuine 2.0 gap:

- The v1 ripple was a `View` wrapper that animated a growing ink circle on
  pointer-down and faded it on pointer-up, clipped to rounded bounds.
- 2.0 has every *piece*: `WidgetHost::bounds` / `dispatch` / `animate`, and
  `Widget::clip`.
- But 2.0's animation contract is **declarative** — a widget returns one
  `AnimationSpec` from `Widget::animation` and the host owns the running state
  ("a widget has no host handle… so it cannot start anything itself"). That
  cannot express a **two-phase, pointer-driven** interaction: expand on press,
  fade on release, driven by event timing rather than a declared spec.

So Stage B needs an interaction-animation capability (per-widget animation
state machines started and advanced from `Widget::event`) before `Ripple` can
exist. Ten components depend on it and do not build until then:
`{material,cupertino,fluent}-card`, `-list_item`, `-icon_button`, and
`material-button`. Everything else in `floom-widget-runtime` is live, and
verified: a non-`Ripple` consumer (`floom-widget-md-avatar`) builds against it.

The mapping table established there is the template for the rest:

| v1 | 2.0 |
|---|---|
| `floom::reactive::{RwSignal, SignalGet, SignalUpdate}` | `floom_legacy_reactive::{…}` (re-exported by the 2.0 prelude) |
| `floom::create_global_rw_signal` | `floom_legacy_reactive::create_global_rw_signal` |
| `floom::peniko::Color` | `peniko::Color` |
| `floom::text::FontWeight` | `parley::style::FontWeight` |

Porting it also surfaced a latent `cfg` bug: `global()` is gated on a
design-system feature but three public wrappers that call it were not, so
`cargo check -p floom-core-theme` with no features had never compiled. Fixed.

### Stage B — the 79 missing widgets

Build them in `floom-widgets`, ordered by how many components unblock:

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
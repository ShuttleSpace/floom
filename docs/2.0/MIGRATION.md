# Migrating from Floom v1 to 2.0

Two crates, two models, one repository:

- **`floom`** is the v1 monolith (`src/`). It is preserved and **will keep
  working for the current major version**. Nothing in this document is required
  of an app that stays on it.
- **`floom2`** is the 2.0 facade (`crates/floom2`). It is a different architecture,
  not a new coat of paint on the old one: a layered crate graph, an explicit
  element tree, three replaceable ABIs, and a headless app shell.

The two trees **do not mix in one frame**. There is no adapter that mounts a v1
`View` inside a 2.0 host, and there is no adapter that mounts a 2.0 widget
inside a v1 app. Migration is therefore *subtree by subtree*: you port a screen,
not a file, and the ported screen is built from `floom2::prelude`.

Every snippet below is compiled and run by `crates/floom2/tests/migration.rs`.
That file is the source of truth; if this page and it disagree, the test is
right.

## Pick your path

| situation | what to do |
| --- | --- |
| staying on v1 for now | nothing, or switch `use floom::prelude::*` to `use floom::compat::v1_prelude::*` to pin the surface (below) |
| porting one screen | port that subtree to `floom2::prelude`; the rest of the app keeps running v1 |
| starting a new app | `floom2::prelude` |

## Step 0 — freeze the v1 surface (optional, cheap)

`floom::prelude` is the v1 prelude and will not change while v1 is supported,
but a future narrowing is easier to survive if the app imports the frozen copy:

```rust
// before
use floom::prelude::*;
// after
use floom::compat::v1_prelude::*;
```

`floom::compat::v1_prelude` is a verbatim mirror of today's v1 prelude — the
same items, same `TypeId`s (asserted by `tests/compat_prelude.rs`). Once an app
imports it, a future change to `floom::prelude` cannot touch it.

## Concept mapping

| v1 | 2.0 | note |
| --- | --- | --- |
| `View` / `IntoView` | `Widget` (and `WidgetAny` in type-erased trees) | both live in `floom-core`; custom widgets reach them through `floom2::advanced` |
| `impl IntoView` return value | `Child::new(ChildKey, LayoutRequest, \|\| Box::new(widget))` | children are **keyed** and built by the host |
| `RwSignal`, `create_effect` | application-owned state | the reactive graph is legacy; `floom-store` if you want signals back |
| `ViewId` | `NodeId` | `NodeId` is issued by, and scoped to, one `WidgetHost` |
| `.style(\|s\| s...)` | `LayoutStyle` (layout) + `StyleSet`/`StyleProps` (paint, per interaction state) | style is a value, not a closure |
| `floom::launch(view)` | `WidgetHost::new()` + `mount_root(..)` + `AppRunner` | the shell runs **headless** — that is why every example is a test |
| `floom::Renderer` (immediate mode) | `floom_renderer::Renderer` (retained: scene + damage in) | see [Renderer plugins](#renderer-plugins) |
| windowing baked into `floom` | `floom-platform` ABI (`Platform`, `Window`, `Surface`, `Clipboard`, `Ime`) | see [Platform plugins](#platform-plugins) |
| — | `floom-native` with capability negotiation | new in 2.0; see [Native views](#native-views) |

## Step 1 — state: signals → the application owns it

**Before (v1).** A signal, and a label that re-runs when it changes:

```rust
use floom::compat::v1_prelude::*;

let mut counter = RwSignal::new(0_i32);

let view = Stack::vertical((
    Label::derived(move || format!("Value: {}", counter.get())),
    Button::new("Increment").action(move || counter += 1),
));

// The reactive graph is what updates the label: no handle to the widget
// is involved, and none is available.
counter.set(41);
assert_eq!(counter.get(), 41);
```

**After (2.0).** The app holds the value, the callback bumps it, and the label
is updated in place (excerpt — the full snippet, including how `label_id` is
found, is `after::the_same_counter_mounts_clicks_and_updates` in the test):

```rust
use floom2::prelude::*;

let clicks = std::sync::Arc::new(std::sync::atomic::AtomicI32::new(0));
let counter = clicks.clone();

let root = Stack::new(Axis::Vertical)
    .child(Child::new(ChildKey::index(0), LayoutRequest::new(), || {
        Box::new(Text::new("Value: 0"))
    }))
    .child(Child::new(
        ChildKey::index(1),
        LayoutRequest::new(),
        move || {
            let counter = counter.clone();
            Box::new(Button::new("Increment").on_click(move || {
                counter.fetch_add(1, std::sync::atomic::Ordering::SeqCst);
            }))
        },
    ));

let root_id = host.mount_root(LayoutStyle::default(), root);
host.frame();

// ... dispatch a press/release over the button, then:
host.widget_mut::<Text>(label_id)
    .expect("label")
    .set_content(format!("Value: {}", clicks.load(std::sync::atomic::Ordering::SeqCst)));
host.frame();
```

Two things to know:

- **`Child` builders are `Send + Sync`** (the host may replay them on any
  thread), so anything captured into one must be too. `on_click` itself takes a
  plain `Box<dyn Fn()>` and is single-threaded.
- **Mutating a widget bumps its revision**, which is what makes the next frame
  repaint it. Forgetting `host.frame()` is the most common "why didn't it
  update".

If you want signals back, `floom-store` (`store(value)`, `select(..)`) provides
fine-grained selectors. It is built on the legacy reactive crate — the one 2.0
→ v1 edge still live in the graph, and one the architecture gate reports as a
note.

## Step 2 — the tree: values → keyed children

In v1 a view is a value you return. In 2.0 the host owns the live widgets and
you describe children with a **key**:

```rust
Child::new(ChildKey::index(0), LayoutRequest::new(), || Box::new(Text::new("first")))
```

The key is what makes reconciliation cheap: keep the key and the widget
survives, change the key and it is replaced. This is the contract a v1
`dyn_stack` / `virtual_stack` user is moving to — see
`examples/floom2-list` (keyed rows survive a reorder) and
`examples/floom2-virtual-list` (38 elements materialized for 100 000 rows).

## Step 3 — identity: `ViewId` → `NodeId`

`ViewId` is a handle into process-global state, handed out at construction and
usable from anywhere:

```rust
use floom::ViewId;

let id = ViewId::new();
assert!(id.is_valid());
```

`NodeId` is issued by a **specific host** and is meaningless to any other:

```rust
let root_id = host.mount_root(LayoutStyle::default(), Text::new("one"));
assert!(host.is_mounted(root_id));
assert!(!other_host.is_mounted(root_id)); // a different host does not know it
```

Practical consequence: code that stashed a `ViewId` in a struct, a thread or a
`static` has to carry the host (or the node's `ElementId`, available as
`host.node_id(node)`) with it.

## Step 4 — styling: closure chains → style values

v1 builds style with closures, including state selectors:

```rust
"Increment".style(|s| s
    .border_radius(10.0)
    .hover(|s| s.background(palette::css::LIGHT_GREEN))
    .active(|s| s.color(palette::css::WHITE))
    .disabled(|s| s.background(palette::css::LIGHT_GRAY)))
```

2.0 resolves a sparse property bag per interaction state.
`floom_style::StyleSet` maps `StyleState` (`NORMAL`, `HOVER`, `PRESSED`,
`FOCUSED`, `DISABLED`, `SELECTED`) to `StyleProps` (`background`, `foreground`,
`opacity`, `font_size`, `padding`, `radius`, `border_width`, `border_color`,
`elevation`), and `resolve(active)` merges the matching sets — `None` means
"unset", so a hover set that mentions only `background` leaves the rest alone.
The design systems (`floom-material`, `floom-cupertino`, `floom-fluent`,
`floom-harmony`) emit `StyleProps`, they do not paint.

Layout is separate: `LayoutStyle` (from `floom-layout`) is a plain value, and
`set_style(node, style)` replaces it.

## Renderer plugins

v1's `Renderer` is immediate mode — the framework walks the tree and calls you:

```rust
fn begin(&mut self, capture: bool);
fn set_transform(&mut self, transform: Affine);
fn set_z_index(&mut self, z_index: i32);
fn clip(&mut self, shape: &impl Shape);
fn fill(&mut self, path: &impl Shape, brush: impl Into<BrushRef>, blur_radius: f64);
// ... stroke, push_layer, draw_glyphs, draw_svg, draw_img
fn finish(&mut self) -> Option<peniko::ImageBrush>;
```

2.0's is retained — you get a scene and a damage region:

```rust
fn begin_frame(&mut self, info: &FrameInfo);
fn render_scene(&mut self, scene: &SceneGraph, damage: &DamageRegion);
fn render_scene_with_resources(&mut self, ..); // resources pre-uploaded
fn end_frame(&mut self);
fn trim_caches(&mut self) {}
```

The port is: keep your device setup, replace the draw-call stream with a scene
walk. Widgets name `floom-renderer`, never a backend; only `floom-app` names
both. Backends are verified against the ABI's own tests
(`crates/floom-renderer/tests/contract.rs`, `render_handoff.rs`), not only
theirs.

## Platform plugins

v1 talks to winit directly from the monolith. 2.0 hides it behind
`floom-platform`:

```rust
pub trait Platform {
    fn id(&self) -> PlatformId;
    fn capabilities(&self) -> PlatformCapabilities;
    fn create_window(&mut self, config: WindowConfig) -> WindowId;
    fn window(&self, id: WindowId) -> Option<&dyn Window>;
    fn poll_events(&mut self, out: &mut Vec<PlatformEvent>);
    fn request_frame(&mut self, window: WindowId);
    fn now_secs(&self) -> f64;
    fn ime(&mut self, _window: WindowId) -> Option<&mut dyn Ime> { None }
}
```

plus `Window`, `Surface`, `Clipboard` and `Ime`. `floom-ui-events-winit` adapts
winit's event loop; a new backend implements the traits and registers in three
places (see `docs/2.0/ARCHITECTURE.md`). Conformance:
`crates/floom-platform/tests/contracts.rs` and, on desktop,
`crates/floom-platform-desktop/tests/{lifecycle,accessibility,winit_capabilities}.rs`.

## Native views

New in 2.0 and with no v1 counterpart: `floom-native`. A provider declares
capabilities; the controller **negotiates** a composition mode per participant,
and a mode the backend cannot do is downgraded, never silently granted — a
participant granted an impossible mode draws nothing, which looks exactly like
one that was never asked for. `examples/floom2-native-plugin` writes a provider
from scratch.

## The compatibility tier

Two mechanisms, both from Phase 29.2:

1. **`floom::compat::v1_prelude`** — the frozen v1 surface (Step 0).
2. **A diagnostic at the mount point.** `floom_core::Widget` and `WidgetAny`
   both carry `#[diagnostic::on_unimplemented]`, so the most common
   cross-version mistake reports the migration path on the line that needs it:

   ```text
   error[E0277]: the trait bound `Label: Widget` is not satisfied
     = note: `Label` is not a Floom 2.0 `Widget`
     = note: v1 `floom::view::View` values are compatibility APIs and cannot be
             mounted into a Floom 2.0 host
     = note: rebuild the subtree from `floom2::prelude` widgets, or keep the
             application on the v1 `floom` crate (preserved for the current
             major version)
   ```

   It is on *both* traits because mount points take `impl WidgetAny`; placing it
   on `Widget` alone left `impl WidgetAny` errors bare.

## What has no 2.0 equivalent

Stated plainly, so nobody plans a port around them:

- **The global reactive graph.** `create_effect`, `create_memo` and friends are
  v1. 2.0 owns state explicitly; `floom-store` is the opt-in bridge.
- **`ViewId`'s global reach.** No 2.0 handle works across hosts.
- **Immediate-mode rendering.** There is no shim that turns a v1 `Renderer`
  into a 2.0 one.
- **The crate name `floom`.** The facade is `floom2` because the monolith holds
  the name. Renaming it is a one-line change once the monolith retires.

## Verification

```bash
cargo test -p floom2 --test migration      # the snippets on this page
cargo test --workspace --no-fail-fast      # everything
bash scripts/check-architecture-deps.sh    # the layering
```

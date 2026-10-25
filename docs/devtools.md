# The devtools panel

The devtools are one line in, a keystroke away — the same deal Electron's
devtools make: `require('electron-devtools')` once, then Ctrl+Shift+I whenever
you want the panel.

## The one line

```rust
let devtools = floom::devtools::attach(&mut host, |state| {
    vec![floom::devtools::ReadoutRow::new(
        "elements",
        state.selected().unwrap_or("none"),
    )]
});
```

`attach` mounts the panel as an overlay above your own tree and hands back the
handle. It starts **hidden**, so an app that never opens it pays for one overlay
and the widget inside it, and nothing more.

The `readout` closure is the panel's content: it is called every frame the panel
is visible, and it answers from your app's own state — typically the inspector
over the live tree the host retains.

Mounting it by hand also means running `Devtools::sync` once per frame; without
it a hidden panel keeps eating its column and a visible one cannot be clicked.
See [The panel is chrome](#the-panel-is-chrome-not-a-dialog).

## Opening it

Every way sets the same flag:

| How | What it is |
| --- | --- |
| **F12** | the shortcut, wired once in the platform loop |
| **Ctrl+Shift+I** (Cmd+Shift+I on macOS) | the same, where F12 belongs to the OS |
| `devtools.show()` / `devtools.hide()` | `show: true` / `show: false` from code |
| `devtools.toggle()` | what the shortcut itself does |
| **Escape** | closes the panel, but only while it is up |

Escape lives in `Devtools::handle_key` rather than in the panel widget, for the
same reason the shortcut does: the keyboard belongs to the app, not to the
panel. A key routed to the focused control never reaches a widget inside an
overlay, so a panel that only listened in its own `event` would be closable from
Escape only in the one case where it had taken the keyboard — which it
deliberately does not. `handle_key` returns `false` while the panel is hidden, so
Escape stays the app's until the panel is actually up.

## Why the shortcut is wired in the loop, not the tree

A key event arrives at the platform loop *before* any widget is consulted. So
the shortcut has to be answered there too — otherwise a keystroke that opens the
panel would also type into whatever had focus.

`WinitApp::key` is that door:

```rust
impl WinitApp for MyApp {
    fn key(&mut self, event: &floom_types::KeyEvent) -> bool {
        self.devtools.handle_key(event)   // true = consumed, tree never sees it
    }
}
```

Returning `true` drops the event: the platform does not forward it as a
`PlatformEvent::Key`, so the focused control never receives it. Returning
`false` (the default) forwards the key, which is what an app with no global
shortcut wants.

If you use the facade's `run_desktop_with_devtools`, both halves are already
done — that is why it exists:

```rust
floom::run_desktop_with_devtools("my app", Size::new(480.0, 320.0), |host| {
    host.mount_root(LayoutStyle::default(), Text::new("hello"));
}, |state| {
    vec![floom::devtools::ReadoutRow::new(
        "visible",
        state.visible().to_string(),
    )]
});
```

## Where the panel lives

`floom-devtools` — the same crate as the headless
[`Inspector`](../crates/floom-devtools/src/lib.rs). The inspector answers
questions about a retained tree; the panel is the visible half of the same
crate, a real widget built from engine primitives rather than a second window.

The facade re-exports it under `floom::devtools`, behind the `devtools` feature.

## The panel is chrome, not a dialog

Three things follow from that, and all three are wired rather than documented
intent:

- **While hidden, its column belongs to the app.** A drawer overlay has bounds
  whether or not it paints anything there, and the overlay stack hit-tests
  against bounds — so a hidden panel would eat every press down the trailing edge
  from the first frame. `attach` therefore marks the overlay passthrough while
  the panel is down and interactive again when it is up.
- **A press outside it does not close it.** The overlay is opened
  non-dismissible, so clicking the app to inspect it does not unmount the panel
  you are inspecting with.
- **Attaching it does not take the keyboard.** Opening a non-modal overlay moves
  focus into it, which is right for a dialog and wrong for chrome. `attach`
  records what had focus and puts it back immediately, so the app can type from
  the first frame.

Those three are driven by `Devtools::sync`, which compares the panel's visibility
with the last visibility it saw and does nothing when it has not moved. It must
run **once per frame**, after whatever changed the visibility:

```rust
// The facade does this for you; this is what it does.
devtools.refresh_tree(host);   // the panel reads the tree as it is now
devtools.sync(host);           // passthrough, hit regions, and the keyboard
```

`sync` needs `&mut WidgetHost` because two of those three are host state, not
widget state. A caller that mounts the panel by hand and skips it gets a panel
that is visible but cannot be clicked, or hidden but still eats its column.

## Try it

```sh
cargo run -p floom-devtools-demo
```

A window with a heading, a row of three buttons and a counter. Press **F12** (or
**Cmd+Shift+I** on macOS, **Ctrl+Shift+I** elsewhere) and
the panel docks over the trailing edge; **Ctrl+Shift+C** arms the pick, and
clicking a button selects it without pressing it. The bar at the top of the panel
switches between **Elements**, **Style** and **Layout** — click a tab rather than
typing a chord, the way Chrome does it. **Escape** closes the panel again. The
demo is one call — `run_desktop_with_devtools` plus the readout closure — and the
whole point of it is that there is nothing else to write.

The demo needs a GPU window, so CI cannot run it. What CI *does* run is
`crates/floom-app/tests/devtools_demo.rs`, which mounts the same app into a real
host and checks the whole chain headlessly: the tree is describable, the panel
composes the app's rows over the engine's, refreshing the cell picks up new
nodes, and a picking click selects without pressing. If that passes, the demo's
one call is doing what it claims.

## The Chrome layout

`attach_elements` composes the readouts that are *about the element tree*:

```rust
let devtools = floom::devtools::attach_elements(&mut host);
// ... then, once per frame:
devtools.refresh_tree(&host);
```

| Panel | What it shows | Where it comes from |
| --- | --- | --- |
| **Elements** | the element tree, nested by depth, with the selected node marked | `attach_elements` |
| **Breadcrumb** | `root > body > ok` — the path to the selection | `attach_elements` |
| **Styles** | the selection's resolved style in every inspected state | `attach_elements`, tab `Style` |
| **Layout** | the selection's box: size, flex, axis, gap, align, inset | `attach_elements`, tab `Layout` |

The tab drives which half is composed in: `devtools.set_tab(DevtoolsTab::Style)`.

### The tab bar

The tabs are clickable. `attach_elements` and `attach_with_rows` mount a bar
above the rows, one button per tab, with the active one filled differently:

| tab | the bar says |
| --- | --- |
| `DevtoolsTab::Tree` | `Elements` |
| `DevtoolsTab::Style` | `Style` |
| `DevtoolsTab::Layout` | `Layout` |

Clicking a tab calls `state.set_tab`, the same cell `set_tab` writes from code,
so the bar and the API are one path. The buttons are built with
`Button::focusable(false)`: a click on the chrome must not take the keyboard away
from the app underneath, and a tab is chrome.

`attach` (the caller-supplied readout) mounts **no** bar — it has no rows of its
own to switch between, and a bar of dead buttons is worse than no bar. A caller
with its own multi-tab readout opens one explicitly:

```rust
let devtools = floom::devtools::attach_with_tabs(
    &mut host,
    |state| match state.tab() {
        floom::devtools::DevtoolsTab::Semantics => floom::devtools::semantics_rows(&entries),
        floom::devtools::DevtoolsTab::Updates => floom::devtools::update_rows(&path),
        _ => Vec::new(),
    },
    [floom::devtools::DevtoolsTab::Semantics, floom::devtools::DevtoolsTab::Updates],
);
```

That is also how a caller fills the two tabs the element readout cannot: the
facade has no semantics entry list and no invalidation path, so those two tabs
are the caller's to answer for.

The tabs you pass are the tabs that get buttons, in the order you pass them.
`DevtoolsTab::ELEMENTS` is the three the element readout can fill; `ALL` is all
five, and is wider than the default 280 px panel, so a caller that wants it
should widen the panel or accept the squeeze.

Two more readouts exist as plain row builders, `performance_rows` and
`console_rows`, but `attach_elements` does **not** compose them, because their
inputs are not the element tree: the first needs a
`floom_inspector::PerformanceSnapshot` from a profiler you run, the second needs
the `floom_error::Report`s your own error boundary caught. Compose them yourself
with `attach_with_rows`:

```rust
let devtools = floom::devtools::attach_with_rows(&mut host, move |_state| {
    let mut rows = floom::devtools::performance_rows(&snapshot);
    rows.extend(floom::devtools::console_rows(&reports));
    rows
});
```

`DevtoolsTab::Semantics` and `DevtoolsTab::Updates` likewise have row builders
(`semantics_rows`, `update_rows`) and no data source inside the facade — a tab
set to either shows the tree and the breadcrumb and nothing below them until the
caller's own readout supplies those rows.

## Picking an element

Chrome's signature interaction — press inspect, click the page, see what you
clicked:

```rust
let devtools = floom::devtools::attach(&mut host, |_| Vec::new());
let picker = floom::devtools::attach_picker(&mut host, devtools.state());
```

The picker needs the panel's state cell (`devtools.state()`, an
`Arc<Mutex<DevtoolsState>>`) rather than the panel itself: a pick *is* a
selection, so the two share one cell and the picker can write the selection the
readout reads without a second handle.

**Ctrl+Shift+C** (Cmd+Shift+C on macOS) arms it; the next click selects, and a
box is drawn around it. An event loop drives the rest:

```rust
// A key, before the tree: an armed pick means the pointer is the inspector's.
fn key(&mut self, event: &KeyEvent) -> bool {
    if self.picker.handle_key(event) {
        return true;
    }
    self.devtools.handle_key(event)
}

// A pointer event, before the tree: the same rule, and the host owns the
// mapping from a point to a live-tree key because it owns the tree.
fn pointer(&mut self, event: &PointerEvent) -> bool {
    let host = self.runner.host();
    let consumed = self.picker.pointer(event, |point| {
        host.element_at(point).map(WidgetHost::node_key)
    });
    self.picker.commit_selection();
    consumed
}
```

A click while picking **selects without pressing**: the picker consumes the
event before the tree sees it. `commit_selection` writes the pick into the panel
state, which is what makes the Elements row and the breadcrumb follow the click.

The highlight is its own overlay, above the panel, so the box is visible over
everything including the devtools. It is mounted **passthrough** — it covers the
window so it can outline an element anywhere in it, and a covering overlay that
also took clicks would make the app unclickable while the box was up, including
unclickable for the pick it exists to serve. Retarget it once per frame:

```rust
picker.sync_highlight(&mut host);   // the box follows the hover, then the pick
```

## What the panel is *not*

It is not a general-purpose debug overlay. It draws rows of text: whatever your
`readout` closure says, in a docked slab on the trailing edge. The engine's
inspector (`Inspector`, in the same crate) is what turns a tree into those rows,
and it stays headless — a CLI, a test, or a remote protocol client can use it
without ever mounting a panel.

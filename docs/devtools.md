# The devtools panel

The devtools are one line in, a keystroke away — the same deal Electron's
devtools make: `require('electron-devtools')` once, then Ctrl+Shift+I whenever
you want the panel.

## The one line

```rust
let devtools = floom::devtools::attach(&mut host, |state| {
    vec![floom::devtools::ReadoutRow {
        label: "elements".into(),
        value: "42".into(),
    }]
});
```

`attach` mounts the panel as an overlay above your own tree and hands back the
handle. It starts **hidden**, so an app that never opens it pays for one element
and nothing else.

The `readout` closure is the panel's content: it is called every frame the panel
is visible, and it answers from your app's own state — typically the inspector
over the live tree the host retains.

## Opening it

Three ways, and they all set the same flag:

| How | What it is |
| --- | --- |
| **F12** (or Ctrl+Shift+I) | the shortcut, wired once in the platform loop |
| `devtools.show()` / `devtools.hide()` | `show: true` / `show: false` from code |
| `devtools.toggle()` | what the shortcut itself does |
| **Escape** | closes the panel once it is up |

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
    vec![floom::devtools::ReadoutRow {
        label: "visible".into(),
        value: state.visible().to_string(),
    }]
});
```

## Where the panel lives

`floom-devtools` — the same crate as the headless
[`Inspector`](../crates/floom-devtools/src/lib.rs). The inspector answers
questions about a retained tree; the panel is the visible half of the same
crate, a real widget built from engine primitives rather than a second window.

The facade re-exports it under `floom::devtools`, behind the `devtools` feature.

## Try it

```sh
cargo run -p floom-devtools-demo
```

A window with a heading, a row of three buttons and a counter. Press **F12** and
the panel docks over the trailing edge; **Ctrl+Shift+C** arms the pick, and
clicking a button selects it without pressing it. The demo is two lines —
`run_desktop_with_devtools` plus the readout closure — and the whole point of it
is that there is nothing else to write.

The demo needs a GPU window, so CI cannot run it. What CI *does* run is
`crates/floom-app/tests/devtools_demo.rs`, which mounts the same app into a real
host and checks the whole chain headlessly: the tree is describable, the panel
composes the app's rows over the engine's, refreshing the cell picks up new
nodes, and a picking click selects without pressing. If that passes, the demo's
two lines are doing what they claim.

## The Chrome layout

`attach_elements` composes the panels Chrome made standard:

```rust
let devtools = floom::devtools::attach_elements(&mut host);
// ... then, once per frame:
devtools.refresh_tree(&host);
```

| Panel | What it shows |
| --- | --- |
| **Elements** | the element tree, nested by depth, with the selected node marked |
| **Breadcrumb** | `root > body > ok` — the path to the selection |
| **Styles** | the selection's resolved style in every inspected state |
| **Layout** | the selection's box: size, flex, axis, gap, align, inset |
| **Performance** | frame stage breakdown with the budget verdict, allocations, cache hit rates |
| **Console** | what the error boundary caught, newest first |

The tab drives which half is composed in: `devtools.set_tab(DevtoolsTab::Style)`.

## Picking an element

Chrome's signature interaction — press inspect, click the page, see what you
clicked:

```rust
let picker = floom::devtools::attach_picker(&mut host, &panel_state);
// Ctrl+Shift+C arms it; the next click selects, and a box is drawn around it.
```

A click while picking **selects without pressing**: the picker consumes the
event before the tree sees it. The highlight is its own overlay, above the panel,
so the box is visible over everything including the devtools.

## What the panel is *not*

It is not a general-purpose debug overlay. It draws rows of text: whatever your
`readout` closure says, in a docked slab on the trailing edge. The engine's
inspector (`Inspector`, in the same crate) is what turns a tree into those rows,
and it stays headless — a CLI, a test, or a remote protocol client can use it
without ever mounting a panel.

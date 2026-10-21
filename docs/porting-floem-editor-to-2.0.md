# Porting the floom-editor satellite to Floom 2.0

The port of `crates/ecosystem/floom-editor` — the v1-era editor satellite —
onto the Floom 2.0 API. This is both the plan and the record of what landed.

## What the satellite was

Two crates in a nested workspace:

| crate | lines | what it is |
| --- | --- | --- |
| `floom-editor-core` | 8,867 | the model: rope buffer, cursor, selection, modes, registers, movement, word motions, soft tabs, indent |
| `floom-editor` | 11,726 | the views: `Editor`, `EditorView`, `TextEditor`, visual lines, keymap, gutter, phantom text |

## Where the port landed

`floom-editor-core` **moved into the root workspace** as
`crates/floom-editor-core`. It has zero references to the `floom` crate — it is
pure logic over `lapce-xi-rope` — so it needed no code changes at all, only a
manifest. It joins the root workspace's member list and is checked by
`scripts/check-architecture-deps.sh` like every other crate. Its 77 unit tests
and 48 doctests are green there.

`floom-editor` (the views) **stays a satellite** at
`crates/ecosystem/floom-editor/`, because the ecosystem doc names it as an
official satellite and the root workspace's `exclude` list keeps
`crates/ecosystem` out of the core. It is ported onto the 2.0 `Widget` API.

## The engine work this depended on

2.0's `floom-text` was a *shaper*: `TextShaper::shape` produces
`LaidOutText { size, runs, lines }` for one string at one style. It had none of
what the editor needs:

| the editor needs | 2.0 before |
| --- | --- |
| per-range styling (syntax highlighting) | one style for the whole string |
| font family per line | fixed default stack, not selectable |
| hit testing: point → byte offset | absent |
| cursor geometry: byte offset → point | absent |
| selection geometry: byte range → rects | absent |
| wrapped-line enumeration | absent (only `lines` metrics) |
| tab expansion with byte mapping | absent |

So `floom-text` grew an editor-grade layout API, in a new module
`crates/floom-text/src/editor_layout.rs`.

### The vocabulary (L0)

`floom-types` gained the text vocabulary the layout needs, all of it
renderer-neutral:

| type | what it is |
| --- | --- |
| `FontFamily` | the five generic families, or one by name |
| `TextStyle` | size, weight, ink |
| `TextSpan` | a byte range and the style that differs from the base |
| `TextWrap` | `None` or `Word` |
| `StyledText` | the text, the base style, the spans, the family, the wrap, the tab width, the break width |

`StyledText` is *data*, not a shaped result, and that is the whole design: the
same description can be laid out for geometry and painted for ink, and the two
agree because they are the same description.

### The layout (L3)

`floom_text::TextLayout` owns **one** shaping and answers every question about
it:

| method | the question |
| --- | --- |
| `hit_test(point)` | which byte is under the pointer? |
| `cursor_point(offset, affinity)` | where is the caret? |
| `cursor_rect(offset, affinity, width)` | what rectangle does the caret occupy? |
| `selection_rects(range)` | what rectangles does the selection cover? |
| `line_metrics_at(offset, affinity)` | what are the metrics of the caret's line? |
| `visual_line_count` / `visual_line_y` / `line_text_range` | how many lines, where do they start, what do they cover? |
| `runs()` | the shaped runs, **with their ink** |

Two behaviours the v1 layout had are reproduced deliberately, because they are
load-bearing rather than incidental:

* **Ink-tight selection rectangles.** v1 unioned the selected glyphs' *outline*
  bounds into the rectangle (`selection_geometry_with_line_metrics`). A
  selection is painted *behind* the text, so a rectangle that covers the
  advances but not the ink shows the background through the glyphs. The new
  layout measures the outlines with `swash`'s scaler, behind a thread-local
  cache keyed on (blob, face, glyph, size, variation coordinates, skew).
* **Tab expansion with a byte-exact source↔display mapping.** The offsets that
  cross the boundary are *source* offsets — the same coordinates the buffer
  uses — so a caller never has to think about which text it is holding.

The shaper's `Affinity` does not leak: `floom_text::CaretAffinity` is the
engine's own, because a caller that named the shaper's would be reaching past
this crate's surface, and a renderer would have to depend on the shaper to
answer a question about a caret.

### The paint path

`PaintContext` lives in `floom-core` (L1) and cannot name `floom-text` (L3)
types, so the op's arguments have to be L0-safe *data*. The op is:

```rust
fn fill_styled_text(&mut self, origin: Point, styled: &StyledText);
```

The host (`floom-app`) shapes that description with the *same* `floom-text`
code the widget laid out with, groups the runs by their ink, and pushes one
`SceneNode::Text` per group — the scene's text node carries one colour, so a
line with three inks is three nodes over one layout.

**Layout/paint agreement** is the property that has to hold, and it holds
because both sides are handed the same `StyledText` value: the widget's is the
one it laid out with, the host's is the one it is given. The host's cache is
keyed on a hash of the whole description and keeps the description, so a hash
collision degrades to a re-intern rather than to the wrong resource.

## Layering of the ported crates

```
floom-editor-core   (L5, model)      — no framework deps at all
floom-editor        (satellite)       — depends on floom-core, floom-text,
                                       floom-types, floom-widgets
```

## Mapping from the v1 API

| v1 | v2 |
| --- | --- |
| `floom::view::View` (the trait) | `floom_core::Widget` |
| `floom::context::PaintCx` | `floom_core::PaintContext` |
| `floom::event::EventCx` | `floom_core::EventContext` |
| `floom::style::*` + `prop!`/`prop_extractor!` | a plain style struct (`EditorStyle`) — the floom-ui answer |
| `floom::reactive::*` | `floom_signal::*` — the same names, the same traits |
| `floom::text::TextLayout` | the new `floom_text::TextLayout` |
| `floom::action::exec_after` | `floom-task` / the animation scheduler |
| `floom::action::set_ime_*` | `floom-ime` |
| `floom::Clipboard` | `floom-platform`'s `Clipboard` (capability-gated) |
| `floom::headless::TestRoot` | `floom-app`'s `WidgetHost` + the scene |
| `floom::launch` | `floom::run_desktop` |
| `floom::Renderer` (v1 paint trait) | `floom_core::PaintContext` |

## The ported view layer

The v1 view crate was 11,726 lines across thirteen files. The port is a
rewrite onto the 2.0 `Widget` API, in five:

| file | what it is |
| --- | --- |
| `style.rs` | `EditorStyle` — a plain struct, with the light and dark themes |
| `styling.rs` | the `Styling` trait and `SimpleStyling` |
| `document.rs` | the `Document` trait and `TextDocument`, over the editor core's `Buffer` |
| `editor.rs` | the `Editor` widget: the text, the caret, the selection, the gutter, the keyboard, the mouse, the scrolling |
| `text_editor.rs` | the builder |

The editor is a **leaf** widget: it paints the gutter and the text itself,
because both come from the same layout and the same state, and a gutter as a
child would have to be told the same thing the text is told.

The state is behind an `Arc<Mutex<EditorState>>` rather than a signal, for the
same reason `floom-widgets`' text input puts its buffer behind one: a 2.0 slot
closure is `Send + Sync` and a signal is `Rc`-based.

### What is not ported, and where it is named

The gaps are named in `floom-editor`'s `lib.rs` module note, and each one is a
difference rather than a silent omission:

* **Soft wrap.** The engine wraps — `TextWrap::Word` with a `max_width`, and
  `TextLayout::lines()` reports the wrapped visual lines with their source
  ranges — and the editor does not use it yet. A long line runs past the right
  edge. Landing it means the scroll position, the gutter and the "which line
  is the caret on" question all have to agree that a source line can be
  several lines on screen.
* **Multiple cursors.** One selection, one caret.
* **Modal editing.** v1 had a modal mode with its own keymap.
* **Phantom text, IME preedit, indent guides, visible whitespace.** v1 drew
  all four.
* **Per-line styling.** v1's `Styling` answered per line; this one answers per
  document, because the line height has to be the same on every line for the
  scroll position to mean anything.
* **The command system.** v1 had a `Command` enum, a registry and a keymap
  that dispatched into it. The port's keyboard handling is direct.

And one *host* rule, which is not a gap in the port but a limit in the
framework: **the host consumes Tab for focus navigation before any widget
sees it**, so an editor's indent-on-Tab cannot fire through `dispatch`.

## Examples

Two, both on `floom::run_desktop`:

* `examples/editor` — the editor filling a window, with a "clear" button
  underneath that reaches into the state from outside the widget.
* `examples/syntax-editor` — the editor with a hand-written keyword and string
  highlighter implementing the port's `Styling` trait. The v1 example used
  `syntect`; this one uses a small scanner, because what the example is here
  to show is the *trait*, and a real highlighter would implement the same one.

## Tests

* `floom-text`: 20 unit tests over the new layout — the line count, the hit
  test, the caret's movement, the selection's rectangles, a span's ink, the
  wrap modes, the tab mapping, the visual line's source range.
* `floom-editor`: eight **paint-level** tests that mount the editor in a
  `WidgetHost`, frame it, and read the **scene** — the text is painted, the
  caret is painted in the caret's colour, a line with two inks is two text
  nodes, a keystroke reaches the document, a shifted letter is typed in upper
  case, a click places the caret, the editor scrolls to keep the caret on
  screen, and a selection is painted in the selection's colour.

The paint-level discipline is the point: a widget that compiles and does
nothing passes every test that looks at its own state, and so does a widget
that draws the wrong thing. Only the scene tells the two apart.

It also earns its keep in a way a unit test cannot: the *click* test hung,
and the hang was a real bug — the hit-test path held the state mutex across
the call that took it again. A mutex is not reentrant, so the test hung
rather than failed. The fix is to scope the guard, and the lesson is that a
widget whose state is behind a lock has a lock-ordering contract that only
running the thing can show.

## What is *not* in scope

* The `floom-ui` satellite's kits — untouched.
* `floom-editor-core`'s semantics — untouched. It is already 2.0-clean.

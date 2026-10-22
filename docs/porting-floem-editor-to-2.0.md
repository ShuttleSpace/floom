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
| `StyledText` | the text, the base style, the spans, the family, the wrap, the tab width, the break width, the line height |

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
| `row_pitch()` / `row_of_offset()` | how far apart the rows are, and which row a byte is on |
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

The gaps are named in `floom-editor`'s `lib.rs` module note, and each one is
a difference rather than a silent omission.

The first round of the port closed three of them, and the second round closed
three more:

| the gap | how it landed |
| --- | --- |
| **Soft wrap** | the editor's line model became a *row* model: a source line is one or more rows, the engine breaks the line, and the gutter, the caret, the selection and the scroll all count rows |
| **Multiple cursors** | the selection is the editor core's own multi-region `Selection`, and every edit applies to all of its regions |
| **Visible whitespace** | a space is a dot in the middle of its advance, a tab is a line across its width |
| **Indent guides** | a vertical line at each tab stop the line's indentation reaches |
| **Per-line styling** | already per line: the trait's `style_line` is handed the line's number and the line's text |
| **The command system** | still not ported: the keyboard handling is direct |

What is *still* not ported:

* **Modal editing.** v1 had a modal mode with its own keymap.
* **Phantom text and IME preedit.** v1 drew both. The preedit needs the
  host's composition, and the host does not hand one to a widget yet — so the
  editor cannot draw what it is never told. The phantom text is a drawing
  feature the editor could implement on its own, and has not.

And one *host* rule, which is not a gap in the port but a limit in the
framework: **the host consumes Tab for focus navigation before any widget
sees it**, so an editor's indent-on-Tab cannot fire through `dispatch`.

### The row model

Soft wrap is the change that reaches furthest, because it changes what a
line *is*.

Before, a line was a line: the scroll offset was a number of lines, the
gutter counted lines, and the caret was on a line. A line that was too long
ran past the edge, which was the honest way to say "the editor does not wrap
yet" without pretending otherwise.

Now a source line is one or more **rows**. The engine breaks the line, and
the widget asks the layout how many rows it took. Three things have to agree
for that to work, and all three are the engine's answers rather than the
widget's guesses:

* **The row height.** The engine is told the height — `StyledText` grew a
  `line_height` field, and the shaper's line height makes the rows exactly
  that far apart. The widget does not measure the rows it did not lay out; it
  counts them.
* **The caret's row.** `TextLayout::row_of_offset` answers which row a byte
  is on, so the caret is drawn on the row the byte is on rather than on the
  line's first row.
* **The selection's rows.** The selection rectangles come back per row, and
  each is placed on the row it belongs to.

The scroll is anchored on a *source line* rather than on a pixel offset into
the document, which is what keeps the whole thing cheap: scrolling forward
is one line of layout, and scrolling back is one line of layout. A pixel
offset would have needed the height of every line above the viewport, which
is a whole document of layout for a number that moves a little at a time.

### Multiple cursors

The selection was a single region and is now the editor core's own
multi-region `Selection`. The core already knew how to edit several regions
at once — `Buffer::edit` takes a list of them and applies them as one delta
— so the model needed no change at all; only the *view* had to stop
assuming there was one region.

The parts that had to be thought about:

* **Where the carets end up.** An edit at one caret shifts the text at every
  caret after it, so a caret is not "its region's start plus the content's
  length" but that plus every earlier region's change. The arithmetic is in
  `caret_positions`.
* **What an arrow key does.** It moves *every* caret, by the same rule
  rather than the same distance: two carets on different lines both go up a
  line and both land on their own columns.
* **What the selection does when it moves.** It collapses, because that is
  what an arrow key does — a selection is a thing to be left behind, not
  dragged along.

### The two offset bugs this work found

The port's first round had two bugs that its tests could not see, because
every one of them used a single-line document.

The layout knows only *its own* line: its offsets are offsets into that
line's text. The editor was handing it *document* offsets — the selection's
range, and the caret's position — without translating them into the line's
coordinates. On the first line the two are the same number, which is why
nothing looked wrong; on any other line the caret was drawn at the end of the
line and a click landed on the first line of the document.

Both are fixed, and both have a test that fails without the fix — which was
checked by reverting the fix and watching the two tests go red.

## Examples

Two, both on `floom::run_desktop`:

* `examples/editor` — the editor filling a window, with a "clear" button
  underneath that reaches into the state from outside the widget.
* `examples/syntax-editor` — the editor with a hand-written keyword and string
  highlighter implementing the port's `Styling` trait. The v1 example used
  `syntect`; this one uses a small scanner, because what the example is here
  to show is the *trait*, and a real highlighter would implement the same one.

## Tests

* `floom-text`: 24 unit tests over the new layout — the line count, the hit
  test, the caret's movement, the selection's rectangles, a span's ink, the
  wrap modes, the tab mapping, the visual line's source range, the row pitch,
  and the row a byte is on.
* `floom-editor`: seventeen **paint-level** tests that mount the editor in a
  `WidgetHost`, frame it, and read the **scene** — the text is painted, the
  caret is painted in the caret's colour, a line with two inks is two text
  nodes, a keystroke reaches the document, a shifted letter is typed in upper
  case, a click places the caret, the editor scrolls to keep the caret on
  screen, a selection is painted in the selection's colour, a wrapped line is
  numbered once and its caret is on its own row, two carets are two carets,
  typing with two carets types at both, alt and a click adds a caret, the
  whitespace and the indent guides are drawn, and a click on the second line
  puts the caret on the second line.

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

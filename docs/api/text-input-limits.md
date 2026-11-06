<!-- SPDX-License-Identifier: MIT -->
<!-- SPDX-FileCopyrightText: The Floem Authors -->
# The text input: shape and known limits

`combobox` and `autocomplete` both build on a text input, so it is the first
*primitive* in the port that is not a port of anything.

## Why the buffer is an `Arc<Mutex<String>>`

A text input buffer cannot be an `RwSignal<String>`: `RwSignal` is `Rc`-based and
a slot closure must be `Send + Sync`, for the same reason `md-dialog` cannot own
its `open` signal. The buffer is an `Arc<Mutex<String>>`: `Send + Sync`, so it can
be captured by a builder, and *shared*, so a caller can still hold a handle and
read or write it. It is the same move as `TreeView`'s forest, and for the same
reason.

So:

- `TextInput::buffer` hands out the handle, and `TextInput::text` reads it — a
  caller does not need a signal to bind to, and can use the same handle from two
  places;
- `TextInput::on_change` fires when the buffer differs from what the widget last
  saw, which covers **both** directions: a keystroke and an external write. The
  external write is not delivered by a signal, so it has to be noticed; that is
  what the widget's `update` is for.

## Two limits, recorded rather than worked around

- **A click cannot place the caret.** `EventContext` carries `cursor_position`
  and `bounds` but no text measurement, so there is no way to turn an x
  coordinate into a byte offset. A click therefore focuses the field and puts the
  caret at the end. Everything else — typing, backspace, delete, arrows,
  Home/End, Enter — works from the caret's current position.
- **The text is not clipped to the box.** `PaintContext` has no clip op, although
  the *host* already has the scene operation (`write_clip_push`, and
  `resolve_clip` in `floom-types`), so this is a missing method rather than a
  missing capability. A field narrower than its text will paint the text past its
  own edge.

Both should be fixed before a text input grows selection handles.

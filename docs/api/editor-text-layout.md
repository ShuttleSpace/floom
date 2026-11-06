<!-- SPDX-License-Identifier: MIT -->
<!-- SPDX-FileCopyrightText: The Floem Authors -->
# Editor-grade text layout

`floom_text::editor_layout` exists because a text editor needs more than "shape
this string". It needs to answer:

- where is the caret when the cursor is at byte *n*?
- which byte is under the pointer?
- what rectangle does the selection cover?
- how many visual lines does this paragraph occupy, and where do they start?

All four questions are about the *same* layout, so they have to come from the
same object. Two layouts — one for the geometry, one for the paint — would
drift, and the drift shows up as a caret that sits between glyphs.

So `TextLayout` is that single object: it owns the shaped result and answers
every question about it.

## Agreement with the painted result

The editor paints the text it laid out. If the host shaped the text *again*, on
its own, the two shapings could differ — a different font fallback, a different
tab expansion — and the caret would land somewhere the pixels are not.

The layout carries the *shaped* result, and the paint path paints exactly that
result rather than re-shaping a string. `TextLayout::runs` hands out the runs
that were shaped; the host interns them and paints them.

## Styled ranges

A range of text can differ from the rest: syntax highlighting bolds a keyword,
an error underlines a span. `floom_types::StyledText` describes that as *data* —
the text, the base style, and the ranges that differ — and this module turns it
into a layout.

The description is deliberately data rather than a shaped result, because the
same description is what the *paint* path is given. See
`floom_types::StyledText`.

## What changed from the 1.x layout

The 1.x `TextLayout` was a mutable bag of setters (`set_text`, `set_size`,
`set_tab_width`, …) that re-shaped on every call, and it leaked Parley's
`Cursor` and `Selection` into its callers. This one is a *value*:
`TextLayout::new` takes a `floom_types::StyledText` and returns a layout that is
already shaped, and a change of text is a new layout. The geometry and the runs
then belong together, because there is only one of each.

The offsets crossing the boundary are *source* byte offsets — the same
coordinates the buffer uses — so a caller never has to think about which text it
is holding.

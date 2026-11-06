<!-- SPDX-License-Identifier: MIT -->
<!-- SPDX-FileCopyrightText: The Floem Authors -->
# Devtools element-tree rows

`floom_devtools::tree` draws the Elements tree as rows plus a context menu.

## Why a row widget and not `floom-ui-composites::tree_view`

The satellite workspace has a `TreeView`, and it is not usable here for two
structural reasons. It lives under `crates/ecosystem`, which the root manifest
excludes and which is a nested workspace of its own — depending on it would make
this crate pull a whole design-system workspace in. More importantly its rows are
a *presentation* of a tree the caller owns; here the tree is the live host's, and
a row has to write back into the devtools state (collapse, select) rather than
report a selection up a callback chain.

## What a row is

A row paints itself rather than composing `Text`s, for one reason: the selected
row needs a filled background *behind* its label, and this host paints a flat
scene in node order, so a sibling box cannot be drawn behind another sibling's
text. `fill_rect` then `fill_text` inside one widget is the only ordering
available.

## What a row does not do

**No hover highlight.** The host never synthesises `PointerPhase::Leave` — there
is no such event in `floom-app`, `floom-runtime-core` or
`floom-platform-desktop` — so a row that painted itself on hover would stay
highlighted forever after the pointer left it, and un-highlighting the *previous*
row is not something a widget can ask for: `request_repaint` names one element,
and the previous row is a different element. The selection background is the only
state a row shows, and it is honest: it comes from the row's own data.

## Why the menu is one widget and not a stack of `Button`s

A `Button` reports its click through a callback that fires on *release* and writes
nothing itself, so a menu built from buttons would need the panel to notice the
write and rebuild before the menu could close — two frames for one click, and a
menu that lingers over the row it was about. The menu instead resolves the entry
from the press position, applies it to the shared state, and closes its own
overlay, all inside one dispatch.

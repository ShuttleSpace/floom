<!-- SPDX-License-Identifier: MIT -->
<!-- SPDX-FileCopyrightText: The Floem Authors -->
# `Surface`: a decorated box with children

`Surface` is the spelling of `Container::new(child).style(..)` — the most common
shape in floem-ui (165 `Container::new` uses, nearly all of them wrapping
something).

## Why a separate type rather than a child on `Container`

2.0's model is that composition is declared by the parent
(`Child::new(key, LayoutRequest, ..)`) and that leaves are leaves. `Container`
is the *decoration description*; `Stack` is the *structure description*; and
`Surface` is the one thing that is both. Keeping `Container` a leaf means its
invalidation model stays the one every other widget uses, while `Surface` adds
exactly the thing a leaf cannot have: content inside the decoration.

## Padding lives here, not on `Container`

A leaf has nothing to inset, which is why `Container` once carried a `padding`
field that was stored and never read, and why `md-card` rendered with its text
against the edge while its test passed. `Surface` is a node with children, so
its `padding` is real: it goes out as [`LayoutRequest::padding`] and insets the
children inside the box.

A margin would *almost* work — with one child the two are geometrically
identical — and "almost" is why they are not interchangeable: a margin is per
child and lands between children as well, while a padding is one inset around
the group. `floom-layout`'s
`a_margin_is_per_child_and_a_padding_is_around_the_group` is the measurement.

## One revision, owned here

`Surface` keeps its own `Revision` rather than forwarding the decoration's, and
bumps it from every builder and setter that touches the decoration. That way a
frame reports dirty at most once per node however many of its fields moved,
which is the same single-report discipline every other widget keeps. The
consequence is that `Self::decoration` hands out a *shared* reference: the
decoration is not reachable mutably from outside, so it cannot be changed
without being invalidated.

## Theme-agnostic

Like every 2.0 widget it takes resolved values; deciding which palette is the
kit's job. See `docs/porting-floem-ui-to-2.0.md`.

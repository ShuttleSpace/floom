<!-- SPDX-License-Identifier: MIT -->
<!-- SPDX-FileCopyrightText: The Floem Authors -->
# `CircularProgress`: why it is mostly parameters

Measured across `md_progress_circular`, `cupertino_progress_circular` and
`fluent_progress_circular`: the three agree on the tokens (`size`, and
`stroke_width` in all but the Cupertino override) and on both colours
(`surface_variant` track, `primary` fill), and disagree about the shape of the
*indeterminate* arc and where it starts. So this is not one component with three
skins, and the module is mostly a list of the ways they differ.

## The disagreements, as parameters rather than as prose

| | Material | Cupertino | Fluent |
|---|---|---|---|
| indeterminate arc | **30%, rotating** | 25%, static | 25%, static |
| indeterminate start | 12 o'clock | 12 o'clock | **3 o'clock** |
| determinate start | 12 o'clock | 12 o'clock | 12 o'clock |

Each row is a field on `CircularProgressStyle`, and each is pinned by a test in
the kit that wants it.

## The rotation is the caller's, not this crate's

Material's indeterminate arc rotates continuously. This widget does not own a
clock, so the rotation is a value the caller sets through
`CircularProgress::set_rotation`.

Nothing inside this crate advances it, for the same reason
`crate::progress_linear` does not advance its sliding segment: the driver belongs
at the host, not here. Until one exists, Material's ring sits at twelve o'clock,
which is Cupertino's and Fluent's behaviour — and is visibly wrong for Material,
in the same way and for the same reason.

## The radius

Cupertino wrote `r = (size - sw).max(1.0) / 2.0` and the other two wrote
`r = size / 2.0 - sw / 2.0`. The two agree for any ring wider than its stroke,
which is every ring a person sees; they differ only for a ring narrower than its
own stroke, where the other two go negative and the engine's `ellipse_arc` takes
the absolute value. The clamped form is used here, so the degenerate case is a
hairline rather than an inverted ring.

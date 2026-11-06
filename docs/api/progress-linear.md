<!-- SPDX-License-Identifier: MIT -->
<!-- SPDX-FileCopyrightText: The Floem Authors -->
# `LinearProgress`: why it is mostly parameters

Measured across `md_progress_linear`, `cupertino_progress_linear` and
`fluent_progress_linear`: **0.17 and 0.19 similarity** — the lowest of any shape
ported so far. The three agree on the tokens (`track_height` 4.0,
`border_radius` 2.0, one shared table) and on both colours
(`surface_variant` track, `primary` fill), and disagree about almost everything
else. So this is *not* one component with three skins, and the module is mostly
a list of the ways they differ.

## The disagreements, as parameters rather than as prose

| | Material | Cupertino | Fluent |
|---|---|---|---|
| fill placement | absolute overlay | absolute overlay, insets spelled out | **flow child, centred** |
| fill corner radius | **none** | track radius | track radius |
| value → percent | exact | **truncated to whole percent** | exact |
| indeterminate | **sliding 40% segment** | static 40% bar | static 40% segment |

Each row is a field on `LinearProgressStyle` or an argument, and each is pinned
by a test in the kit that wants it. Nothing here is "resolved".

## Two places the kits *look* like they disagree and do not

* **Fill height.** Material writes `height_pct(100.0)` on an absolutely
  positioned fill; Cupertino writes `height(track_height)`. The track's height
  *is* `track_height` in both, so the two spellings cannot produce different
  geometry. One parameter, and `the_fill_is_as_tall_as_the_track` is the test
  that says so — a claim about layout, made falsifiable rather than asserted.
* **Track width.** Material writes `width_full()`, the other two
  `width_pct(100.0)`. Same box.

## The sliding segment has no driver, and that is recorded rather than faked

Material's indeterminate bar used an off-thread thread that slept 16 ms and
wrote an `ArcRwSignal`. That is not portable to 2.0 and should not be: a widget
that mutates shared state off-thread is exactly the shape 2.0's
`Arc<Mutex<..>>` state exists to avoid.

So `Indeterminate::SlidingSegment` declares the geometry — a segment of
`width_pct` travelling across `travel_pct` — and the caller supplies the offset
through `LinearProgress::set_fill_offset_pct`. Nothing inside the widgets crate
advances it. `floom-app` does have a clock (`WidgetHost::tick_animations`), so
the driver is a small piece of work that belongs at the host, not here; until it
exists, Material's indeterminate bar sits still at the left edge, which is
Cupertino's and Fluent's behaviour and is visibly wrong for Material.

## One token reads one way here and another

The documentation is the outlier, and it is left alone — fixing it would change
a published token's meaning, and the disagreement that matters is Material's
behaviour, which is preserved.

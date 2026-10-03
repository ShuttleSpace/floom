# Renderer backend support matrix

What each renderer backend actually answers, and what it silently does not.

This exists because "a backend without a blur primitive can still draw the
offset silhouette and count what it could not do — *the degradation is
recorded, never silent*" is a stated design principle (`floom-types::Shadow`),
and a principle with no ledger decays. This file is the ledger.

## The contract

`PaintContext::draw_shadow` has a default no-op body:

```rust
fn draw_shadow(&mut self, rect: Rect, radius: f32, shadow: &Shadow) {
    let _ = (rect, radius, shadow);
}
```

So a backend that does not override it **drops every shadow request without
counting it**. That is the silent degradation the principle forbids, and it is
the gap this file tracks. `Filter` (blur / backdrop blur) has the same shape
and is tracked the same way below.

## Status

`spread` is CSS's fourth `box-shadow` length and dilates the silhouette
*before* the blur. Implementations must do that, not widen the blur kernel —
a widened blur reads as "a fatter, softer shadow", which is a different effect
and the most likely wrong implementation.

| Backend | `draw_shadow` | Shadow rendered | `spread` honoured | Degradation counted | TODO |
|---|---|---|---|---|---|
| `floom-renderer-software` | yes | yes — SDF silhouette, 3× box blur, per-pixel | **yes** | `shadows_drawn` | reference implementation |
| `floom-renderer-vello` | no (GPU path only) | **no** — fill renders, shadow does not | n/a | `shadows_skipped` | needs a blur primitive in vello; must dilate, then blur |
| `floom-renderer-skia` | no | no | n/a | **no** | must implement, and add a counter |
| `floom-renderer-web` | no | no | n/a | **no** | must implement (canvas/WebGPU shadow), and add a counter |
| `floom-renderer-tui` | no | no | n/a | **no** | decide: drop, or render as a hard silhouette |

### Per-backend notes

**`floom-renderer-vello`** — the closest to correct today, and still not
correct. `gpu.rs` renders the node's fill, finds no blur primitive in vello, and
increments `stats.shadow_skips`, which surfaces as `shadows_skipped`. That
satisfies "never silent". It renders *no* shadow at all. When vello grows a blur
primitive, the implementation must dilate the silhouette by `spread` first and
its render-cache key must include it (it does as of the `spread` change — the
key omitted `spread` initially, which would have let two nodes differing only in
spread collide in the cache).

**`floom-renderer-skia`** — drops shadows with no counter. Skia has native
elevation/blur support, so this is the cheapest of the three to fix, but it
currently fails the "never silent" rule.

**`floom-renderer-web`** — drops shadows with no counter. The web backend needs
a decision rather than an implementation: a canvas 2D context has
`shadowBlur`/`shadowOffset` but **no spread**, so a faithful `spread` needs a
WebGPU or filter path, and the canvas fallback must report the truncation.

**`floom-renderer-tui`** — a character grid has no soft edges. The honest
options are to drop shadows deliberately (and say so in the docs rather than
leaving it as an accident of a default method body), or to approximate with a
hard offset silhouette. It should not attempt a blur.

## Rules for whoever implements these

1. **Dilate, do not blur-alias.** Apply `spread` to the silhouette before the
   blur; see `Shadow::spread` and the software backend's SDF.
2. **Account for it in `reach()`.** A damage region built from
   `Shadow::reach()` must contain a dilated shadow. Negative spread is clamped
   at zero there because erosion cannot extend reach.
3. **Count what you could not do.** Add a `shadows_skipped`-style counter and
   surface it, rather than overwriting the default no-op body.
4. **Test against the software backend.** The software renderer answers per
   pixel and is the reference; `crates/floom-renderer-software/tests/` holds
   the pixel-level discipline to copy.

## Also unrendered

Not shadows, but the same failure mode — recorded here so the pattern is
visible:

| Feature | Contract | Backends answering | Gap |
|---|---|---|---|
| `Filter::Blur` | `PaintContext` filter hooks | software (via `backdrop_filter`/`filter`) | same as above: default no-op elsewhere |
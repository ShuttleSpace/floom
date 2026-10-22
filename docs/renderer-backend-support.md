# Renderer backend support matrix

What each renderer backend actually answers, and what it silently does not.

This exists because "a backend without a blur primitive can still draw the
offset silhouette and count what it could not do — *the degradation is
recorded, never silent*" is a stated design principle (`floom-types::Shadow`),
and a principle with no ledger decays. This file is the ledger.

## The rule

A request a backend cannot honour is **counted**, and the count is surfaced
through the backend's stats. A silent skip reads as a feature that quietly
does nothing, and there is no way to tell that apart from a feature that was
never asked for.

Every backend has a `*Stats` struct with the counters below. The names match
across backends on purpose: a caller reads one number the same way whichever
backend produced it.

## The vocabulary

The 2.0 scene vocabulary is `floom-scene`'s `SceneNode` — twelve variants:

| Variant | What it asks for |
|---|---|
| `GroupPush` / `GroupPop` | a transform scope |
| `Rect { bounds, fill, radius, shadow }` | a shape, optionally rounded, optionally shadowed |
| `Path { path, fill, stroke }` | a vector path, filled and/or stroked |
| `ClipPush` / `ClipPop` | a clip region |
| `FilterPush` / `FilterPop` | a filter scope |
| `BackdropFilter { rect, filter }` | a filter over the pixels already drawn |
| `Shader { bounds, shader }` | a custom WGSL program |
| `Image { bounds, image, alpha }` | a raster image |
| `Text { origin, size, glyphs, color }` | laid-out text |

## The matrix

| Backend | Renders pixels? | Rounded corners | Drop shadow | Filter scope | Backdrop filter | Custom shader | Pixel-level tests |
|---|---|---|---|---|---|---|---|
| `floom-renderer-software` | **yes** | **yes** | **yes**, honouring `spread` | **yes** | **yes** | ⏭️ counted | **yes** |
| `floom-renderer-vello` | **yes** (GPU) | **yes** | ⏭️ counted | ⏭️ counted | ⏭️ counted | **yes** | no |
| `floom-renderer-web` | **yes** (CPU, blitted to a `<canvas>`) | **yes** | **yes** | **yes** | **yes** | ⏭️ counted | no |
| `floom-renderer-tui` | **yes** (character cells) | ⏭️ counted | ⏭️ counted | ⏭️ counted | ⏭️ counted | ⏭️ counted | no |
| `floom-renderer-skia-abi` | **no** — it records | ⏭️ counted | ⏭️ counted | ⏭️ counted | ⏭️ counted | ⏭️ counted | n/a |

Legend: **yes** = rendered · ⏭️ counted = skipped, and the count says so.

## Per-backend notes

### `floom-renderer-software` — the reference

The only backend that answers per pixel, and the one every other backend is
checked against. It renders rounded corners, drop shadows (with `spread`
dilating the silhouette *before* the blur, as CSS does), filter scopes,
backdrop filters, images and text.

It has no shader runtime, so a `Shader` node is skipped and counted
(`shaders_skipped`). A WGSL program needs a GPU runtime, and the software
rasteriser is not one.

Its pixel-level tests (`crates/floom-renderer-software/tests/`) are the
discipline the other backends copy.

### `floom-renderer-vello` — the GPU path, and its one real gap

The default desktop and Android backend. It renders shapes, paths, images,
text and custom shaders.

It has **no blur primitive**, so three requests are degraded:

- a drop shadow is not drawn (`shadows_skipped`);
- a filter scope draws its content sharp rather than filtered
  (`filters_skipped`);
- a backdrop filter draws nothing (`filters_skipped`).

All three are counted, so the degradation is a number rather than an absence.
The fix is upstream: when vello grows a blur primitive, the shadow must
**dilate the silhouette by `spread` first** and then blur it, and the
render-cache key must include `spread` (it does).

### `floom-renderer-web` — the software path, blitted

The web presenter rasterises on the CPU through `SoftwareRenderer` and blits
the buffer into a `<canvas>`, so it inherits the software backend's coverage
exactly: rounded corners, drop shadows with `spread`, filter scopes and
backdrop filters all work.

What it does not have is a shader runtime, so a `Shader` node is skipped and
counted, the same as on the software path.

### `floom-renderer-tui` — a character grid

The narrowest backend, and honest about it. It renders solid-filled rects and
text; everything else is skipped and counted.

Two of the degradations are worth naming because they used to be **silent**:

- a **rounded corner** and a **drop shadow** are both impossible on a
  character grid, and both were swallowed by a `..` in the match arm. They are
  counted now, so a card that lost its shadow is a number rather than an
  absence.

The clip is tracked as a stack of rects, because a pushed region applies to
everything drawn after the marker. A path clip is approximated by its declared
extent, which is conservative in the safe direction: a glyph inside the
bounds but outside the path is still drawn.

### `floom-renderer-skia-abi` — a recorder, not a renderer

This one links no Skia, opens no surface and puts no pixel on a screen. It
turns a scene into **Skia-shaped commands** — `canvas.save()`,
`canvas.concat(matrix)`, `canvas.clipPath` — and counts what a submit path
would have to degrade.

It was called `floom-renderer-skia` until 2026-10-22, and that name was a
backend-shaped lie: a crate that sits among the backends and draws nothing
is one. The rename says what it is, so nobody reaches for it expecting
pixels.

The commands are already in Skia's own vocabulary, so a real submit path is a
translation rather than a redesign, and the contract tests in the crate pin
that vocabulary.

## The rules for whoever implements a submit path

1. **Dilate, do not blur-alias.** Apply `spread` to the silhouette *before*
   the blur; see `Shadow::spread` and the software backend's SDF. Folding it
   into the blur instead is a different, and much rarer, effect.
2. **Account for it in `reach()`.** A damage region built from
   `Shadow::reach()` must contain a dilated shadow. Negative spread is
   clamped at zero there, because erosion cannot extend reach.
3. **Count what you could not do.** Add a `*_skipped` counter and surface it,
   rather than overwriting a default no-op body.
4. **Test against the software backend.** It answers per pixel and is the
   reference; `crates/floom-renderer-software/tests/` holds the discipline to
   copy.

## The gaps that remain

- **vello's blur.** The GPU path has no shadow, no filter scope and no
  backdrop filter. Counting is not rendering; a card with a shadow looks
  flat on the desktop and on Android until vello grows a blur primitive or
  the software path is used as a fallback.
- **Pixel-level tests for the GPU path.** The web, the terminal and the
  software backend have pixel evidence now — the web compares itself against
  the reference buffer pixel for pixel, the terminal's golden is its
  character grid as text, and the software backend has text evidence that
  the ink is a glyph *shape* rather than a filled box. The vello GPU path
  has none, because producing a comparable buffer from it needs a GPU
  context, and CI has none.
- **The CJK case is only covered where the font is.** `floom-text` asserts
  that CJK does not shape to `.notdef`, and that test is macOS-gated
  because it needs a font that is installed — an environment property
  rather than a code one. The rendering-level evidence is therefore Latin,
  and the CJK evidence stays where the font is known to exist.
- **One vocabulary for the counters.** The capability query is in place
  (`Renderer::capabilities` and `Presenter::capabilities`), and a host can
  ask before it draws. The *counters* are still spelled slightly differently
  between backends, so a caller that wants to report "how much was dropped"
  across backends has to know which one it is holding.

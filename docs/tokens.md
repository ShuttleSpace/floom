# Tokens

Where a colour, a size or a motion curve is allowed to come from, in the order a
component is supposed to look. The one-line version: **a component may read any
token and may not invent a value; a per-instance change is a prop, not a token.**

This matters for agents more than for people. Asked to "make the button bigger",
the cheapest edit is a literal, and a literal is how 82 hard-coded colours
accumulated across the three design systems before `just gate style` existed.

## The four layers

| Layer | Lives in | Who may write it | What it is |
|---|---|---|---|
| 1. Global | `floom-theme` (`ColorTokens`, `TypographyTokens`, `SpacingTokens`) | the theme, once per app | the seven colour roles every design system agrees on: `background`, `surface`, `primary`, `on_primary`, `danger`, `text`, `text_muted` |
| 2. Design system | `floom-{material,cupertino,fluent}/src/tokens.rs` | the design-system crate | the roles the shared struct has no slot for — Material carries nineteen colour roles; `INFO`/`SUCCESS`/`WARNING` are the pair-plus-one no system agrees on; `COLOR_PICKER_DEFAULT` |
| 3. Component | `floom-core-theme/src/component_tokens/<widget>.rs` | the design-system crate | per-widget metrics derived from layer 1–2 (`ButtonTokens::from_spacing(&SpacingTokens)`), never a fresh literal |
| 4. Instance | the widget's own props | the app | a deliberate, local override — `Button::style(...)`, not a new token |

A component reads down this list and writes nowhere. If a design system needs a
value the layers above cannot express, the value belongs in that system's
`tokens.rs` (layer 2), where it can be compared with the other two systems,
rather than in the widget that happened to need it first.

## What changing a token does

| Change | Effect | Safe? |
|---|---|---|
| layer 1 colour role | every widget in every design system follows | yes — this is the intended seam |
| layer 2 role | that system only; the other two keep their own value for the same role | yes, but it is now a divergence someone can see in diffs |
| layer 3 metric | every instance of that widget in that system | yes |
| a literal inside a widget | nothing else moves, and the next theme change misses it | no — `just gate style` fails the build |

## How this is enforced, and how it is not

- `bash scripts/gate.sh style` (`scripts/check-design-tokens.sh`) fails on
  `Color::rgb/rgba/srgb/srgb_u8` anywhere in the three systems' `src/widgets/`.
  Proven to fail by injection, in the shape the rest of the gates are.
- The status colours exist once per system (`crates/ecosystem/floom-ui/crates/floom-{material,cupertino,fluent}/src/tokens.rs`)
  rather than once per widget: the Fluent banner and alert used to disagree on
  what warning yellow was (`#F7CE00` vs `#FFB900`), and a test now asserts they
  are equal.
- **Not enforced**: nothing checks that a component reads layer 3 instead of
  reaching into layer 2 directly, and nothing checks that `tokens.rs` has no
  unused constants. Both need a real token graph rather than a grep, and neither
  is scheduled.

## Related

- [components.md](components.md) — which components exist and the ten contract
  rules they share.
- [vision.md](vision.md) — the design-system position: three systems,
  one engine, no fourth.
- `crates/ecosystem/floom-ui/crates/floom-core-theme/src/component_tokens/` —
  the per-widget tables layer 3 is made of.

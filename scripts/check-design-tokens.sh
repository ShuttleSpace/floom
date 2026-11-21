#!/usr/bin/env bash
# Every colour a design-system widget paints must come from a token.
#
# The widgets take a theme and read roles out of it; a `Color::rgb(33, 150, 243)`
# in a widget body is a colour that ignores the theme, the dark scheme, and the
# other two design systems. The palette itself is not banned — it lives in each
# crate's `tokens.rs`, one copy per role, which is what this script is protecting.
set -euo pipefail

cd "$(dirname "$0")/.."

# `Color::TRANSPARENT` and `Color::BLACK` are constants, not literals, and the
# pattern below only matches the constructors that take channel values.
pattern='(Color|PaintColor|ThemeColor)::(rgb|rgba|srgb|srgb_u8)\('

hits=$(grep -rnE "$pattern" \
  crates/ecosystem/floom-ui/crates/floom-material/src/widgets \
  crates/ecosystem/floom-ui/crates/floom-cupertino/src/widgets \
  crates/ecosystem/floom-ui/crates/floom-fluent/src/widgets || true)

if [ -n "$hits" ]; then
  echo "design tokens: a widget paints a colour literal instead of a token:" >&2
  echo "$hits" >&2
  echo "" >&2
  echo "move the value to the crate's src/tokens.rs and read it from there" >&2
  exit 1
fi

echo "design tokens: no colour literals in the three widget trees"

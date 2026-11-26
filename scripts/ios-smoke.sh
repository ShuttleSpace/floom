#!/usr/bin/env bash
# Scaffold an app, build it for the iOS simulator, run it, and prove it drew.
#
# This is the mobile half of the "tested" claim in docs/release-policy.md: a
# target is tested when something runs on it, not when it cross-compiles. The
# assertion is a pixel count, because a screenshot is the only thing an app that
# renders nothing cannot fake — and it needs no accessibility client.
set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
WORK="${WORK:-$(mktemp -d)}"
NAME="${NAME:-smoke}"

say() { printf '\n=== %s\n' "$*"; }

say "scaffolding $NAME in $WORK"
mkdir -p "$WORK"
cd "$WORK"
rm -rf "$NAME"
"$ROOT/target/debug/floom" create "$NAME" --platforms ios --floom-path "$ROOT/crates/app/floom"
cd "$NAME"

say "packaging for the simulator"
"$ROOT/target/debug/floom" package --platform ios

APP="$WORK/$NAME/ios/build/$NAME.app"
test -d "$APP" || { echo "no app bundle at $APP" >&2; exit 1; }

say "picking a simulator"
UDID=$(xcrun simctl list devices available -j \
  | python3 -c 'import json,sys
devices = json.load(sys.stdin)["devices"]
for runtime in sorted(devices, reverse=True):
    for device in devices[runtime]:
        if device["name"].startswith("iPhone"):
            print(device["udid"]); raise SystemExit')
test -n "$UDID" || { echo "no iPhone simulator is installed in this Xcode" >&2; exit 1; }
xcrun simctl boot "$UDID" 2>/dev/null || true
xcrun simctl bootstatus "$UDID" -b

say "installing and launching"
BUNDLE=$(/usr/libexec/PlistBuddy -c 'Print :CFBundleIdentifier' "$APP/Info.plist")
xcrun simctl install "$UDID" "$APP"
SIMCTL_CHILD_FLOOM_GLYPH_REPORT=1 xcrun simctl launch "$UDID" "$BUNDLE"
sleep 4

say "screenshot"
SHOT="$WORK/$NAME.png"
xcrun simctl io "$UDID" screenshot "$SHOT"

say "counting what the app drew"
# `sips` turns the PNG into a BMP, whose header is 14 bytes of file header plus a
# 40-byte DIB header: no image library needed to read pixels.
BMP="$WORK/$NAME.bmp"
sips -s format bmp "$SHOT" --out "$BMP" >/dev/null
python3 - "$BMP" <<'PY'
import struct, sys

with open(sys.argv[1], 'rb') as handle:
    data = handle.read()
offset = struct.unpack_from('<I', data, 10)[0]
width, height = struct.unpack_from('<ii', data, 18)
height = abs(height)
bytes_per_pixel = struct.unpack_from('<H', data, 28)[0] // 8
row = (width * bytes_per_pixel + 3) & ~3

ink = 0
blue = 0
for y in range(height):
    start = offset + y * row
    line = data[start:start + width * bytes_per_pixel]
    for x in range(0, len(line), bytes_per_pixel):
        b, g, r = line[x], line[x + 1], line[x + 2]
        if (r + g + b) < 700:
            ink += 1
        if b > 150 and b > r + 40:
            blue += 1

print(f"screen {width}x{height}: {ink} inked pixels, {blue} blue")
if blue < 500:
    sys.exit(f"the app drew no button: {blue} blue pixels, expected at least 500")
PY

# What a screenshot cannot answer: a tofu box and a stroke carry about the same
# ink at 14px, so the app shapes a Han line itself and leaves the `.notdef` count
# in its sandbox. Non-zero means the face never resolved.
DATA=$(xcrun simctl get_app_container "$UDID" "$BUNDLE" data)
REPORT="$DATA/tmp/floom-glyphs.txt"
tries=0
while [ ! -f "$REPORT" ] && [ "$tries" -lt 20 ]; do
  sleep 1
  tries=$((tries + 1))
done
if [ ! -f "$REPORT" ]; then
  echo "the app left no glyph report at $REPORT" >&2
  exit 1
fi
GLYPHS=$(cat "$REPORT")
case "$GLYPHS" in
  "notdef=0") say "ok: Han text shaped to real glyphs ($GLYPHS)" ;;
  *) echo "CJK text shaped to .notdef boxes: $GLYPHS" >&2; exit 1 ;;
esac

say "ok: $NAME rendered on $UDID"

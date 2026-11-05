#!/usr/bin/env bash
# SPDX-License-Identifier: MIT
# SPDX-FileCopyrightText: The Floem Authors
# Build the macOS gate driver (`drv`).
#
# `docs/gates/macos.md` records what a spot-check observes, and a spot-check
# that drives the app from another process needs to synthesise input and read
# OS state: click a button, type through the selected input method, read the
# system pasteboard, dump the accessibility tree. `drv` is that tool — it is
# Swift because those are CoreGraphics / AppKit / Carbon APIs, and it exists as
# source here so the next spot-check is reproducible rather than remembered.
#
# Usage:
#   bash scripts/macos-gate/build-drv.sh [output-path]
#
# Requires: Xcode command-line tools (`xcrun --find swiftc`). The *first* run
# needs two TCC grants for the process that launched this shell (see the gate
# file): Accessibility (to synthesise key/mouse events and read the AX tree)
# and Screen Recording (to take screenshots). Without the first, `drv click`
# and `drv type` are silently discarded by the window server.

set -euo pipefail

here="$(cd "$(dirname "${BASH_SOURCE[0]}")" && pwd)"
out="${1:-${TMPDIR:-/tmp}/floom-macos-drv}"

swiftc -O -target arm64-apple-macosx15.0 -o "$out" "$here/drv.swift" 2>/dev/null
printf 'built %s\n' "$out"
"$out" trust

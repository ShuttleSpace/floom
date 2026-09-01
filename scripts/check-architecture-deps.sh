#!/usr/bin/env bash
# Architecture dependency boundary checks for Floem 2.0.
#
# Forbidden edges (once both crates exist in the workspace graph):
#   floem-scene            → concrete platform / windowing crates
#   floem-types            → platform / GPU crates
#   floem-*-widgets / floem-widgets → concrete renderer backends
#   floem_*_renderer / floem-renderer-* → widget crates
#
# Pre-migration: the monolithic `floem` crate still mixes these concerns.
# Those paths are allowlisted below until the corresponding Phase tasks land.
# New crates must not add forbidden edges; allowlist entries must shrink over time.

set -euo pipefail

ROOT="$(cd "$(dirname "$0")/.." && pwd)"
cd "$ROOT"

if ! command -v python3 >/dev/null; then
  echo "error: python3 required" >&2
  exit 1
fi

META_JSON="$(cargo metadata --format-version 1 --no-deps 2>/dev/null)"
# Prefer full metadata (with deps) for edge checks; fall back if fetch fails.
if cargo metadata --format-version 1 >/tmp/floem-arch-meta.json 2>/dev/null; then
  META_FILE=/tmp/floem-arch-meta.json
else
  echo "$META_JSON" >/tmp/floem-arch-meta-nodeps.json
  META_FILE=/tmp/floem-arch-meta-nodeps.json
  echo "warning: using --no-deps metadata; dependency edges limited" >&2
fi

python3 - "$META_FILE" <<'PY'
import json, sys
from pathlib import Path

meta = json.loads(Path(sys.argv[1]).read_text())
packages = {p["name"]: p for p in meta["packages"]}
resolve = meta.get("resolve")
nodes = {n["id"]: n for n in resolve["nodes"]} if resolve else {}

# Map package id → name for resolve nodes
id_to_name = {}
for p in meta["packages"]:
    id_to_name[p["id"]] = p["name"]

def pkg_deps(name: str) -> set[str]:
    """Direct dependency package names for a workspace package."""
    p = packages.get(name)
    if not p:
        return set()
    # Prefer resolve graph when available
    for nid, node in nodes.items():
        if id_to_name.get(nid) == name or nid.startswith(name + " "):
            deps = set()
            for dep in node.get("deps", []):
                deps.add(dep["name"])
            return deps
    # Fallback: manifest deps (names only)
    return {d["name"] for d in p.get("dependencies", []) if d.get("kind") in (None, "normal", "build")}

PLATFORM_HINTS = {
    "winit", "floem-winit", "floem-winit-core", "tgui-winit-ohos",
    "objc2", "objc2-app-kit", "objc2-ui-kit", "android_logger", "ndk", "jni",
    "napi", "napi-derive", "accesskit_unix", "accesskit_macos", "accesskit_windows",
    "muda", "rfd", "copypasta", "clipboard-win", "windows-sys",
}
GPU_HINTS = {
    "wgpu", "vello", "vger", "floem-vger", "skia-safe", "metal", "ash",
}
RENDERER_PACKAGES = {
    "floem_vger_renderer", "floem_vello_renderer", "floem_skia_renderer",
    "floem_tiny_skia_renderer", "floem_renderer",
}
WIDGET_HINTS = {"floem-widgets", "floem_widgets"}

# Crates that do not exist yet — checked when present
FUTURE = [
    ("floem-types", PLATFORM_HINTS | GPU_HINTS, "types must stay platform/GPU free"),
    ("floem-scene", PLATFORM_HINTS | {"winit", "floem-winit"}, "scene must not depend on OS windowing"),
]

failures = []
notes = []

# --- Future crate rules ---
for crate, banned, msg in FUTURE:
    if crate not in packages:
        notes.append(f"SKIP {crate}: not in workspace yet ({msg})")
        continue
    deps = pkg_deps(crate)
    bad = deps & banned
    if bad:
        failures.append(f"FORBIDDEN {crate} → {sorted(bad)}: {msg}")
    else:
        notes.append(f"OK {crate}: no forbidden deps ({msg})")

# --- Renderer must not depend on widget crates ---
for r in RENDERER_PACKAGES:
    if r not in packages:
        continue
    deps = pkg_deps(r)
    bad = deps & WIDGET_HINTS
    # Also forbid depending on the monolith once widgets live elsewhere;
    # today renderers correctly do not depend on `floem`.
    if "floem" in deps and r != "floem":
        # soft note: unexpected
        failures.append(f"FORBIDDEN {r} → floem (renderer must not depend on widgets/app)")
    if bad:
        failures.append(f"FORBIDDEN {r} → {sorted(bad)}")

# --- Widget crates must not depend on concrete renderer backends ---
for w in list(WIDGET_HINTS):
    if w not in packages:
        notes.append(f"SKIP {w}: not in workspace yet")
        continue
    deps = pkg_deps(w)
    bad = deps & (RENDERER_PACKAGES - {"floem_renderer"})  # abstract trait crate OK later
    if bad:
        failures.append(f"FORBIDDEN {w} → concrete renderer {sorted(bad)}")

# --- Legacy allowlist: monolithic floem mixes concerns ---
LEGACY_ALLOW = {
    "floem",  # app+widgets+platform+paint until migration
}
if "floem" in packages:
    deps = pkg_deps("floem")
    mixed = sorted((deps & PLATFORM_HINTS) | (deps & GPU_HINTS) | (deps & RENDERER_PACKAGES))
    notes.append(
        "ALLOWLIST floem (legacy monolith): mixes platform/GPU/renderer — "
        f"sample deps present: {mixed[:12]}{'...' if len(mixed) > 12 else ''}"
    )

print("=== Floem architecture dependency check ===")
for n in notes:
    print(f"  · {n}")
if failures:
    print("\nFAILURES:")
    for f in failures:
        print(f"  ✗ {f}")
    sys.exit(1)
print("\nPASS (no new forbidden edges)")
sys.exit(0)
PY

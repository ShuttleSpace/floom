#!/usr/bin/env bash
# Architecture dependency boundary checks for Floem 2.0.
#
# Forbidden edges (once both crates exist in the workspace graph):
#   floem-scene            → concrete platform / windowing crates
#   floem-types            → platform / GPU crates
#   floem-*-widgets / floem-widgets → concrete renderer backends
#   floem-legacy-renderer-* / floem-renderer-* → widget crates
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
resolve = meta.get("resolve")
nodes = {n["id"]: n for n in resolve["nodes"]} if resolve else {}


def norm(name: str) -> str:
    """Cargo's resolve graph spells dependencies the way the *lib target* is
    spelled — `floem_core`, not `floem-core` — while manifests and this script
    use hyphens. Comparing the two without normalising makes every set
    intersection below empty, i.e. the check reports PASS while checking
    nothing. Everything is compared normalised.
    """
    return name.replace("-", "_")


packages = {norm(p["name"]): p for p in meta["packages"]}

# Map package id → name for resolve nodes
id_to_name = {}
for p in meta["packages"]:
    id_to_name[p["id"]] = p["name"]

# Package ids of this workspace's own members. Names are not unique in the
# resolve graph — a same-named crate outside the repo (a sibling checkout pulled
# in by a path dependency) can shadow ours, and "first node whose name matches"
# then checks the wrong crate. Always resolve within the workspace first.
workspace_members = set(meta.get("workspace_members", []))


def resolve_node(name: str):
    """The resolve node for a *workspace member* named `name`, if there is one."""
    wanted = norm(name)
    for nid, node in nodes.items():
        if nid in workspace_members and norm(id_to_name.get(nid, "")) == wanted:
            return node
    for nid, node in nodes.items():
        if norm(id_to_name.get(nid, "")) == wanted:
            return node
    return None


def pkg_deps(name: str) -> set[str]:
    """Direct dependency package names for a workspace package."""
    name = norm(name)
    p = packages.get(name)
    if not p:
        return set()
    # Prefer resolve graph when available
    node = resolve_node(name)
    if node is not None:
        return {norm(dep["name"]) for dep in node.get("deps", [])}
    # Fallback: manifest deps (names only)
    return {norm(d["name"]) for d in p.get("dependencies", [])
            if d.get("kind") in (None, "normal", "build")}


def shipped_deps(name: str) -> set[str]:
    name = norm(name)
    """Like `pkg_deps`, but without dev-dependencies.

    Layering is a property of what a crate *ships*: a test-only dependency does
    not make the crate's architecture depend on the thing it tests with. The app
    dev-depending on `floem-testkit` while `floem-testkit` depends on the app is
    the normal shape of an integration harness, not an upward edge.
    """
    node = resolve_node(name)
    if node is not None:
        deps = set()
        for dep in node.get("deps", []):
            kinds = {k.get("kind") for k in dep.get("dep_kinds", [])}
            if kinds and kinds <= {"dev"}:
                continue
            deps.add(norm(dep["name"]))
        return deps
    return {norm(d["name"]) for d in packages[name].get("dependencies", [])
            if d.get("kind") in (None, "normal", "build")}

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
    # Legacy v1 renderers, relocated under `legacy/` with a `floem-legacy-`
    # package prefix (e.g. the old `floem_renderer` is now `floem-legacy-renderer`).
    "floem-legacy-renderer", "floem-legacy-renderer-vello",
    "floem-legacy-renderer-skia", "floem-legacy-renderer-vger",
    "floem-legacy-renderer-tiny-skia",
    # 2.0 renderer ABI + backends, under `render/`. `floem-renderer` is the
    # abstract ABI (the one widgets are allowed to name); the rest are concrete.
    "floem-renderer", "floem-renderer-vello", "floem-renderer-software",
    "floem-renderer-skia", "floem-renderer-tui",
}
WIDGET_HINTS = {"floem-widgets", "floem-widgets-data", "floem-widgets-media"}
# Legacy underscore spelling still referenced by pre-2.0 crates.
WIDGET_ALIASES = {"floem_widgets"}

# Crates that do not exist yet — checked when present
FUTURE = [
    ("floem-types", PLATFORM_HINTS | GPU_HINTS, "types must stay platform/GPU free"),
    ("floem-scene", PLATFORM_HINTS | {"winit", "floem-winit"}, "scene must not depend on OS windowing"),
    ("floem-platform", PLATFORM_HINTS | GPU_HINTS, "platform ABI must stay free of concrete OS/GPU crates"),
]

failures = []
notes = []

# Every name set is compared against resolve-graph names, so normalise them all
# once here — see `norm`.
PLATFORM_HINTS = {norm(x) for x in PLATFORM_HINTS}
GPU_HINTS = {norm(x) for x in GPU_HINTS}
RENDERER_PACKAGES = {norm(x) for x in RENDERER_PACKAGES}
WIDGET_HINTS = {norm(x) for x in WIDGET_HINTS}
WIDGET_ALIASES = {norm(x) for x in WIDGET_ALIASES}
FUTURE = [(norm(crate), {norm(x) for x in banned}, msg) for crate, banned, msg in FUTURE]

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
    bad = deps & (WIDGET_HINTS | WIDGET_ALIASES)
    # Also forbid depending on the monolith once widgets live elsewhere;
    # today renderers correctly do not depend on `floem`.
    if "floem" in deps and r != "floem":
        # soft note: unexpected
        failures.append(f"FORBIDDEN {r} → floem (renderer must not depend on widgets/app)")
    if bad:
        failures.append(f"FORBIDDEN {r} → {sorted(bad)}")

# --- Widget crates must not depend on concrete renderer backends or OS crates ---
for w in sorted(WIDGET_HINTS):
    if w not in packages:
        notes.append(f"SKIP {w}: not in workspace yet")
        continue
    deps = pkg_deps(w)
    # Abstract trait crate OK later; concrete backends are not.
    bad_renderer = deps & (RENDERER_PACKAGES - {"floem_renderer"})
    bad_platform = deps & PLATFORM_HINTS
    if bad_renderer:
        failures.append(f"FORBIDDEN {w} → concrete renderer {sorted(bad_renderer)}")
    if bad_platform:
        failures.append(f"FORBIDDEN {w} → platform/windowing {sorted(bad_platform)}")
    if not bad_renderer and not bad_platform:
        notes.append(f"OK {w}: no concrete renderer or platform dependency")

# --- Layer order: no crate may depend on a crate above it ---
#
# The spec's §6 dependency flow (types → core → runtime → layout/text/scene →
# ABI → backends), instantiated for the crates that exist today. §5 explicitly
# allows the *directory* layout to be adapted; the dependency directions are the
# normative part, and this is where they become mechanically checkable.
#
# `floem-accessibility` sits at L3 rather than with the rest of `framework/`
# because the platform backends depend on it to publish semantics: it is a
# neutral schema crate that happens to live under framework/.
LAYERS: dict[str, int] = {}
for _names, _level in [
    # Legacy v1 crates (under `legacy/`): the bottom of the graph. Anything may
    # depend *down* onto them; nothing may depend up out of them. Keeping them
    # in the table turns a stray legacy → 2.0 edge into a failure instead of a
    # silently skipped dependency.
    (
        [
            "floem-legacy-renderer", "floem-legacy-renderer-vello",
            "floem-legacy-renderer-skia", "floem-legacy-renderer-vger",
            "floem-legacy-renderer-tiny-skia", "floem-legacy-reactive",
            "floem-legacy-test",
        ],
        0,
    ),
    (["floem-types"], 0),
    (["floem-core"], 1),
    (["floem-runtime-core", "floem-runtime", "floem-scheduler"], 2),
    (
        [
            "floem-layout", "floem-style", "floem-text", "floem-scene",
            "floem-virtual", "floem-accessibility",
        ],
        3,
    ),
    (
        [
            "floem-renderer", "floem-renderer-software", "floem-renderer-vello",
            "floem-renderer-skia", "floem-renderer-tui",
            "floem-platform", "floem-platform-desktop", "floem-platform-android",
            "floem-platform-ios", "floem-platform-harmony", "floem-platform-web",
            "floem-platform-embedded", "floem-platform-tui", "floem-platform-mobile",
            "floem-native",
            # winit event-loop adapter; no internal floem deps, consumed by the
            # desktop platform layer above.
            "floem-ui-events-winit",
        ],
        4,
    ),
    (
        [
            "floem-animation", "floem-focus", "floem-gesture", "floem-overlay",
            "floem-theme", "floem-store", "floem-task", "floem-forms",
            "floem-navigation", "floem-persistence", "floem-services",
            "floem-error", "floem-ime", "floem-editor",
            # Design systems sit beside `floem-theme` (which they extend) and
            # below the widgets they style. They are renderer-neutral: their
            # output is `floem_style` property values, not paint calls.
            "floem-design", "floem-material", "floem-cupertino", "floem-fluent",
            "floem-harmony",
        ],
        5,
    ),
    (["floem-widgets", "floem-widgets-data"], 6),
    (["floem-app"], 7),
    # Tooling/compiler tier: these may depend on anything below them and are not
    # shipped as part of an application binary. `floem-testkit` is the runtime
    # test harness; `floem-snapshot` is the golden-image engine (Phase 24.2);
    # `floem-codegen` is the native-glue compiler (Phase 18); `floem-ui-ir` is
    # the versioned UI intermediate representation (Phase 22.1).
    (["floem-testkit", "floem-snapshot", "floem-codegen", "floem-ui-ir", "floem-dsl", "floem-hot-reload", "floem-inspector", "floem-devtools", "floem-profiler", "floem-agent", "floem-accuracy"], 8),
    # L9: the public facade (`floem2::prelude`), Phase 29.1. It aggregates the
    # whole stack — including the L8 devtools bundle behind a feature — so it
    # sits above everything it re-exports. Unlike the L8 tooling tier it IS
    # shipped as part of an application binary.
    (["floem2"], 9),
]:
    for _n in _names:
        LAYERS[_n] = _level

# Upward edges that exist, are understood, and are not to be "fixed" blindly.
# Each entry needs a reason; an entry without one is a bug waiting to happen.
UPWARD_ALLOW: dict[tuple[str, str], str] = {}
LAYERS = {norm(k): v for k, v in LAYERS.items()}
UPWARD_ALLOW = {(norm(a), norm(b)): why for (a, b), why in UPWARD_ALLOW.items()}

for crate, level in sorted(LAYERS.items()):
    if crate not in packages:
        notes.append(f"SKIP layer check {crate}: not in workspace yet")
        continue
    bad = []
    for dep in sorted(shipped_deps(crate)):
        dep_level = LAYERS.get(dep)
        if dep_level is None or dep_level <= level:
            continue
        if (crate, dep) in UPWARD_ALLOW:
            notes.append(
                f"ALLOWLIST {crate}(L{level}) → {dep}(L{dep_level}): "
                f"{UPWARD_ALLOW[(crate, dep)]}"
            )
            continue
        bad.append(f"{dep}(L{dep_level})")
    if bad:
        failures.append(f"UPWARD {crate}(L{level}) → {', '.join(bad)}")
    else:
        notes.append(f"OK {crate}(L{level}): no dependency on a higher layer")

# --- Completeness: every 2.0 library crate must be registered above ---
#
# The loop above only inspects crates that appear in LAYERS. A workspace member
# that was never added there is silently unchecked — which is how a crate with a
# genuine upward edge can still be reported "OK" by simply not being listed.
# (This is the same class of defect as the `norm()` miss that once let three
# rules skip: a check that cannot fire.) Scope the guard to the 2.0 library
# directories; `examples/` and the legacy monolith (`src/`) are deliberately not
# layered, and `vendor/` + `ecosystem/` are excluded from the workspace.
LIB_DIRS = (
    "foundation/", "runtime/", "layout/", "scene/", "text/", "render/",
    "platform/", "framework/", "widgets/", "app/", "native/", "compiler/",
    "tooling/", "ui-events/", "design-systems/",
)
workspace_root = Path(meta["workspace_root"])
unregistered = []
for _pid in workspace_members:
    _name = norm(id_to_name.get(_pid, ""))
    _p = packages.get(_name)
    if not _p:
        continue
    try:
        _rel = Path(_p["manifest_path"]).parent.relative_to(workspace_root)
    except ValueError:
        continue
    _rel = _rel.as_posix() + "/"
    if any(_rel.startswith(d) for d in LIB_DIRS) and _name not in LAYERS:
        unregistered.append(f"{_name} ({_rel})")
if unregistered:
    failures.append(
        "UNREGISTERED 2.0 library crate(s) missing from LAYERS: "
        + ", ".join(sorted(unregistered))
    )

# Legacy v1 crates a 2.0 crate still depends on: the v1 reactive graph.
# Reported, not failed — see the topology decision doc.
LEGACY_DEPS = {norm(x) for x in ("floem_legacy_reactive",)}
reach = sorted(
    {d for c in LAYERS if c in packages for d in shipped_deps(c)} & LEGACY_DEPS
)
if reach:
    users = sorted(c for c in LAYERS if c in packages and shipped_deps(c) & LEGACY_DEPS)
    notes.append(
        f"NOTE legacy dependency: {', '.join(users)} still depend on "
        f"{', '.join(reach)} (the v1 reactive crate, not a 2.0 layer)"
    )

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

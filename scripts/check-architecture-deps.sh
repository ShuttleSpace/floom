#!/usr/bin/env bash
# Architecture dependency boundary checks for Floom 2.0.
#
# Forbidden edges (once both crates exist in the workspace graph):
#   floom-scene            → concrete platform / windowing crates
#   floom-types            → platform / GPU crates
#   floom-*-widgets / floom-widgets → concrete renderer backends
#   floom-legacy-renderer-* / floom-renderer-* → widget crates
#
# Pre-migration: the monolithic `floom` crate still mixes these concerns.
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
if cargo metadata --format-version 1 >/tmp/floom-arch-meta.json 2>/dev/null; then
  META_FILE=/tmp/floom-arch-meta.json
else
  echo "$META_JSON" >/tmp/floom-arch-meta-nodeps.json
  META_FILE=/tmp/floom-arch-meta-nodeps.json
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
    spelled — `floom_core`, not `floom-core` — while manifests and this script
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
    dev-depending on `floom-testkit` while `floom-testkit` depends on the app is
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
    # Legacy v1 renderers, relocated under `legacy/` with a `floom-legacy-`
    # package prefix (e.g. the old `floom_renderer` is now `floom-legacy-renderer`).
    "floom-legacy-renderer", "floom-legacy-renderer-vello",
    "floom-legacy-renderer-skia", "floom-legacy-renderer-vger",
    "floom-legacy-renderer-tiny-skia",
    # 2.0 renderer ABI + backends, under `render/`. `floom-renderer` is the
    # abstract ABI (the one widgets are allowed to name); the rest are concrete.
    "floom-renderer", "floom-renderer-vello", "floom-renderer-software",
    "floom-renderer-skia", "floom-renderer-tui",
}
WIDGET_HINTS = {"floom-widgets", "floom-widgets-data", "floom-widgets-media"}
# Legacy underscore spelling still referenced by pre-2.0 crates.
WIDGET_ALIASES = {"floom_widgets"}

# Crates that do not exist yet — checked when present
FUTURE = [
    ("floom-types", PLATFORM_HINTS | GPU_HINTS, "types must stay platform/GPU free"),
    ("floom-scene", PLATFORM_HINTS | {"winit", "floem-winit"}, "scene must not depend on OS windowing"),
    ("floom-platform", PLATFORM_HINTS | GPU_HINTS, "platform ABI must stay free of concrete OS/GPU crates"),
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
    # today renderers correctly do not depend on `floom`.
    if "floom" in deps and r != "floom":
        # soft note: unexpected
        failures.append(f"FORBIDDEN {r} → floom (renderer must not depend on widgets/app)")
    if bad:
        failures.append(f"FORBIDDEN {r} → {sorted(bad)}")

# --- Widget crates must not depend on concrete renderer backends or OS crates ---
for w in sorted(WIDGET_HINTS):
    if w not in packages:
        notes.append(f"SKIP {w}: not in workspace yet")
        continue
    deps = pkg_deps(w)
    # Abstract trait crate OK later; concrete backends are not.
    bad_renderer = deps & (RENDERER_PACKAGES - {"floom_renderer"})
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
# `floom-accessibility` sits at L3 rather than with the rest of `framework/`
# because the platform backends depend on it to publish semantics: it is a
# neutral schema crate that happens to live under framework/.
LAYERS: dict[str, int] = {}
for _names, _level in [
    (["floom-types"], 0),
    (["floom-core"], 1),
    # `floom-signal` is the reactive runtime (it was `floom-legacy-reactive`
    # until this layer's members stopped being v1 leftovers). It sits at L2 with
    # the runtime core that bridges it, not at L0 with where it used to live: it
    # is not v1 code, and `floom-runtime-core` depends on it to turn signal
    # writes into dirty flags.
    (["floom-runtime-core", "floom-scheduler", "floom-signal"], 2),
    (
        [
            "floom-layout", "floom-style", "floom-text", "floom-scene",
            "floom-virtual", "floom-accessibility",
        ],
        3,
    ),
    (
        [
            "floom-renderer", "floom-renderer-software", "floom-renderer-vello",
            "floom-renderer-skia", "floom-renderer-tui", "floom-renderer-web",
            "floom-platform", "floom-platform-desktop", "floom-platform-android",
            "floom-platform-ios", "floom-platform-harmony", "floom-platform-web",
            "floom-platform-embedded", "floom-platform-tui", "floom-platform-mobile",
            "floom-native",
            # winit event-loop adapter; no internal floom deps, consumed by the
            # desktop platform layer above.
            "floom-ui-events-winit",
        ],
        4,
    ),
    (
        [
            "floom-animation", "floom-focus", "floom-gesture", "floom-overlay", "floom-store", "floom-task", "floom-forms",
            "floom-navigation", "floom-persistence", "floom-services",
            "floom-error", "floom-ime", "floom-editor",
            # The design systems (`floom-theme`, `floom-design`, `floom-material`,
            # `floom-cupertino`, `floom-fluent`, `floom-harmony`) are deliberately
            # absent. They moved to the `floom-ui` workspace, which the root
            # workspace `exclude`s, so this gate never sees them as members and
            # cannot enforce their edges. Their own build is checked there.
        ],
        5,
    ),
    (["floom-widgets", "floom-widgets-data"], 6),
    (["floom-app"], 7),
    # Tooling/compiler tier: these may depend on anything below them and are not
    # shipped as part of an application binary. `floom-testkit` is the runtime
    # test harness; `floom-snapshot` is the golden-image engine (Phase 24.2);
    # `floom-codegen` is the native-glue compiler (Phase 18); `floom-ui-ir` is
    # the versioned UI intermediate representation (Phase 22.1).
    (["floom-testkit", "floom-snapshot", "floom-codegen", "floom-ui-ir", "floom-dsl", "floom-hot-reload", "floom-inspector", "floom-devtools", "floom-profiler", "floom-agent", "floom-accuracy"], 8),
    # L9: the public facade (`floom::prelude`). It aggregates the whole stack —
    # including the L8 devtools bundle behind a feature — so it sits above
    # everything it re-exports. Unlike the L8 tooling tier it IS shipped as part
    # of an application binary.
    (["floom"], 9),
    # `floom-cli` scaffolds projects against the facade and links none of the
    # stack; it has to sit above the layer it names, or a scaffolded project
    # could point at a crate the CLI itself does not.
    (["floom-cli"], 9),
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
# crates, which now all sit directly under `crates/` since the domain
# directories were dissolved. Out of scope: `examples/`, the separate
# workspaces under `crates/vendor/` + `crates/ecosystem/`, the legacy v1 crates
# (`crates/floom-legacy-*`, not layered), and the monolithic `crates/floom`
# (handled by LEGACY_ALLOW below).
LIB_DIR = "crates/"
LIB_DIR_SKIP_PREFIXES = (
    "crates/vendor/",
    "crates/ecosystem/",
    "crates/floom-legacy-",
)
LIB_DIR_SKIP_EXACT = {"crates/floom/"}
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
    if (
        _rel.startswith(LIB_DIR)
        and not _rel.startswith(LIB_DIR_SKIP_PREFIXES)
        and _rel not in LIB_DIR_SKIP_EXACT
        and _name not in LAYERS
    ):
        unregistered.append(f"{_name} ({_rel})")
if unregistered:
    failures.append(
        "UNREGISTERED 2.0 library crate(s) missing from LAYERS: "
        + ", ".join(sorted(unregistered))
    )

# Legacy v1 crates a 2.0 crate still depends on: the v1 reactive graph.
# Reported, not failed — see the topology decision doc.
LEGACY_DEPS = {norm(x) for x in ("floom_legacy_reactive",)}
reach = sorted(
    {d for c in LAYERS if c in packages for d in shipped_deps(c)} & LEGACY_DEPS
)
if reach:
    users = sorted(c for c in LAYERS if c in packages and shipped_deps(c) & LEGACY_DEPS)
    notes.append(
        f"NOTE legacy dependency: {', '.join(users)} still depend on "
        f"{', '.join(reach)} (the v1 reactive crate, not a 2.0 layer)"
    )

# --- Facade allowlist: the public crate mixes concerns on purpose ---
#
# It aggregates the whole stack on purpose -- that is what a facade is for --
# so "it depends on both a renderer and a platform" is its definition, not a
# finding. The allowlist exists so the checker does not spend the rest of its
# output repeating it.
#
# This entry used to read "legacy monolith", naming a v1 crate that no longer
# exists. The name outlived its referent in the same way `floom-legacy-reactive`
# did, and the message it printed described a mixing that no longer had a cause.
FACADE_ALLOW = {
    "floom",  # the public facade: aggregates the stack below it
}
if "floom" in packages:
    deps = pkg_deps("floom")
    mixed = sorted((deps & PLATFORM_HINTS) | (deps & GPU_HINTS) | (deps & RENDERER_PACKAGES))
    notes.append(
        "ALLOWLIST floom (facade): aggregates platform/GPU/renderer by design — "
        f"{len(mixed)} such deps, expected"
    )

print("=== Floom architecture dependency check ===")
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

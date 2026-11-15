#!/usr/bin/env python3
"""Structural verifier for the floom-ui feature-kit merge.

Run: python3 scripts/verify_merge.py <ds>   (ds in material|cupertino|fluent)

Asserts the merged crate in crates/floom-<ds> has the exact shape the merge
must produce, WITHOUT compiling. Run before the generator exists (expects
FAIL) and after (expects PASS). This is the red test for the merge.

It is only meaningful while the original component crates still exist
(crates/<ds>/<prefix>-*); run it during Tasks 1-3, not after Task 4 deletes
the sources.
"""
import os
import re
import sys

def _repo_root():
    d = os.path.dirname(os.path.abspath(__file__))
    while d != os.path.dirname(d):
        if os.path.isdir(os.path.join(d, ".git")):
            return d
        d = os.path.dirname(d)
    return d  # fallback: this dir


ROOT = _repo_root()
UI = os.path.join(ROOT, "crates", "ecosystem", "floom-ui")
PREFIX = {"material": "md", "cupertino": "cupertino", "fluent": "fluent"}


def fail(msg):
    print("FAIL:", msg)
    sys.exit(1)


def feat_of(dirname, prefix):
    assert dirname.startswith(prefix + "-"), dirname
    return dirname[len(prefix) + 1:].replace("-", "_")


def main():
    ds = sys.argv[1]
    prefix = PREFIX[ds]
    merged = os.path.join(UI, "crates", "floom-" + ds)
    comp_root = os.path.join(UI, "crates", ds)

    if not os.path.isdir(merged):
        fail("merged crate dir missing: %s" % merged)

    ctoml_path = os.path.join(merged, "Cargo.toml")
    if not os.path.isfile(ctoml_path):
        fail("merged Cargo.toml missing: %s" % ctoml_path)
    ctoml = open(ctoml_path).read()

    if 'edition = "2021"' not in ctoml:
        fail('Cargo.toml missing `edition = "2021"` (must not inherit 2024)')
    if "default = []" not in ctoml:
        fail("Cargo.toml missing `default = []`")
    if "full = [" not in ctoml:
        fail("Cargo.toml missing `full = [`")
    if re.search(r'floom-widget-(md|cupertino|fluent)-icon', ctoml):
        fail("merged Cargo.toml still references a deleted icon crate")

    widgets_mod = os.path.join(merged, "src", "widgets", "mod.rs")
    if not os.path.isfile(widgets_mod):
        fail("src/widgets/mod.rs not generated")
    modrs = open(widgets_mod).read()

    fb = re.search(r"\[features\](.*?)(\n\[|\Z)", ctoml, re.S)
    features_block = fb.group(1) if fb else ""

    comp_dirs = sorted(
        d for d in os.listdir(comp_root)
        if os.path.isdir(os.path.join(comp_root, d)) and d.startswith(prefix + "-")
    )
    if not comp_dirs:
        fail(
            "no source component dirs under %s — this is a one-shot verifier "
            "that only runs before the merge deletes the sources; the committed "
            "artifacts are now the source of truth" % comp_root
        )

    orig_tests = 0
    gen_tests = 0
    seen_gen_paths = set()
    for d in comp_dirs:
        feat = feat_of(d, prefix)
        if not os.path.isfile(os.path.join(merged, "src", "widgets", feat + ".rs")):
            fail("missing src/widgets/%s.rs" % feat)
        if '#[cfg(feature = "%s")] pub mod %s;' % (feat, feat) not in modrs:
            fail("widgets/mod.rs missing cfg'd `pub mod %s`" % feat)
        if ("\n%s = [" % feat) not in features_block:
            fail("[features] missing entry `%s`" % feat)

        tdir = os.path.join(comp_root, d, "tests")
        if os.path.isdir(tdir):
            test_files = sorted(
                (f for f in os.listdir(tdir) if f.endswith(".rs")),
                key=lambda f: (f == "api.rs", f),
            )
            for i, f in enumerate(test_files):
                orig_tests += 1
                stem = os.path.splitext(f)[0]
                suffix = "" if i == 0 else "_" + stem
                gname = "%s_%s%s.rs" % (prefix, feat, suffix)
                g = os.path.join(merged, "tests", gname)
                if gname in seen_gen_paths:
                    fail("generated test path collision: %s" % gname)
                seen_gen_paths.add(gname)
                if not os.path.isfile(g):
                    fail("missing generated test %s" % g)
                t = open(g).read()
                if not t.startswith('#![cfg(feature = "%s")]' % feat):
                    fail("generated test %s not cfg-gated" % os.path.basename(g))
                gen_tests += 1

    if orig_tests != gen_tests:
        fail("test count mismatch: orig=%d gen=%d" % (orig_tests, gen_tests))

    if not os.path.isfile(os.path.join(merged, "tests", "gallery.rs")):
        fail("kit golden test tests/gallery.rs missing")
    if ds == "material" and not os.path.isfile(os.path.join(merged, "tests", "motion.rs")):
        fail("material kit golden test tests/motion.rs missing")

    # No leftover cross-crate references in generated component code.
    leftover = re.compile(r"floom_widget_%s_" % prefix)
    for root, _, files in os.walk(os.path.join(merged, "src", "widgets")):
        for fn in files:
            if fn.endswith(".rs") and leftover.search(open(os.path.join(root, fn)).read()):
                fail("leftover floom_widget_%s_ ref in src/widgets/%s" % (prefix, fn))
    tdir = os.path.join(merged, "tests")
    if os.path.isdir(tdir):
        for fn in os.listdir(tdir):
            if fn.startswith(prefix + "_") and fn.endswith(".rs"):
                if leftover.search(open(os.path.join(tdir, fn)).read()):
                    fail("leftover floom_widget_%s_ ref in tests/%s" % (prefix, fn))

    print("PASS: %s merged (%d components, %d tests)" % (ds, len(comp_dirs), gen_tests))
    sys.exit(0)


main()

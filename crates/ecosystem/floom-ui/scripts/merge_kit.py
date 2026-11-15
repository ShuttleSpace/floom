#!/usr/bin/env python3
"""One-shot: merge a floom-ui design system's per-component crates into the
single kit crate, one cargo feature per component (web-sys style).

Usage: python3 scripts/merge_kit.py <ds>   # ds in material|cupertino|fluent

This only CREATES the merged source + Cargo.toml inside crates/floom-<ds>.
It deliberately does NOT delete the old component crates or edit the workspace
Cargo.toml -- that is a separate, atomic human commit step (see plan Task 4).

Spec: docs/superpowers/specs/2026-11-15-floom-ui-feature-kits-design.md
"""
import os
import re
import sys

ROOT = None
for d in [os.path.dirname(os.path.abspath(__file__))]:
    while d != os.path.dirname(d):
        if os.path.isdir(os.path.join(d, ".git")):
            ROOT = d
            break
        d = os.path.dirname(d)
UI = os.path.join(ROOT, "crates", "ecosystem", "floom-ui")
PREFIX = {"material": "md", "cupertino": "cupertino", "fluent": "fluent"}
SPECIAL = {"floom-widget-runtime", "floom-core-theme"}  # carry DS feature
WS_DEPS = {}  # name -> abs path (from [workspace.dependencies])


def load_ws_deps():
    txt = open(os.path.join(UI, "Cargo.toml")).read()
    for m in re.finditer(r'([A-Za-z0-9_-]+)\s*=\s*\{\s*path\s*=\s*"([^"]+)"', txt):
        WS_DEPS[m.group(1)] = os.path.normpath(os.path.join(UI, m.group(2)))


def feat_of(dirname, prefix):
    assert dirname.startswith(prefix + "-"), dirname
    return dirname[len(prefix) + 1:].replace("-", "_")


def cross_component(name, prefix):
    return name.startswith("floom-widget-%s-" % prefix)


def parse_dep_line(line):
    m = re.match(r'\s*([A-Za-z0-9_-]+)\s*=\s*(\{.*\}|".*?")\s*$', line)
    if not m:
        return None
    name, rhs = m.group(1), m.group(2)
    if rhs.startswith('"'):
        return name, {"kind": "raw", "raw": line.rstrip()}
    if "workspace" in rhs and "path" not in rhs:
        fm = re.search(r'features\s*=\s*\[([^\]]*)\]', rhs)
        feats = [x.strip().strip('"') for x in fm.group(1).split(",") if x.strip()] if fm else []
        return name, {"kind": "ws", "feats": feats}
    if "path" in rhs:
        pm = re.search(r'path\s*=\s*"([^"]+)"', rhs)
        fm = re.search(r'features\s*=\s*\[([^\]]*)\]', rhs)
        feats = [x.strip().strip('"') for x in fm.group(1).split(",") if x.strip()] if fm else []
        return name, {"kind": "path", "path": pm.group(1), "feats": feats}
    return name, {"kind": "raw", "raw": line.rstrip()}


def rewrite_src(text, prefix):
    return text.replace("floom_widget_%s_" % prefix, "crate::widgets::")


def rewrite_test(text, prefix, ds):
    return text.replace("floom_widget_%s_" % prefix, "floom_%s::widgets::" % ds)


def main():
    ds = sys.argv[1]
    prefix = PREFIX[ds]
    load_ws_deps()
    comp_root = os.path.join(UI, "crates", ds)
    merged = os.path.join(UI, "crates", "floom-" + ds)
    src_dir = os.path.join(merged, "src")
    widgets_dir = os.path.join(src_dir, "widgets")
    tests_dir = os.path.join(merged, "tests")
    os.makedirs(widgets_dir, exist_ok=True)
    os.makedirs(tests_dir, exist_ok=True)

    deps, dev_deps = {}, {}
    feat_src_deps = {}

    def store(name, info, is_dev):
        bucket = dev_deps if is_dev else deps
        if name in bucket:
            ex = bucket[name]
            if ex["kind"] == "ws" or info["kind"] == "ws":
                feats = set(ex.get("feats", [])) | set(info.get("feats", []))
                bucket[name] = {"kind": "ws", "feats": sorted(feats)}
            else:
                feats = set(ex.get("feats", [])) | set(info.get("feats", []))
                bucket[name] = {"kind": "path",
                                "abs": info.get("abs") or ex.get("abs"),
                                "feats": sorted(feats)}
        else:
            bucket[name] = info

    def add_dep(target, name, info, is_dev):
        if info["kind"] == "raw":
            store(name, info, is_dev)
            return
        if info["kind"] == "ws":
            feats = list(info["feats"])
            if name in SPECIAL and ds not in feats:
                feats.append(ds)
            store(name, {"kind": "ws", "feats": feats}, is_dev)
            return
        if info["kind"] == "path":
            store(name, {"kind": "path",
                         "abs": os.path.normpath(os.path.join(target, info["path"])),
                         "feats": list(info["feats"])}, is_dev)

    # thin-kit deps
    kit_toml = open(os.path.join(merged, "Cargo.toml")).read()
    for section, is_dev in (("[dependencies]", False), ("[dev-dependencies]", True)):
        sm = re.search(re.escape(section) + r"(.*?)(\n\[|\Z)", kit_toml, re.S)
        if not sm:
            continue
        for line in sm.group(1).splitlines():
            r = parse_dep_line(line)
            if r:
                add_dep(merged, *r, is_dev)

    comp_dirs = sorted(
        d for d in os.listdir(comp_root)
        if os.path.isdir(os.path.join(comp_root, d)) and d.startswith(prefix + "-")
    )

    for d in comp_dirs:
        cdir = os.path.join(comp_root, d)
        feat = feat_of(d, prefix)
        ctoml = open(os.path.join(cdir, "Cargo.toml")).read()
        for section, is_dev in (("[dependencies]", False), ("[dev-dependencies]", True)):
            sm = re.search(re.escape(section) + r"(.*?)(\n\[|\Z)", ctoml, re.S)
            if not sm:
                continue
            for line in sm.group(1).splitlines():
                r = parse_dep_line(line)
                if not r:
                    continue
                name = r[0]
                if cross_component(name, prefix):  # becomes intra-crate module
                    continue
                add_dep(cdir, *r, is_dev)
        # infer per-component feature deps from cross-component refs
        fd = [feat_of("%s-%s" % (prefix, m), prefix) for m in
              re.findall(r'floom-widget-%s-([a-z0-9_-]+)' % re.escape(prefix), ctoml)]
        fd = [x for x in fd if x != feat]
        feat_src_deps[feat] = sorted(set(fd))

        # source files
        csrc = os.path.join(cdir, "src")
        lib_txt = rewrite_src(open(os.path.join(csrc, "lib.rs")).read(), prefix)
        open(os.path.join(widgets_dir, feat + ".rs"), "w").write(lib_txt)
        for fn in os.listdir(csrc):
            if fn in ("lib.rs", "main.rs") or not fn.endswith(".rs"):
                continue
            sub = os.path.join(widgets_dir, feat)
            os.makedirs(sub, exist_ok=True)
            open(os.path.join(sub, fn), "w").write(
                rewrite_src(open(os.path.join(csrc, fn)).read(), prefix))

        # tests: one output file per SOURCE test file. The component's primary
        # (non-`api`) test keeps `<prefix>_<feat>.rs`; extra files such as
        # `api.rs` get a distinct `_<stem>` suffix. Ordering non-`api` first
        # keeps the primary name stable. Collapsing all sources onto one path
        # used to silently overwrite 32 contract-API tests.
        tdir = os.path.join(cdir, "tests")
        if os.path.isdir(tdir):
            test_files = sorted(
                (f for f in os.listdir(tdir) if f.endswith(".rs")),
                key=lambda f: (f == "api.rs", f),
            )
            for i, tf in enumerate(test_files):
                t = rewrite_test(open(os.path.join(tdir, tf)).read(), prefix, ds)
                out = '#![cfg(feature = "%s")]\n' % feat + t
                stem = os.path.splitext(tf)[0]
                suffix = "" if i == 0 else "_" + stem
                open(os.path.join(tests_dir, "%s_%s%s.rs" % (prefix, feat, suffix)), "w").write(out)

    # widgets/mod.rs
    mod_lines = ["#![cfg_attr(feature = \"full\", allow(unused))]"]  # keep lints quiet when partial
    for feat in sorted(feat_src_deps):
        mod_lines.append('#[cfg(feature = "%s")] pub mod %s;' % (feat, feat))
    open(os.path.join(widgets_dir, "mod.rs"), "w").write("\n".join(mod_lines) + "\n")

    # lib.rs: keep thin-kit modules, add pub mod widgets; never overwrite kit tests
    lib_rs = os.path.join(src_dir, "lib.rs")
    base = open(lib_rs).read()
    if "pub mod widgets;" not in base:
        if not base.endswith("\n"):
            base += "\n"
        base += "\npub mod widgets;\n"
    open(lib_rs, "w").write(base)

    write_cargo_toml(merged, ds, deps, dev_deps, feat_src_deps)
    print("merged %s: %d components -> crates/floom-%s" % (ds, len(comp_dirs), ds))


def emit(name, info, merged):
    if info["kind"] == "ws":
        if name in SPECIAL:
            return '%s = { workspace = true, features = ["%s"] }' % (name, DS)
        feats = info.get("feats", [])
        if feats:
            return '%s = { workspace = true, features = [%s] }' % (
                name, ", ".join('"%s"' % f for f in feats))
        return "%s = { workspace = true }" % name
    if info["kind"] == "path":
        rel = os.path.relpath(info["abs"], merged)
        feats = info.get("feats", [])
        if feats:
            return '%s = { path = "%s", features = [%s] }' % (
                name, rel, ", ".join('"%s"' % f for f in feats))
        return '%s = { path = "%s" }' % (name, rel)
    return info["raw"]


DS = None


def write_cargo_toml(merged, ds, deps, dev_deps, feat_src_deps):
    global DS
    DS = ds
    kit_toml = open(os.path.join(merged, "Cargo.toml")).read()
    out = []
    section = None
    for line in kit_toml.splitlines():
        if line.startswith("["):
            section = line.strip()
            if section in ("[dependencies]", "[dev-dependencies]", "[features]"):
                continue  # replaced below
            out.append(line)
            continue
        if section == "[package]" and "edition" in line and "workspace" in line:
            out.append('edition = "2021"')
            continue
        if section in ("[dependencies]", "[dev-dependencies]", "[features]"):
            continue  # skip old dep lines
        out.append(line)
    # trim trailing blank lines before appending new sections
    while out and out[-1].strip() == "":
        out.pop()

    out.append("")
    out.append("[dependencies]")
    for name in sorted(deps):
        out.append(emit(name, deps[name], merged))
    out.append("")
    out.append("[dev-dependencies]")
    for name in sorted(dev_deps):
        out.append(emit(name, dev_deps[name], merged))
    out.append("")
    out.append("[features]")
    for feat in sorted(feat_src_deps):
        fd = feat_src_deps[feat]
        out.append('%s = [%s]' % (feat, ", ".join('"%s"' % x for x in fd)))
    out.append("full = [%s]" % ", ".join('"%s"' % f for f in sorted(feat_src_deps)))
    out.append("default = []")
    out.append("")
    open(os.path.join(merged, "Cargo.toml"), "w").write("\n".join(out))


if __name__ == "__main__":
    main()

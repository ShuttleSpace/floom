#!/usr/bin/env python3
"""Scaffold the 3 Batch-20 components x 3 design systems = 9 crates
and register them in the workspace + floom-elements manifests.

Idempotent: re-running will not duplicate already-registered entries.
"""
import os

ROOT = "/Users/ahs/Project/Rust/floom-ui"
COMPS = [
    "number_input",
    "pull_to_refresh",
    "confirm_edit",
]
DS = [
    ("material", "md"),
    ("cupertino", "cupertino"),
    ("fluent", "fluent"),
]

# 1) Create crates + Cargo.toml + empty crates/floom/src/lib.rs placeholder.
for ds, prefix in DS:
    for comp in COMPS:
        crate_dir = os.path.join(ROOT, "crates", ds, f"{prefix}-{comp}")
        src_dir = os.path.join(crate_dir, "src")
        os.makedirs(src_dir, exist_ok=True)
        pkg = f"floom-widget-{prefix}-{comp}"
        if ds == "material":
            deps = (
                "floom = { workspace = true }\n"
                "floom-core-theme = { workspace = true }\n"
                'floom-widget-runtime = { workspace = true, features = ["material"] }\n'
            )
        else:
            deps = (
                "floom = { workspace = true }\n"
                "floom-core-theme = { workspace = true }\n"
            )
        with open(os.path.join(crate_dir, "Cargo.toml"), "w") as f:
            f.write(f"[package]\nname = \"{pkg}\"\nversion = \"0.1.0\"\nedition = \"2021\"\n\n[dependencies]\n{deps}")
        with open(os.path.join(src_dir, "lib.rs"), "w") as f:
            f.write("// placeholder; filled by Material source + swap\n")

# 2) Workspace members - insert before "crates/floom-testing" (idempotent).
ws_path = os.path.join(ROOT, "Cargo.toml")
with open(ws_path) as f:
    ws = f.read()
if '"crates/material/md-number_input",' not in ws:
    member_lines = ""
    for ds, prefix in DS:
        for comp in COMPS:
            member_lines += f'    "crates/{ds}/{prefix}-{comp}",\n'
    anchor = '    "crates/floom-testing",'
    assert anchor in ws, "workspace anchor missing"
    ws = ws.replace(anchor, member_lines + anchor, 1)
    with open(ws_path, "w") as f:
        f.write(ws)

# 3) floom-elements deps - insert before "[features]" (idempotent).
fe_path = os.path.join(ROOT, "crates/floom-elements/Cargo.toml")
with open(fe_path) as f:
    fe = f.read()
if 'floom-widget-md-number_input' not in fe:
    dep_lines = ""
    for ds, prefix in DS:
        for comp in COMPS:
            dep_lines += f'floom-widget-{prefix}-{comp} = {{ path = "../{ds}/{prefix}-{comp}", optional = true }}\n'
    fe_anchor = "\n[features]"
    assert fe_anchor in fe, "features anchor missing"
    fe = fe.replace(fe_anchor, "\n" + dep_lines.rstrip("\n") + fe_anchor, 1)

# 4) floom-elements feature arrays - insert before each array's closing "]".
#    validation_messages (Batch 10) is the current last registered entry.
feature_entries = {
    "md": '    "floom-widget-md-validation_messages",\n]',
    "cupertino": '    "floom-widget-cupertino-validation_messages",\n]',
    "fluent": '    "floom-widget-fluent-validation_messages",\n]',
}
for ds, prefix in DS:
    if f'floom-widget-{prefix}-number_input' in fe:
        continue
    entries = ""
    for comp in COMPS:
        entries += f'    "floom-widget-{prefix}-{comp}",\n'
    old = feature_entries[prefix]
    assert old in fe, f"feature anchor missing for {prefix}"
    fe = fe.replace(old, entries + old, 1)

with open(fe_path, "w") as f:
    f.write(fe)

print("scaffolded 9 crates + registered manifests (idempotent)")

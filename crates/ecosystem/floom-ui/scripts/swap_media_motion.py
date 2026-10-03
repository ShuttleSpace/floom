#!/usr/bin/env python3
"""Swap the 3 Material Batch-17 sources to Cupertino + Fluent.

For each component the Material source is cloned with:
  * `Md{Name}`      -> `{Ds}{Name}`   (struct name)
  * `MaterialTheme` -> `{Ds}Theme`    (theme type)
  * `theme_signal()`-> `RwSignal::new({Ds}Theme::new())`
  * the `use floom_widget_runtime::theme_signal;` import dropped
  * `design_system: "material"` -> `design_system: "{ds}"`
Leak assertions guarantee nothing Material-specific remains.
"""
import os

ROOT = "/Users/ahs/Project/Rust/floom-ui"
COMPS = [
    "parallax",
    "infinite_scroller",
    "hover",
]
TARGETS = [("cupertino", "cupertino", "Cupertino"), ("fluent", "fluent", "Fluent")]


def pascal(name: str) -> str:
    return "".join(p.capitalize() for p in name.split("_"))


def swap_one(comp: str, ds: str, prefix: str, ds_name: str) -> None:
    mat_src = os.path.join(ROOT, "crates", "material", f"md-{comp}", "src", "lib.rs")
    with open(mat_src) as f:
        src = f.read()

    struct = f"Md{pascal(comp)}"
    t_struct = f"{ds_name}{pascal(comp)}"
    t_theme = f"{ds_name}Theme"

    out = src
    out = out.replace(struct, t_struct)
    out = out.replace("MaterialTheme", t_theme)
    out = out.replace("theme_signal()", f"RwSignal::new({t_theme}::new())")
    out = out.replace("use floom_widget_runtime::theme_signal;\n", "")
    out = out.replace('design_system: "material"', f'design_system: "{ds}"')

    # Leak assertions — nothing Material-specific may survive.
    leaks = [struct, "MaterialTheme", "theme_signal", "floom_widget_runtime", '"material"']
    for tok in leaks:
        assert tok not in out, f"leak '{tok}' in {ds}-{comp}"

    dst = os.path.join(ROOT, "crates", ds, f"{prefix}-{comp}", "src", "lib.rs")
    with open(dst, "w") as f:
        f.write(out)
    print(f"swapped {ds}-{comp} -> {t_struct}")


for comp in COMPS:
    for ds, prefix, ds_name in TARGETS:
        swap_one(comp, ds, prefix, ds_name)

print("swap complete: 3 batch-17 x 2 design systems")

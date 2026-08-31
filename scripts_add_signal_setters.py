#!/usr/bin/env python3
"""Inject a `with_<getter>(RwSignal<bool>)` setter into the 27 concrete
widget crates so the multi-dispatch wrappers can inject their own owned signal.

The getter (open_signal / show_signal / refreshing_signal) already exists in
every concrete crate with body `self.<field>` and a 4-space closing brace. We
insert a sibling setter right after that closing brace.
"""
import os

ROOT = os.getcwd()
print("ROOT =", ROOT)

# (stem, getter_name, field_name)
SPECS = [
    ("dialog", "open_signal", "open"),
    ("menu", "open_signal", "open"),
    ("navigation_drawer", "open_signal", "open"),
    ("snackbar", "open_signal", "open"),
    ("sheet", "open_signal", "open"),
    ("slide_transition", "show_signal", "show"),
    ("scale_transition", "show_signal", "show"),
    ("expand_transition", "show_signal", "show"),
    ("pull_to_refresh", "refreshing_signal", "refreshing"),
]

PREFIXES = {"material": "md", "cupertino": "cupertino", "fluent": "fluent"}


def process(stem, getter, field):
    for ds, pfx in PREFIXES.items():
        # glob in case of hyphen/underscore stem variants
        import glob
        matches = glob.glob(os.path.join(ROOT, "crates", ds, f"{pfx}-{stem}"))
        if not matches:
            print(f"  MISSING DIR: {ds}/{pfx}-{stem}")
            continue
        d = matches[0]
        path = os.path.join(d, "src", "lib.rs")
        with open(path) as f:
            lines = f.readlines()

        start = None
        for i, l in enumerate(lines):
            if l.strip() == f"pub fn {getter}(&self) -> RwSignal<bool> {{":
                start = i
                break
        if start is None:
            print(f"  NO GETTER {path}")
            continue

        close = None
        for j in range(start + 1, min(start + 12, len(lines))):
            if lines[j].rstrip() == "    }":
                close = j
                break
        if close is None:
            print(f"  NO CLOSE {path}")
            continue

        setter = (
            f"    pub fn with_{getter}(mut self, {field}: RwSignal<bool>) -> Self {{\n"
            f"        self.{field} = {field};\n"
            f"        self\n"
            f"    }}\n"
        )
        lines.insert(close + 1, setter)
        with open(path, "w") as f:
            f.writelines(lines)
        print(f"  OK {path}")


if __name__ == "__main__":
    for stem, getter, field in SPECS:
        print(f"== {stem} ({getter}) ==")
        process(stem, getter, field)

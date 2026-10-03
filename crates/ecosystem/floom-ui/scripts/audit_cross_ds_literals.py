#!/usr/bin/env python3
"""Cross-DS size-literal divergence audit for floom-ui.

For each component triplicated across material/cupertino/fluent, extract calls to
size/space/radius-related style methods whose argument is a *pure numeric literal*
(e.g. `.width(40.0)`, `.size(32.0, 32.0)`, `.padding(12.0, 8.0)`). Then per component,
per method, compare the literal arguments across the three DS. Where the three DS disagree
(including "one DS uses a literal while another uses a token"), flag a candidate divergence.

This does NOT judge token-worthiness; it only produces objective divergence candidates
so we can then decide which are genuine per-DS differentiations worth tokenizing.
"""
import os
import re
import json
from collections import defaultdict

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
CRATE_ROOT = os.path.join(ROOT, "crates")

# (prefix_dir, prefix)  -> design system key
DS = [
    ("material", "md-"),
    ("cupertino", "cupertino-"),
    ("fluent", "fluent-"),
]

METHODS = [
    "width", "height", "min_width", "min_height", "max_width", "max_height",
    "size", "gap", "padding", "padding_left", "padding_right", "padding_top",
    "padding_bottom", "padding_horiz", "padding_vert", "border_radius", "radius",
    "inset_top", "inset_left", "inset_right", "inset_bottom", "margin",
    "margin_left", "margin_right", "margin_top", "margin_bottom", "flex_basis",
    "min_size", "max_size", "border_width", "icon_size", "font_size",
]

# Build a combined regex: `.<method>(`  (word boundary so `min_width` != `width`)
METHOD_RE = re.compile(
    r"\.(" + "|".join(METHODS) + r")\s*\("
)

NUM_TOKEN = re.compile(r"^-?\d+(\.\d+)?(e-?\d+)?$")
# a "pure literal args" string contains only numbers, commas, whitespace, dots, minus sign
PURE_RE = re.compile(r"^\s*-?\d+(\.\d+)?(e-?\d+)?(\s*,\s*-?\d+(\.\d+)?(e-?\d+)?)*\s*$")


def find_balance(s, start):
    """Given s and index of the opening '(' at start, return index of matching ')'."""
    depth = 0
    i = start
    while i < len(s):
        c = s[i]
        if c == "(":
            depth += 1
        elif c == ")":
            depth -= 1
            if depth == 0:
                return i
        i += 1
    return -1


def extract_literals(text):
    """Return dict method -> set(list_of_argstrings) for purely-numeric-literal args."""
    out = defaultdict(set)
    for m in METHOD_RE.finditer(text):
        method = m.group(1)
        open_idx = m.end() - 1  # index of '('
        close_idx = find_balance(text, open_idx)
        if close_idx == -1:
            continue
        args = text[open_idx + 1:close_idx]
        # skip if contains anything other than numbers/commas/whitespace/dot/minus
        if PURE_RE.match(args):
            out[method].add(args.strip())
    return out


def main():
    # discover components present in all three DS
    comps = defaultdict(dict)  # base -> ds -> crate_dir
    for ds_dir, prefix in DS:
        d = os.path.join(CRATE_ROOT, ds_dir)
        if not os.path.isdir(d):
            continue
        for name in os.listdir(d):
            if name.startswith(prefix):
                base = name[len(prefix):]
                crate = os.path.join(d, name)
                lib = os.path.join(crate, "src", "lib.rs")
                if os.path.isfile(lib):
                    comps[base][ds_dir] = lib

    triplicated = sorted(b for b, v in comps.items()
                         if all(k in v for k, _ in DS))

    results = []
    for base in triplicated:
        per_ds = {}
        for ds_dir, _ in DS:
            lib = comps[base][ds_dir]
            with open(lib, encoding="utf-8") as f:
                txt = f.read()
            per_ds[ds_dir] = extract_literals(txt)

        # collect all methods seen for this component
        all_methods = set()
        for ds_dir, _ in DS:
            all_methods.update(per_ds[ds_dir].keys())

        comp_divergences = {}
        for meth in sorted(all_methods):
            vals = {ds_dir: per_ds[ds_dir].get(meth, set())
                    for ds_dir, _ in DS}
            # union of all literal sets
            all_sets = [v for v in vals.values()]
            union = set().union(*all_sets) if all_sets else set()
            # divergence if not all three identical
            distinct = set(frozenset(v) for v in vals.values())
            if len(distinct) > 1:
                comp_divergences[meth] = {
                    "material": sorted(vals["material"]),
                    "cupertino": sorted(vals["cupertino"]),
                    "fluent": sorted(vals["fluent"]),
                }
        if comp_divergences:
            results.append({
                "component": base,
                "divergences": comp_divergences,
            })

    # summary
    print(f"Triplicated components: {len(triplicated)}")
    print(f"Components with at least one cross-DS literal divergence: {len(results)}")
    total_sites = sum(
        len(d["divergences"]) for d in results
    )
    print(f"Total divergent method-sites: {total_sites}")
    print()

    # rank by number of divergent method-sites (most likely genuine design-system diffs first)
    results.sort(key=lambda d: -len(d["divergences"]))

    for d in results:
        print(f"### {d['component']}  ({len(d['divergences'])} divergent sites)")
        for meth, v in d["divergences"].items():
            print(f"  {meth}: mat={v['material']} cup={v['cupertino']} flu={v['fluent']}")
        print()

    # also dump JSON for downstream triage
    with open(os.path.join(ROOT, "scripts", "_cross_ds_literals.json"), "w") as f:
        json.dump(results, f, indent=2)
    print("Wrote scripts/_cross_ds_literals.json")


if __name__ == "__main__":
    main()

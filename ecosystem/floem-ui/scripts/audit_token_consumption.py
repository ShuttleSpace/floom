#!/usr/bin/env python3
"""Audit per-DS component-token consumption across all widget crates.

For every crate under crates/{material,cupertino,fluent}/, check whether its
source consumes `theme.component(ComponentId::X)`. Crates that do NOT are
candidates for hardcoded sizing (i.e. the per-DS tokens defined in
floem-core-theme are bypassed).

Also reports hardcoded numeric literals passed to size-ish style setters,
which is the concrete symptom of bypassing tokens.
"""

import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
CRATES = ROOT / "crates"
DS_DIRS = {"material": "md-", "cupertino": "cupertino-", "fluent": "fluent-"}

# style setters whose numeric literal argument indicates hardcoded sizing
SIZE_SETTERS = [
    "height",
    "width",
    "min_height",
    "min_width",
    "size",
    "padding_horiz",
    "padding_vert",
    "padding",
    "border_radius",
    "font_size",
    "gap",
    "row_gap",
    "col_gap",
]
HARDCODE_RE = re.compile(
    r"\.(" + "|".join(SIZE_SETTERS) + r")\(\s*(\d+(?:\.\d+)?)\s*\)"
)
COMPONENT_RE = re.compile(r"ComponentId::(\w+)")


def crate_sources(crate_dir: Path):
    src = crate_dir / "src"
    if not src.is_dir():
        return []
    return sorted(src.rglob("*.rs"))


def main():
    rows = []
    for ds, prefix in DS_DIRS.items():
        ds_dir = CRATES / ds
        if not ds_dir.is_dir():
            continue
        for crate in sorted(ds_dir.iterdir()):
            if not crate.is_dir() or not crate.name.startswith(prefix):
                continue
            files = crate_sources(crate)
            if not files:
                continue
            text = "\n".join(f.read_text(errors="replace") for f in files)
            ids = sorted(set(COMPONENT_RE.findall(text)))
            hardcodes = HARDCODE_RE.findall(text)
            rows.append(
                {
                    "ds": ds,
                    "crate": crate.name,
                    "component": crate.name[len(prefix) :],
                    "ids": ids,
                    "consumes": bool(ids),
                    "hardcode_count": len(hardcodes),
                }
            )

    total = len(rows)
    no_token = [r for r in rows if not r["consumes"]]

    print(f"TOTAL widget crates scanned: {total}")
    print(f"CONSUME per-DS tokens      : {total - len(no_token)}")
    print(f"DO NOT consume tokens      : {len(no_token)}")
    print()

    # group non-consumers by component name so we see cross-DS patterns
    by_component = {}
    for r in no_token:
        by_component.setdefault(r["component"], []).append((r["ds"], r["hardcode_count"]))

    print("=== components NOT consuming per-DS tokens (grouped) ===")
    print(f"{'component':<24} {'DS missing':<34} hardcoded-size-literals")
    for comp in sorted(by_component):
        entries = by_component[comp]
        ds_list = ",".join(d for d, _ in sorted(entries))
        hc = "/".join(str(c) for _, c in sorted(entries))
        print(f"{comp:<24} {ds_list:<34} {hc}")

    print()
    print("=== consumers with the MOST hardcoded size literals (top 20) ===")
    consumers = [r for r in rows if r["consumes"]]
    consumers.sort(key=lambda r: -r["hardcode_count"])
    print(f"{'crate':<34} {'ids':<28} hardcoded")
    for r in consumers[:20]:
        ids = ",".join(r["ids"])[:26]
        print(f"{r['crate']:<34} {ids:<28} {r['hardcode_count']}")

    return 0


if __name__ == "__main__":
    sys.exit(main())

#!/usr/bin/env bash
# The component contract: every design-system widget reads its colours and metrics
# from the shared token tables, and the three systems agree on which tables a given
# widget needs.
#
# Two halves: the DSL vocabulary is generated from `floom_dsl::ui_ir` (one table,
# emitted not written), and this script covers the Rust components -- the 3 x ~99
# widgets in `floom-material` / `floom-cupertino` / `floom-fluent`. Comments are
# stripped first: a mention in prose is not a dependency.
set -euo pipefail

cd "$(dirname "$0")/.."
exec python3 - <<'PY'
import pathlib, re, sys

ROOT = pathlib.Path("crates/ecosystem/floom-ui/crates")
SYSTEMS = ["floom-material", "floom-cupertino", "floom-fluent"]
SHARED = {"ComponentTokens", "color", "color_ext", "typography", "spacing"}
# Widgets that are not painted: providers of dates/icons/locales, a spacer, a grid
# layout, and the token modules themselves. Listing them is the point -- a new
# widget that reads no table has to be added here on purpose.
SKIPPED = {
    "date_adapter_provider", "defaults_provider", "display_provider", "grid",
    "icon_paths", "icons_provider", "locale_provider", "spacer", "theme_provider",
}

def strip_comments(text: str) -> str:
    text = re.sub(r"/\*.*?\*/", "", text, flags=re.S)
    return re.sub(r"//[^\n]*", "", text)

def widgets(system: str) -> dict[str, set[str]]:
    directory = ROOT / system / "src" / "widgets"
    out: dict[str, set[str]] = {}
    for path in sorted(directory.glob("*.rs")):
        body = strip_comments(path.read_text())
        names = {n for n in re.findall(r"\b([A-Z][A-Za-z]*Tokens)\b", body) if n != "ComponentTokens"}
        groups = set(re.findall(r"\btokens::([a-z_]+)\b", body))
        out[path.stem] = names
        if not (groups & SHARED) and not names and path.stem not in SKIPPED:
            out.setdefault("__no_tokens__", set()).add(path.name)
    return out

tables = {system: widgets(system) for system in SYSTEMS}
problems: list[str] = []

for system, found in tables.items():
    for name in sorted(found.get("__no_tokens__", [])):
        problems.append(f"{system}/{name} reads no token table (hardcoded look)")

common = set(tables[SYSTEMS[0]]) & set(tables[SYSTEMS[1]]) & set(tables[SYSTEMS[2]])
for widget in sorted(common - {"__no_tokens__"}):
    sets = {system: tables[system][widget] for system in SYSTEMS}
    if len({frozenset(s) for s in sets.values()}) > 1:
        detail = ", ".join(f"{s}: {sorted(v) or 'none'}" for s, v in sets.items())
        problems.append(f"{widget} needs different tables per system ({detail})")

total = sum(len(v) for v in tables.values()) - sum(len(v.get("__no_tokens__", set())) for v in tables.values())
if problems:
    print("FAIL components: the contract is not one contract")
    for problem in problems:
        print(f"  - {problem}")
    sys.exit(1)

print(f"ok: {len(common)} widgets in all three systems, same token tables; {total} widget files read the shared tables")
PY

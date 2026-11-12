#!/usr/bin/env python3
"""Check that every plugin's TypeScript facade mirrors its Rust API.

The facade is the plugin's public contract for a TypeScript author, and the
crate's Rust API is the contract for a Rust author. They have to be the same
API: a facade that has drifted offers an editor a function the crate does not
have, and hides one that it does. That is the difference between a facade that
helps and one that lies.

This reads both sides and reports every disagreement:

  * an `export function` / `export const` in the facade with no Rust
    counterpart — the facade promises an API the plugin does not have;
  * a crate-root public function with no facade entry — the facade hides an
    API the plugin has;
  * a class whose methods disagree with the Rust type's, an interface whose
    fields disagree with the Rust struct's, and an enum whose variants
    disagree with the Rust enum's.

Two things are deliberately not API:

  * `extern "C"` functions. Those are the plugin's ABI — the symbols the
    native half calls — not a surface either language's author calls.
  * a native handle's concrete type. A provider is a platform object
    (`NSView *`, a Java view) that has no TypeScript counterpart, so the
    facade names it `unknown`. That is the honest shape.

The Rust side is read by scanning the source, not by compiling it: a plugin's
public surface is a handful of `pub` items, and a scan needs no build and no
dependency on the plugin's platform arms.

Usage:
    scripts/check-plugin-facades.py            # report and exit non-zero on drift
    scripts/check-plugin-facades.py --verbose  # also list the plugins that conform
"""

from __future__ import annotations

import os
import re
import sys
from pathlib import Path

ROOT = os.path.abspath(os.path.join(os.path.dirname(__file__), ".."))
PLUGINS = os.path.join(ROOT, "crates", "ecosystem", "plugins")

# A public function: `pub fn`, `pub unsafe fn`, `pub const fn`, `pub async fn`,
# and the combinations of those modifiers a public function can carry.
FN_RE = re.compile(
    r"^pub\s+(?:(?:default|const|async|unsafe)\s+)*fn\s+([a-z_][a-z0-9_]*)",
    re.M,
)
# `pub extern "C" fn` — the ABI, not the API.
ABI_RE = re.compile(r"^pub\s+extern\s+\"[^\"]+\"\s+fn\s+([a-z_][a-z0-9_]*)", re.M)
TYPE_RE = re.compile(r"^pub\s+(?:struct|enum|trait)\s+([A-Z][A-Za-z0-9_]*)", re.M)
# `pub use platform::{a, b as c}` — the re-exported names are public API.
REEXPORT_RE = re.compile(r"^pub\s+use\s+([a-z_][a-z0-9_]*)::\{([^}]*)\}", re.M)
# `pub use platform::*` — the whole module's public surface is public API.
GLOB_RE = re.compile(r"^pub\s+use\s+([a-z_][a-z0-9_]*)\s*::\s*\*", re.M)
# A public constant: part of the API, and part of what the facade has to name.
CONST_RE = re.compile(r"^pub\s+const\s+([A-Z][A-Z0-9_]*)\s*:", re.M)


def read(path: str) -> str:
    try:
        with open(path, encoding="utf-8") as handle:
            return handle.read()
    except OSError:
        return ""


def brace_body(source: str, open_at: int) -> tuple[str, int]:
    """The text between the brace at `open_at` and its match."""
    depth = 0
    for index in range(open_at, len(source)):
        char = source[index]
        if char == "{":
            depth += 1
        elif char == "}":
            depth -= 1
            if depth == 0:
                return source[open_at + 1 : index], index + 1
    return source[open_at:], len(source)


def type_alias_body(source: str, start: int) -> str:
    """The text of a type alias, up to the `;` that ends it at depth zero."""
    depth = 0
    for index in range(start, len(source)):
        char = source[index]
        if char in "{[(":
            depth += 1
        elif char in "}])":
            depth -= 1
        elif char == ";" and depth == 0:
            return source[start:index]
    return source[start:]


def public_functions(src: str) -> set[str]:
    """Every public free function, minus the ABI symbols."""
    return set(FN_RE.findall(src)) - set(ABI_RE.findall(src))


def public_consts(src: str) -> set[str]:
    """Every public constant. A constant is part of the API: a capability id
    that the crate publishes and the facade does not name is an id a TypeScript
    author cannot read."""
    return set(CONST_RE.findall(src))


def public_types(src: str) -> dict[str, dict[str, set[str]]]:
    """Every public type, with its variants, fields and methods."""
    types: dict[str, dict[str, set[str]]] = {}

    def slot(name: str) -> dict[str, set[str]]:
        return types.setdefault(name, {"variants": set(), "fields": set(), "methods": set()})

    for match in TYPE_RE.finditer(src):
        slot(match.group(1))
    for match in re.finditer(r"^pub\s+enum\s+([A-Z][A-Za-z0-9_]*)\s*\{", src, re.M):
        body, _ = brace_body(src, match.end() - 1)
        for line in body.splitlines():
            stripped = line.strip()
            if not stripped or stripped.startswith("//"):
                continue
            head = stripped.split("(", 1)[0].split("=")[0].split("{")[0].strip().rstrip(",")
            if head and head[0].isupper() and head.isidentifier():
                slot(match.group(1))["variants"].add(head)
    for match in re.finditer(r"^pub\s+struct\s+([A-Z][A-Za-z0-9_]*)\s*\{", src, re.M):
        body, _ = brace_body(src, match.end() - 1)
        for line in body.splitlines():
            stripped = line.strip()
            if stripped.startswith("pub ") and ":" in stripped:
                name = stripped[4:].split(":", 1)[0].strip()
                if name.isidentifier():
                    slot(match.group(1))["fields"].add(name)
    for match in re.finditer(r"^impl(?:<[^>]*>)?\s+([A-Z][A-Za-z0-9_]*)\s*\{", src, re.M):
        body, _ = brace_body(src, match.end() - 1)
        methods = set(
            re.findall(
                r"^\s{4}pub\s+(?:(?:default|const|async|unsafe)\s+)*fn\s+([a-z_][a-z0-9_]*)",
                body,
                re.M,
            )
        )
        slot(match.group(1))["methods"].update(methods)
    return types


def wide_names(src: str) -> set[str]:
    """Every public-looking name in a source, at any indentation.

    A macro body can declare a `pub` item at any depth: the file-dialog
    plugin's `SUPPORTED` is written inside a `macro_rules!` body, and it is
    part of the crate's API. A source scanner that only reads column zero
    cannot see those, so this wider scan is what the *phantom* check asks --
    "does this name appear anywhere in the crate at all?". The narrower,
    crate-root scan is what the *missing* check asks, because a private
    module's item is not something the facade has to name.
    """
    names = set(re.findall(r"\bpub\s+(?:(?:default|const|async|unsafe)\s+)*fn\s+([a-z_][a-z0-9_]*)", src))
    names |= set(re.findall(r"\bpub\s+const\s+([A-Z][A-Z0-9_]*)\s*:", src))
    names |= set(re.findall(r"\bpub\s+(?:struct|enum|trait)\s+([A-Z][A-Za-z0-9_]*)", src))
    return names


def crate_api(src_dir: str) -> tuple[set[str], set[str], dict[str, dict[str, set[str]]], set[str]]:
    """The plugin crate's public API: free functions, constants and types.

    The last set is the wider "is this name anywhere in the crate" scan.
    """
    lib = read(os.path.join(src_dir, "lib.rs"))
    functions = public_functions(lib)
    consts = public_consts(lib)
    types = public_types(lib)
    anywhere = wide_names(lib)

    for match in REEXPORT_RE.finditer(lib):
        for part in match.group(2).split(","):
            part = part.strip().split(" as ")[0].strip()
            if part:
                functions.add(part)
                anywhere.add(part)
    for match in GLOB_RE.finditer(lib):
        module = read(os.path.join(src_dir, match.group(1) + ".rs"))
        functions |= public_functions(module)
        consts |= public_consts(module)
        anywhere |= wide_names(module)
        for name, info in public_types(module).items():
            for key in ("variants", "fields", "methods"):
                types.setdefault(name, {"variants": set(), "fields": set(), "methods": set()})[
                    key
                ] |= info[key]
    return functions, consts, types, anywhere


def dts_api(source: str) -> dict[str, object]:
    """The facade's API: exported functions, constants, classes, interfaces, aliases."""
    functions = set(re.findall(r"export\s+function\s+([a-z_][A-Za-z0-9_]*)", source))
    # A constant's name is usually SCREAMING_CASE, so it does not have to start
    # lower like a function's.
    constants = set(re.findall(r"export\s+const\s+([A-Za-z_][A-Za-z0-9_]*)", source))
    classes: dict[str, set[str]] = {}
    for match in re.finditer(r"export\s+class\s+([A-Z][A-Za-z0-9_]*)\s*\{", source):
        body, _ = brace_body(source, match.end() - 1)
        classes[match.group(1)] = set(
            re.findall(r"^\s+(?:static\s+)?([a-z_][A-Za-z0-9_]*)\s*\(", body, re.M)
        )
    interfaces: dict[str, set[str]] = {}
    for match in re.finditer(r"export\s+interface\s+([A-Z][A-Za-z0-9_]*)\s*\{", source):
        body, _ = brace_body(source, match.end() - 1)
        interfaces[match.group(1)] = set(
            re.findall(r"^\s+([a-z_][A-Za-z0-9_]*)\s*[?:]", body, re.M)
        )
    aliases: dict[str, set[str]] = {}
    for match in re.finditer(r"export\s+type\s+([A-Z][A-Za-z0-9_]*)\s*=", source):
        body = type_alias_body(source, match.end())
        aliases[match.group(1)] = set(re.findall(r'"([A-Za-z_][A-Za-z0-9_]*)"', body))
    return {
        "functions": functions,
        "constants": constants,
        "classes": classes,
        "interfaces": interfaces,
        "aliases": aliases,
    }


def report(entry: str, message: str) -> None:
    print(f"FAIL {entry}: {message}", file=sys.stderr)


def main() -> int:
    verbose = "--verbose" in sys.argv
    args = [a for a in sys.argv[1:] if not a.startswith("-")]
    # An optional directory: the plugin root to check. The default is the
    # repository's own plugin workspace; a scaffolded plugin can be pointed at
    # to check that what `floom create --plugin` writes is already conforming.
    root = Path(args[0]) if args else Path(PLUGINS)
    failures = 0
    checked = 0

    for entry in sorted(os.listdir(root)):
        directory = os.path.join(root, entry)
        if not os.path.isdir(directory):
            continue
        if not args and not entry.startswith("floom-plugin-"):
            continue
        if not os.path.isfile(os.path.join(directory, "src", "lib.rs")):
            continue
        checked += 1
        rust_fns, rust_consts, rust_types, anywhere = crate_api(
            os.path.join(directory, "src")
        )
        source = read(os.path.join(directory, "index.d.ts"))
        facade = dts_api(source)
        dts_fns: set[str] = facade["functions"]
        dts_consts: set[str] = facade["constants"]
        classes: dict[str, set[str]] = facade["classes"]
        interfaces: dict[str, set[str]] = facade["interfaces"]
        aliases: dict[str, set[str]] = facade["aliases"]

        # The names the facade offers, against the names the crate has
        # somewhere. A crate-root function answers for itself; an associated
        # function answers for its type, so the facade may expose it at module
        # level.
        crate_names = set(rust_fns) | set(rust_consts) | anywhere
        for info in rust_types.values():
            crate_names |= info["methods"]
        phantom = sorted((dts_fns | dts_consts) - crate_names)
        if phantom:
            failures += 1
            report(entry, f"index.d.ts offers {', '.join(phantom)}, which the crate does not export")

        # A crate-root function the facade does not mention.
        missing = sorted(rust_fns - dts_fns)
        if missing:
            failures += 1
            report(
                entry,
                f"index.d.ts does not declare the Rust API function(s) {', '.join(missing)}",
            )

        # A crate-root constant the facade does not mention.
        missing_consts = sorted(rust_consts - dts_consts)
        if missing_consts:
            failures += 1
            report(
                entry,
                f"index.d.ts does not declare the Rust API constant(s) {', '.join(missing_consts)}",
            )

        for name, info in sorted(rust_types.items()):
            variants = info["variants"]
            fields = info["fields"]
            methods = info["methods"]
            if name in classes:
                declared = classes[name]
                absent = sorted(methods - declared)
                invented = sorted(declared - methods)
                if absent:
                    failures += 1
                    report(entry, f"{name} does not declare the Rust method(s) {', '.join(absent)}")
                if invented:
                    failures += 1
                    report(entry, f"{name} declares {', '.join(invented)}, which the crate does not implement")
            elif name in interfaces:
                declared = interfaces[name]
                absent = sorted(fields - declared)
                invented = sorted(declared - fields)
                if absent:
                    failures += 1
                    report(entry, f"{name} does not declare the Rust field(s) {', '.join(absent)}")
                if invented:
                    failures += 1
                    report(entry, f"{name} declares {', '.join(invented)}, which the crate does not have")
            elif name in aliases and variants:
                absent = sorted(variants - aliases[name])
                invented = sorted(aliases[name] - variants)
                if absent:
                    failures += 1
                    report(entry, f"{name} does not list the Rust variant(s) {', '.join(absent)}")
                if invented:
                    failures += 1
                    report(entry, f"{name} lists {', '.join(invented)}, which the crate does not have")

        # A facade may borrow a type from the framework — a native slot is a
        # number — so only a *class* has to be the plugin's own.
        for name in sorted(set(classes) - set(rust_types)):
            failures += 1
            report(entry, f"index.d.ts declares the class {name}, which the crate does not have")

        if verbose and not failures:
            print(f"ok {entry}: {len(rust_fns)} functions, {len(rust_types)} types")

    if failures:
        print(
            f"plugin facades: {failures} disagreement(s) across {checked} plugin(s)",
            file=sys.stderr,
        )
        return 1
    print(f"plugin facades: {checked} plugin(s) mirror their Rust API")
    return 0


if __name__ == "__main__":
    raise SystemExit(main())

#!/usr/bin/env python3
"""Make Footer / SystemBar widgets consume their per-DS component tokens.

Before: height/pad came from `FooterProps`/`SystemBarProps` defaults (56/16 and
24/8) which are Material values, so Cupertino (44/12, 22/6) and Fluent (48/16,
28/8) tokens defined in `MaterialComponentTokens::{cupertino,fluent}()` were
silently bypassed.

After: props are `Option<f32>`; the widget resolves them against
`theme.component(ComponentId::Footer|SystemBar)` INSIDE the style closure, so a
runtime design-system switch re-resolves the size too. Explicit `.height()` /
`.pad()` still win.
"""

import re
import sys
from pathlib import Path

ROOT = Path(__file__).resolve().parent.parent
CRATES = ROOT / "crates"

FOOTER_CRATES = [
    ("material/md-footer", "MdFooter"),
    ("cupertino/cupertino-footer", "CupertinoFooter"),
    ("fluent/fluent-footer", "FluentFooter"),
]
SYSBAR_CRATES = [
    ("material/md-system_bar", "MdSystemBar"),
    ("cupertino/cupertino-system_bar", "CupertinoSystemBar"),
    ("fluent/fluent-system_bar", "FluentSystemBar"),
]

FOOTER_HELPER = '''
fn footer_tokens(theme: &MultiTheme) -> FooterTokens {
    match theme.component(ComponentId::Footer) {
        ComponentTokens::Footer(t) => t,
        _ => unreachable!("ComponentId::Footer must resolve to FooterTokens"),
    }
}
'''

SYSBAR_HELPER = '''
fn system_bar_tokens(theme: &MultiTheme) -> SystemBarTokens {
    match theme.component(ComponentId::SystemBar) {
        ComponentTokens::SystemBar(t) => t,
        _ => unreachable!("ComponentId::SystemBar must resolve to SystemBarTokens"),
    }
}
'''


def must_replace(src: str, old: str, new: str, what: str, path: str) -> str:
    if old not in src:
        raise SystemExit(f"FAIL[{path}]: anchor not found for {what}:\n{old}")
    if src.count(old) != 1:
        raise SystemExit(
            f"FAIL[{path}]: anchor for {what} occurs {src.count(old)}x (want 1)"
        )
    return src.replace(old, new)


def patch_footer(path: Path, ty: str) -> None:
    src = path.read_text()
    p = str(path.relative_to(ROOT))

    # 1. imports
    src = must_replace(
        src,
        "use floem_core_theme::props::footer::FooterProps;",
        "use floem_core_theme::component_tokens::{ComponentId, ComponentTokens, FooterTokens};\n"
        "use floem_core_theme::props::footer::FooterProps;",
        "imports",
        p,
    )

    # 2. setters -> Some(..)
    src = must_replace(
        src, "self.props.height = height;", "self.props.height = Some(height);", "height setter", p
    )
    src = must_replace(
        src, "self.props.pad = pad;", "self.props.pad = Some(pad);", "pad setter", p
    )

    # 3. resolved_style(): resolve against tokens
    src = must_replace(
        src,
        f"        let theme = self.theme.get();\n        {ty}Style {{",
        f"        let theme = self.theme.get();\n"
        f"        let tokens = footer_tokens(&theme);\n"
        f"        {ty}Style {{",
        "resolved_style token lookup",
        p,
    )
    src = must_replace(
        src,
        "            height: self.props.height,\n            pad: self.props.pad,",
        "            height: self.props.height.unwrap_or(tokens.height),\n"
        "            pad: self.props.pad.unwrap_or(tokens.pad),",
        "resolved_style fields",
        p,
    )

    # 4. view(): capture Option props, resolve for register_layout
    src = must_replace(
        src,
        "        let height = props.height;\n"
        "// layout-provider auto-register",
        "        let prop_height = props.height;\n"
        "        let prop_pad = props.pad;\n"
        "        let height = prop_height.unwrap_or_else(|| footer_tokens(&theme.get()).height);\n"
        "        // layout-provider auto-register",
        "view register_layout",
        p,
    )
    src = must_replace(
        src, "        let pad = props.pad;\n", "", "drop old pad binding", p
    )

    # 5. style closure: resolve tokens reactively
    src = must_replace(
        src,
        "            let t = theme.get();\n"
        "            let mut s = s\n"
        "                .width_pct(100.0)\n"
        "                .height(height)\n"
        "                .padding(pad)",
        "            let t = theme.get();\n"
        "            let tk = footer_tokens(&t);\n"
        "            let mut s = s\n"
        "                .width_pct(100.0)\n"
        "                .height(prop_height.unwrap_or(tk.height))\n"
        "                .padding(prop_pad.unwrap_or(tk.pad))",
        "style closure sizes",
        p,
    )
    src = must_replace(
        src, ".border_top(1.0);", ".border_top(tk.border);", "border thickness", p
    )

    # 6. helper fn at end of file
    if "fn footer_tokens(" not in src:
        src = src.rstrip("\n") + "\n" + FOOTER_HELPER
    path.write_text(src)
    print(f"OK footer: {p}")


def patch_sysbar(path: Path) -> None:
    src = path.read_text()
    p = str(path.relative_to(ROOT))

    src = must_replace(
        src,
        "use floem_core_theme::props::system_bar::SystemBarProps;",
        "use floem_core_theme::component_tokens::{ComponentId, ComponentTokens, SystemBarTokens};\n"
        "use floem_core_theme::props::system_bar::SystemBarProps;",
        "imports",
        p,
    )
    src = must_replace(
        src, "self.props.height = height;", "self.props.height = Some(height);", "height setter", p
    )
    src = must_replace(
        src, "self.props.pad = pad;", "self.props.pad = Some(pad);", "pad setter", p
    )

    # resolved_style: insert token lookup right after `let theme = self.theme.get();`
    src = must_replace(
        src,
        "        let theme = self.theme.get();\n        let (surface, content)",
        "        let theme = self.theme.get();\n"
        "        let tokens = system_bar_tokens(&theme);\n"
        "        let (surface, content)",
        "resolved_style token lookup",
        p,
    )
    src = must_replace(
        src,
        "            height: self.props.height,\n            pad: self.props.pad,",
        "            height: self.props.height.unwrap_or(tokens.height),\n"
        "            pad: self.props.pad.unwrap_or(tokens.pad),",
        "resolved_style fields",
        p,
    )

    src = must_replace(
        src,
        "        let height = props.height;\n"
        "// layout-provider auto-register",
        "        let prop_height = props.height;\n"
        "        let prop_pad = props.pad;\n"
        "        let height = prop_height.unwrap_or_else(|| system_bar_tokens(&theme.get()).height);\n"
        "        // layout-provider auto-register",
        "view register_layout",
        p,
    )
    src = must_replace(
        src, "        let pad = props.pad;\n", "", "drop old pad binding", p
    )

    src = must_replace(
        src,
        "            s.width_pct(100.0)\n"
        "                .height(height)\n"
        "                .padding(pad)",
        "            let tk = system_bar_tokens(&t);\n"
        "            s.width_pct(100.0)\n"
        "                .height(prop_height.unwrap_or(tk.height))\n"
        "                .padding(prop_pad.unwrap_or(tk.pad))",
        "style closure sizes",
        p,
    )

    if "fn system_bar_tokens(" not in src:
        src = src.rstrip("\n") + "\n" + SYSBAR_HELPER
    path.write_text(src)
    print(f"OK system_bar: {p}")


def main() -> int:
    for rel, ty in FOOTER_CRATES:
        patch_footer(CRATES / rel / "src" / "lib.rs", ty)
    for rel, _ty in SYSBAR_CRATES:
        patch_sysbar(CRATES / rel / "src" / "lib.rs")
    return 0


if __name__ == "__main__":
    sys.exit(main())

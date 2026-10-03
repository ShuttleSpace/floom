#!/usr/bin/env python3
"""Replace literal Unicode glyph icons with SVG icon functions in cupertino & fluent
component crates, mirroring the Material md_icon_svg fix.

Idempotent + indentation-agnostic: detects each block's base indent from the source,
so it works regardless of nesting depth. Safe to re-run (skips already-converted files).
"""
import os
import re

ROOT = "/Users/ahs/Project/Rust/floem-ui/crates"
DOWN = "expand_more"
RIGHT = "chevron_right"
CHECK = "check"
CLOSE = "close"
EDIT = "edit"
STAR = "star"


def fix_select_family(out, ds):
    """select/autocomplete/combobox: Label::new(\"▾\") -> svg(expand_more) + font_size->size."""
    fn = f"{ds}_icon_svg"
    out = out.replace('let chevron = Label::new("▾")', f'let chevron = svg(move || {fn}("{DOWN}"))')
    out = re.sub(r'\.font_size\((\d+\.\d+)\)(\s*)\.cursor\(CursorStyle::Pointer\)',
                 r'.size(\1, \1)\2.cursor(CursorStyle::Pointer)', out)
    out = out.replace('.font_size(12.0)\n', '.size(12.0, 12.0)\n')
    return out


def fix_glyph(out, ds, cond):
    fn = f"{ds}_icon_svg"
    m = re.search(r'^(?P<ind>[ \t]*)let glyph = Label::derived\(move \|\| \{', out, re.M)
    if not m:
        return out
    base = m.group('ind'); b4 = base + '    '; b8 = base + '        '
    old = (f'{base}let glyph = Label::derived(move || {{\n'
           f'{b4}if {cond} {{\n'
           f'{b8}"▾".to_string()\n'
           f'{b4}}} else {{\n'
           f'{b8}"▸".to_string()\n'
           f'{b4}}}\n'
           f'{base}}})\n'
           f'{base}.style(move |s| {{\n'
           f'{b4}let t = theme.get();\n'
           f'{b4}s.color(t.colors().on_surface_variant.to_peniko())\n'
           f'{b8}.font_size(16.0)\n'
           f'{base}}});')
    new = (f'{base}let glyph = svg(move || {{\n'
           f'{b4}if {cond} {{\n'
           f'{b8}{fn}("{DOWN}")\n'
           f'{b4}}} else {{\n'
           f'{b8}{fn}("{RIGHT}")\n'
           f'{b4}}}\n'
           f'{base}}})\n'
           f'{base}.style(move |s| {{\n'
           f'{b4}let t = theme.get();\n'
           f'{b4}s.size(16.0, 16.0).color(t.colors().on_surface_variant.to_peniko())\n'
           f'{base}}});')
    if old in out:
        out = out.replace(old, new)
    return out


def fix_tree(out, ds):
    fn = f"{ds}_icon_svg"
    m = re.search(r'^(?P<ind>[ \t]*)let chevron = if expanded \{ "▾" \} else \{ "▸" \};', out, re.M)
    if not m:
        return out
    base = m.group('ind'); b4 = base + '    '
    old = (f'{base}let chevron = if expanded {{ "▾" }} else {{ "▸" }};\n'
           f'{base}Label::new(chevron)\n'
           f'{b4}.style(move |s| {{\n'
           f'{b4}s.width(expander_size)\n'
           f'{b4}    .height(expander_size)\n'
           f'{b4}    .items_center()\n'
           f'{b4}    .justify_center()\n'
           f'{b4}    .color(theme.get().colors().on_surface_variant.to_peniko())\n'
           f'{b4}    .font_size(12.0)\n'
           f'{b4}    .cursor(CursorStyle::Pointer)\n'
           f'{base}}})')
    new = (f'{base}let chevron_name = if expanded {{ "{DOWN}" }} else {{ "{RIGHT}" }};\n'
           f'{b4}svg(move || {fn}(chevron_name))\n'
           f'{b4}.style(move |s| {{\n'
           f'{b4}s.width(expander_size)\n'
           f'{b4}    .height(expander_size)\n'
           f'{b4}    .items_center()\n'
           f'{b4}    .justify_center()\n'
           f'{b4}    .color(theme.get().colors().on_surface_variant.to_peniko())\n'
           f'{b4}    .cursor(CursorStyle::Pointer)\n'
           f'{base}}})')
    if old in out:
        out = out.replace(old, new)
    return out


def fix_check(out, ds, sz_expr):
    fn = f"{ds}_icon_svg"
    m = re.search(r'^(?P<ind>[ \t]*)let mark = Label::derived\(move \|\| \{', out, re.M)
    if not m:
        return out
    base = m.group('ind'); b4 = base + '    '; b8 = base + '        '
    old = (f'{base}let mark = Label::derived(move || {{\n'
           f'{b4}if checked.get() {{\n'
           f'{b8}"✓".to_string()\n'
           f'{b4}}} else {{\n'
           f'{b8}String::new()\n'
           f'{b4}}}\n'
           f'{base}}})\n'
           f'{base}.style(move |s| {{\n'
           f'{b4}let t = theme.get();\n'
           f'{b4}let mk = if checked.get() {{\n'
           f'{b8}t.colors().on_primary\n'
           f'{b4}}} else {{\n'
           f'{b8}Color::rgba(0, 0, 0, 0)\n'
           f'{b4}}};\n'
           f'{b4}let mk = if disabled {{\n'
           f'{b8}with_alpha(t.colors().on_surface, 38)\n'
           f'{b4}}} else {{\n'
           f'{b8}mk\n'
           f'{b4}}};\n'
           f'{b4}s.color(mk.to_peniko())\n'
           f'{b8}.font_size({sz_expr})\n'
           f'{b8}.font_weight(FontWeight::Bold.to_floem())\n'
           f'{base}}});')
    new = (f'{base}let mark = svg(move || {fn}("{CHECK}")).style(move |s| {{\n'
           f'{b4}let t = theme.get();\n'
           f'{b4}let mk = if checked.get() {{\n'
           f'{b8}t.colors().on_primary\n'
           f'{b4}}} else {{\n'
           f'{b8}Color::rgba(0, 0, 0, 0)\n'
           f'{b4}}};\n'
           f'{b4}let mk = if disabled {{\n'
           f'{b8}with_alpha(t.colors().on_surface, 38)\n'
           f'{b4}}} else {{\n'
           f'{b8}mk\n'
           f'{b4}}};\n'
           f'{b4}s.size({sz_expr}).color(mk.to_peniko())\n'
           f'{base}}});')
    if old in out:
        out = out.replace(old, new)
    return out


def fix_rating(out, ds):
    fn = f"{ds}_icon_svg"
    m = re.search(r'^(?P<ind>[ \t]*)let star = Container::new\(Label::new\("★"\.to_string\(\)\)\)', out, re.M)
    if not m:
        return out
    base = m.group('ind'); b4 = base + '    '; b8 = base + '        '
    old = (f'{base}let star = Container::new(Label::new("★".to_string()))\n'
           f'{b4}.style(move |s| {{\n'
           f'{b4}let v = value.get();\n'
           f'{b4}let t = theme.get();\n'
           f'{b4}let c = if (i as f32) < v {{\n'
           f'{b8}Color::rgb(255, 204, 0)\n'
           f'{b4}}} else {{\n'
           f'{b8}t.colors().outline\n'
           f'{b4}}};\n'
           f'{b4}s.font_size(star_size).color(c.to_peniko())\n'
           f'{b4}}})')
    new = (f'{base}let star = svg(move || {fn}("{STAR}"))\n'
           f'{b4}.style(move |s| {{\n'
           f'{b4}let v = value.get();\n'
           f'{b4}let t = theme.get();\n'
           f'{b4}let c = if (i as f32) < v {{\n'
           f'{b8}Color::rgb(255, 204, 0)\n'
           f'{b4}}} else {{\n'
           f'{b8}t.colors().outline\n'
           f'{b4}}};\n'
           f'{b4}s.size(star_size, star_size).color(c.to_peniko())\n'
           f'{b4}}})')
    if old in out:
        out = out.replace(old, new)
    return out


def fix_stepper(out, ds):
    fn = f"{ds}_icon_svg"
    m = re.search(r'^(?P<ind>[ \t]*)let marker = Container::new\(Label::new\(if completed \{', out, re.M)
    if not m:
        return out
    base = m.group('ind'); b4 = base + '    '; b8 = base + '        '
    old = (f'{base}let marker = Container::new(Label::new(if completed {{\n'
           f'{b4}"✓".to_string()\n'
           f'{b4}}} else {{\n'
           f'{b4}format!("{{}}", i + 1)\n'
           f'{b4}}}))\n'
           f'{base}.style(move |s| {{\n'
           f'{b4}let t = theme_m.get();\n'
           f'{b4}let done = sel_m.get() > i || sel_m.get() == i;\n'
           f'{b4}let (bg, fg, bd) = if done {{\n'
           f'{b8}(t.colors().primary, t.colors().on_primary, t.colors().primary)\n'
           f'{b4}}} else {{\n'
           f'{b8}(t.colors().surface, t.colors().on_surface_variant, t.colors().outline)\n'
           f'{b4}}};\n'
           f'{b4}s.width(step_size)')
    new = (f'{base}let marker_inner: floem::views::AnyView = if completed {{\n'
           f'{b4}svg(move || {fn}("{CHECK}"))\n'
           f'{b4}    .style(move |s| {{\n'
           f'{b4}        let t = theme_m.get();\n'
           f'{b4}        let done = sel_m.get() > i || sel_m.get() == i;\n'
           f'{b4}        let fg = if done {{\n'
           f'{b4}            t.colors().on_primary\n'
           f'{b4}        }} else {{\n'
           f'{b4}            t.colors().on_surface_variant\n'
           f'{b4}        }};\n'
           f'{b4}        s.size(step_size * 0.55, step_size * 0.55).color(fg.to_peniko())\n'
           f'{b4}    }})\n'
           f'{b4}    .into_any()\n'
           f'{b4}}} else {{\n'
           f'{b4}    Label::new(format!("{{}}", i + 1)).into_any()\n'
           f'{b4}}};\n'
           f'{base}let marker = Container::new(marker_inner)\n'
           f'{base}.style(move |s| {{\n'
           f'{b4}let t = theme_m.get();\n'
           f'{b4}let done = sel_m.get() > i || sel_m.get() == i;\n'
           f'{b4}let (bg, fg, bd) = if done {{\n'
           f'{b8}(t.colors().primary, t.colors().on_primary, t.colors().primary)\n'
           f'{b4}}} else {{\n'
           f'{b8}(t.colors().surface, t.colors().on_surface_variant, t.colors().outline)\n'
           f'{b4}}};\n'
           f'{b4}s.width(step_size)')
    if old in out:
        out = out.replace(old, new)
    return out


def fix_confirm(out, ds):
    fn = f"{ds}_icon_svg"
    repls = [
        ('Container::new(Label::new("✓"))',
         f'Container::new(svg(move || {fn}("{CHECK}")).style(move |s| {{ let t = theme.get(); s.size(16.0, 16.0).color(t.colors().primary.to_peniko()) }}))'),
        ('Container::new(Label::new("✗"))',
         f'Container::new(svg(move || {fn}("{CLOSE}")).style(move |s| {{ let t = theme.get(); s.size(16.0, 16.0).color(t.colors().on_surface_variant.to_peniko()) }}))'),
        ('Container::new(Label::new("✎"))',
         f'Container::new(svg(move || {fn}("{EDIT}")).style(move |s| {{ let t = theme.get(); s.size(16.0, 16.0).color(t.colors().on_surface_variant.to_peniko()) }}))'),
    ]
    for old, new in repls:
        if old in out:
            out = out.replace(old, new)
    return out


def process(ds, comp, text):
    if comp in ("select", "autocomplete", "combobox"):
        return fix_select_family(text, ds)
    if comp in ("list_group", "expansion_panels"):
        cond = "expanded.get()" if comp == "list_group" else "expanded.get()[i]"
        return fix_glyph(text, ds, cond)
    if comp == "treeview":
        return fix_tree(text, ds)
    if comp in ("checkbox", "checkbox_btn"):
        sz = "size * 0.7, size * 0.7" if comp == "checkbox" else "tk.font_size * 1.2, tk.font_size * 1.2"
        return fix_check(text, ds, sz)
    if comp == "rating":
        return fix_rating(text, ds)
    if comp == "stepper":
        return fix_stepper(text, ds)
    if comp == "confirm_edit":
        return fix_confirm(text, ds)
    return text


def add_dep(cargo_text, ds):
    dep = f"floem-widget-{ds}-icon = {{ workspace = true }}"
    if dep in cargo_text:
        return cargo_text
    return cargo_text.replace(
        "floem-core-theme = { workspace = true }",
        "floem-core-theme = { workspace = true }\n" + dep, 1)


def add_imports(lib_text, ds):
    imp = f"use floem_widget_{ds}_icon::{ds}_icon_svg;"
    if "use floem::views::" in lib_text and "svg" not in lib_text:
        def repl(mm):
            body = mm.group(1)
            parts = [p.strip() for p in body.split(",") if p.strip()]
            if "svg" not in parts:
                parts.insert(0, "svg")
            return "use floem::views::{" + ", ".join(parts) + "}"
        lib_text = re.sub(r"use floem::views::\{([^}]*)\}", repl, lib_text, count=1)
    if imp not in lib_text:
        if "use floem::prelude::*;" in lib_text:
            lib_text = lib_text.replace("use floem::prelude::*;", "use floem::prelude::*;\n" + imp, 1)
        else:
            lib_text = imp + "\n" + lib_text
    return lib_text


def main():
    targets = {
        "cupertino": ["select", "autocomplete", "combobox", "list_group", "expansion_panels",
                      "treeview", "checkbox", "checkbox_btn", "rating", "stepper", "confirm_edit"],
        "fluent": ["select", "autocomplete", "combobox", "list_group", "expansion_panels",
                   "treeview", "checkbox", "checkbox_btn", "rating", "stepper", "confirm_edit"],
    }
    report = []
    for ds, comps in targets.items():
        for comp in comps:
            path = os.path.join(ROOT, ds, f"{ds}-{comp}", "src", "lib.rs")
            with open(path, "r", encoding="utf-8") as f:
                text = f.read()
            before = text
            new_text = process(ds, comp, text)
            new_text = add_imports(new_text, ds)
            if new_text != before:
                with open(path, "w", encoding="utf-8") as f:
                    f.write(new_text)
                report.append(f"CHANGED {ds}-{comp}")
            else:
                report.append(f"noop   {ds}-{comp}")
            cargo_path = os.path.join(ROOT, ds, f"{ds}-{comp}", "Cargo.toml")
            with open(cargo_path, "r", encoding="utf-8") as f:
                ctext = f.read()
            cnew = add_dep(ctext, ds)
            if cnew != ctext:
                with open(cargo_path, "w", encoding="utf-8") as f:
                    f.write(cnew)
    for ds in ("cupertino", "fluent"):
        icon_lib = os.path.join(ROOT, ds, f"{ds}-icon", "src", "lib.rs")
        with open(icon_lib, "r", encoding="utf-8") as f:
            t = f.read()
        fn = f"{ds}_icon_svg"
        if f"pub use icons::{fn};" not in t:
            t = t.replace("mod icons;", f"mod icons;\npub use icons::{fn};", 1)
            with open(icon_lib, "w", encoding="utf-8") as f:
                f.write(t)
            report.append(f"CHANGED {ds}-icon re-export")
        else:
            report.append(f"noop   {ds}-icon re-export")
    for r in report:
        print(r)


if __name__ == "__main__":
    main()

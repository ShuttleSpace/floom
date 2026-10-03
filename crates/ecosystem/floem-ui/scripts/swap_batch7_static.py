#!/usr/bin/env python3
"""Replace the 4 Batch-7 static gallery blocks with showcase calls.

Targets (name -> showcase fn):
  VirtualScroller -> virtual_scroller_showcase
  App             -> app_showcase
  Main            -> main_showcase
  InfiniteScroller-> infinite_scroller_showcase

Strategy: locate the `// N. Name` comment, then the following `pages.push((`,
then balance-match the closing `));` (combined ()[]{} depth) and replace the
whole push with a one-liner Box::new(<fn>).
"""
import io
import sys

PATH = "examples/demo/src/gallery.rs"

TARGETS = [
    ("VirtualScroller", "virtual_scroller_showcase"),
    ("App", "app_showcase"),
    ("Main", "main_showcase"),
    ("InfiniteScroller", "infinite_scroller_showcase"),
]


def find_balanced(text, start):
    """`start` points at the first char AFTER `pages.push(`. Depth is 2 (the
    two open parens of `((`). Return index just past the matching `));`."""
    depth = 2  # we are inside pages.push((
    i = start
    n = len(text)
    while i < n:
        c = text[i]
        if c in "([{":
            depth += 1
        elif c in ")]}":
            depth -= 1
            if depth == 0:
                return i + 1  # past the closing char
        i += 1
    raise RuntimeError("unbalanced at end")


def main():
    with io.open(PATH, "r", encoding="utf-8") as f:
        src = f.read()

    for name, fn in TARGETS:
        # anchor on the page key literal: "Name".to_string(),
        key = '"' + name + '".to_string(),'
        ki = src.find(key)
        if ki == -1:
            sys.stderr.write(f"WARN: key not found for {name}\n")
            continue
        # find the enclosing pages.push((( before the key
        pp = src.rfind("pages.push((", 0, ki)
        if pp == -1:
            sys.stderr.write(f"WARN: pages.push not found for {name}\n")
            continue
        # also locate the leading comment line(s) for this block
        # find the last "// " comment that precedes pp on its own line
        comment_line_start = pp
        # walk backwards over blank/comment lines to include the `// N. Name` marker
        scan = pp
        while scan > 0:
            nl = src.rfind("\n", 0, scan)
            line = src[nl + 1 : scan].strip()
            if line.startswith("//") or line == "":
                comment_line_start = nl + 1
                scan = nl
            else:
                break
        # start right after pages.push((
        start = pp + len("pages.push((")
        end = find_balanced(src, start)
        # Replace from comment_line_start..end with new content
        new_block = (
            src[comment_line_start:pp].rstrip("\n")
            + "\n"
            + "    pages.push((\n"
            + '        "'
            + name
            + '".to_string(),\n'
            + "        Box::new("
            + fn
            + "),\n"
            + "    ));\n"
        )
        src = src[:comment_line_start] + new_block + src[end:]
        sys.stderr.write(f"OK: replaced {name}\n")

    with io.open(PATH, "w", encoding="utf-8") as f:
        f.write(src)
    sys.stderr.write("done\n")


if __name__ == "__main__":
    main()

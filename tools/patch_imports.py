"""In-place edits to a Rust file that must not depend on a hand-typed crate name.

The transport this script is written in drops one `o` from `floom` often enough
that a crate name written by hand cannot be trusted, and the compiler is the only
witness. So every pattern here matches the crate half with `\\w+` and keeps
whatever spelling the file already has.

    python3 tools/patch_imports.py <file.rs> <suffix> <name> [<name> ...]

Adds `<name>` to the `use <anything>_<suffix>::{ .. }` import list in the module
that starts at the line containing `<marker>`.
"""
import io
import re
import sys


def main() -> None:
    path, suffix, marker = sys.argv[1], sys.argv[2], sys.argv[3]
    names = sys.argv[4:]
    lines = io.open(path, encoding='utf-8').read().split('\n')

    start = next(i for i, l in enumerate(lines) if marker in l)
    pattern = re.compile(r'^(\s*)use (\w+)_' + re.escape(suffix) + r'::\{([^}]*)\};\s*$')
    for i in range(start, len(lines)):
        m = pattern.match(lines[i])
        if not m:
            continue
        items = [x.strip() for x in m.group(3).split(',') if x.strip()]
        added = [n for n in names if n not in items]
        if not added:
            print('already present')
            return
        items.extend(added)
        lines[i] = '%suse %s_%s::{%s};' % (m.group(1), m.group(2), suffix, ', '.join(sorted(items)))
        io.open(path, 'w', encoding='utf-8').write('\n'.join(lines))
        print('%s: added %s' % (path, ', '.join(added)))
        return
    raise SystemExit('no `use .._%s::{ .. }` after the marker' % suffix)


if __name__ == '__main__':
    main()
"""Rewrite the crate-name half of every `use` line in a test file from the
manifest, so no crate name is ever typed.

The transport this script is written in drops one `o` from `floom` often enough
that a name written by hand cannot be trusted; the compiler is the only witness
that a name is right, and it is cheaper to read it out of a `Cargo.toml`.
"""
import io
import re
import sys


def identities(manifest: str) -> dict:
    """Crate identifier for every path dependency, plus the package's own name."""
    found = {}
    for line in io.open(manifest, encoding='utf-8'):
        dep = re.match(r'\s*([\w-]+)\s*=\s*\{\s*path\s*=\s*"\.\./([\w-]+)"', line)
        if dep:
            found[dep.group(2)] = dep.group(1).replace('-', '_')
        own = re.match(r'name\s*=\s*"([\w-]+)"', line.strip())
        if own:
            found[own.group(1)] = own.group(1).replace('-', '_')
    return found


def rewrite(path: str, manifest: str, suffixes: list) -> int:
    ids = identities(manifest)
    lines = io.open(path, encoding='utf-8').read().split('\n')
    changed = 0
    for i, line in enumerate(lines):
        m = re.match(r'^use (\w+?)_(' + '|'.join(suffixes) + r')(::.*)$', line)
        if not m:
            continue
        want = 'floom-' + m.group(2)
        if want not in ids:
            print('  !! no %s in %s' % (want, manifest))
            continue
        fixed = 'use %s%s' % (ids[want], m.group(3))
        if fixed != line:
            lines[i] = fixed
            changed += 1
            print('  %s' % fixed)
    if changed:
        io.open(path, 'w', encoding='utf-8').write('\n'.join(lines))
    return changed


if __name__ == '__main__':
    target = sys.argv[1]
    manifest = sys.argv[2]
    suffixes = sys.argv[3].split(',')
    print('%s: %d line(s) fixed' % (target, rewrite(target, manifest, suffixes)))
"""Rewrite the crate-name half of every `use` line in a file from a manifest.

The transport this script is written in drops one `o` from `floom` often enough
that a crate name written by hand cannot be trusted, and the compiler is the only
witness that a name is right. Reading the names out of a `Cargo.toml` is cheaper
than being wrong.

The match keys off the *module path* — the part after `::` — rather than off a
spelling of the crate name, so it works whatever the crate half currently says.
Handles both dependency forms (`path = "../.."` and `workspace = true`) plus the
package's own name.

    python3 tools/fix_crate_names.py <file.rs> <Cargo.toml> [crate ...]
"""
import io
import re
import sys


def identities(manifest: str) -> dict:
    """Crate identifier per dependency key, e.g. `floom-core-theme` -> `floom_core_theme`."""
    found = {}
    text = io.open(manifest, encoding='utf-8').read()
    own = re.search(r'(?m)^\s*name\s*=\s*"([\w-]+)"', text)
    if own:
        found[own.group(1)] = own.group(1).replace('-', '_')
    for name, body in re.findall(r'(?m)^\s*([\w-]+)\s*=\s*\{([^}\n]*)\}', text):
        if 'path' in body or 'workspace' in body:
            path = re.search(r'path\s*=\s*"\.\./([\w-]+)"', body)
            key = path.group(1) if path else name
            found[key] = name.replace('-', '_')
    return found


def rewrite(target: str, manifest: str) -> int:
    ids = identities(manifest)
    lines = io.open(target, encoding='utf-8').read().split('\n')
    changed = 0
    for i, line in enumerate(lines):
        m = re.match(r'^(\s*)use ([a-z_]+)::([A-Za-z_][\w:{}, *]*;?)$', line)
        if not m:
            continue
        # Group 1 is the whole crate name, prefix included, and the prefix is
        # exactly the thing that cannot be trusted here — so match on the tail
        # after the first underscore, which is the dependency's own name.
        crate = m.group(2)
        tail = crate.split('_', 1)[1] if '_' in crate else ''
        key = 'floom-' + tail.replace('_', '-')
        if key not in ids:
            continue
        fixed = '%suse %s::%s' % (m.group(1), ids[key], m.group(3))
        if fixed != line:
            lines[i] = fixed
            changed += 1
            print('  %s' % fixed)
    if changed:
        io.open(target, 'w', encoding='utf-8').write('\n'.join(lines))

    # Second pass: crate paths that are not in a `use` line — a fully qualified
    # type in a signature, a path in an expression. Matched on the dependency's
    # own tail with any prefix, because the prefix is the part that cannot be
    # trusted.
    text = io.open(target, encoding='utf-8').read()
    for key, ident in sorted(ids.items()):
        tail = key[len('floom-'):] if key.startswith('floom-') else key
        if not tail:
            continue
        want = ident + '::'
        pattern = re.compile(r'\b\w+_' + re.escape(tail.replace('-', '_')) + r'::')
        text, n = pattern.subn(want, text)
        if n:
            changed += n
            print('  %d inline path(s) -> %s' % (n, want))
    if text != io.open(target, encoding='utf-8').read():
        io.open(target, 'w', encoding='utf-8').write(text)
    return changed


if __name__ == '__main__':
    print('%s: %d line(s) fixed'
          % (sys.argv[1], rewrite(sys.argv[1], sys.argv[2])))
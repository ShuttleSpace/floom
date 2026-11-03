#!/usr/bin/env python3
"""What does porting the excluded floem-ui kits actually cost?

Answers one question: for each excluded kit, does a widget already exist to
adapt, or does the widget have to be written first?

It exists because the first two attempts at this question were wrong, and
wrong in ways that looked authoritative:

* the first version only looked in `floom-ui-composites`, so it reported
  `md-slider`, `md-radio`, `md-text-field`, `md-grid`, `md-list`, `md-image` as
  needing a new widget -- all of them already exist in `floom-widgets`. A cost
  model that checks one of the two places a widget can live is worse than none.
* the second version matched crate names on `md_` when they are written `md-`,
  so it grouped nothing and reported 0 families.

And the output needs reading, not trusting. "A widget exists" is a shortlist,
not a verdict: `md-slider` matches `Slider` and the port is indeed short, but
its v1 `view()` is 190 lines of hand-built slider, and the engine's `Slider` has
no style type at all -- so porting it means writing that style type first, with
the three kits' disagreement matrix intact. `button_group` matches `Button` and
is nothing like one.

What the survey is good for is ordering the backlog: it separates the families
that are a style type plus three adapters from the ones that are a widget written
from scratch.

Usage:  python3 scripts/survey-excluded-kits.py
"""

import os
import re
import subprocess
import sys

ROOT = os.path.dirname(os.path.dirname(os.path.abspath(__file__)))
UI = os.path.join(ROOT, 'crates', 'ecosystem', 'floom-ui')
COMP = os.path.join(UI, 'crates', 'floom-ui-composites', 'src')

# The prefix is `md-` / `cupertino-` / etc. -- hyphens, not underscores. The
# first version of this script used underscores and silently matched nothing.
KITS = {'md-': 'material', 'cupertino-': 'cupertino', 'fluent-': 'fluent', 'harmony-': 'harmony'}


def read(path):
    with open(path, encoding='utf-8', errors='ignore') as fh:
        return fh.read()


def composites():
    return {f[:-3] for f in os.listdir(COMP)
            if f.endswith('.rs') and f not in ('lib.rs', 'any.rs')}


def engine_widgets():
    """Names re-exported from `floom-widgets`' lib.rs."""
    lib = read(os.path.join(ROOT, 'crates', 'floom-widgets', 'src', 'lib.rs'))
    found = set()
    for m in re.finditer(r'pub use \w+::\{([^}]*)\}', lib, re.S):
        for n in m.group(1).replace('\n', ' ').split(','):
            n = n.strip()
            if n and re.match(r'^[A-Z][A-Za-z0-9]*$', n):
                found.add(n)
    return found


def excluded():
    m = read(os.path.join(UI, 'Cargo.toml'))
    i = m.index('exclude = [')
    j = m.index(']', i)
    return [x.strip().strip('"') for x in m[i:j].replace('\n', '').split(',') if x.strip()]


def is_v1(text):
    return bool(re.search(r'\bfleom::|::prelude|AnyView|impl View|Label::derived|IntoView', text))


def norm(s):
    return s.replace('_', '').replace('-', '').lower()


def main():
    have = composites() | engine_widgets()
    have_norm = {norm(w): w for w in have}

    families = {}
    for c in excluded():
        src = os.path.join(UI, c, 'src', 'lib.rs')
        if not os.path.exists(src):
            continue
        if not is_v1(read(src)):
            continue
        base = c.split('/')[-1]
        comp = ds = None
        for p, d in KITS.items():
            if base.startswith(p):
                comp, ds = base[len(p):], d
                break
        if comp is None:
            continue
        families.setdefault(comp, set()).add(ds)

    covered, uncovered = {}, {}
    for comp, dss in families.items():
        n = norm(comp)
        hits = [w for k, w in have_norm.items() if k and (k in n or n in k)]
        (covered if hits else uncovered)[comp] = (dss, hits)

    out = sys.stdout
    print('a widget exists (cheap: style type + three adapters)', file=out)
    print('-' * 78, file=out)
    for comp, (dss, hits) in sorted(covered.items(), key=lambda kv: (-len(kv[1][0]), kv[0])):
        print('  %-26s %-30s -> %s' % (comp, ','.join(sorted(dss)), ','.join(sorted(hits)[:3])), file=out)

    print(file=out)
    print('no widget exists (write the widget first)', file=out)
    print('-' * 78, file=out)
    for comp, (dss, _) in sorted(uncovered.items()):
        print('  %-26s %s' % (comp, ','.join(sorted(dss))), file=out)

    print(file=out)
    print('component families: %d' % len(families), file=out)
    print('  a widget exists: %d families, %d kits'
          % (len(covered), sum(len(v[0]) for v in covered.values())), file=out)
    print('  none exists:     %d families, %d kits'
          % (len(uncovered), sum(len(v) for v in uncovered.values())), file=out)
    print(file=out)
    print('Read before trusting the "a widget exists" list: a name match is a', file=out)
    print('shortlist. `md-slider` matches `Slider` and its v1 view() is 190 lines;', file=out)
    print('the engine\'s Slider has no style type, so that family starts there.', file=out)


if __name__ == '__main__':
    main()
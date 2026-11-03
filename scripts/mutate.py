#!/usr/bin/env python3
"""Apply one deliberate edit to one file, for mutation-testing a test.

The fourth delivery gate is "a failing test exists", and the only honest way
to satisfy it is to break the code on purpose and watch the *right* test go
red. A test that passes both before and after a mutation is not evidence of
anything, and the reason that is easy to miss is that the mutation itself can
silently fail to apply: a pattern with the wrong indentation, or one that
appears twice, leaves the code untouched. Then "the test still passes" reads
as "the test does not cover this" when the truth is "nothing was changed".

So this refuses to proceed unless the pattern is present *exactly once*:

  scripts/mutate.py <file> <exact-old-text> <new-text>
  cargo test -p <crate> --lib

Restore by keeping a copy first:

  cp f.rs /tmp/f.rs.orig
  scripts/mutate.py f.rs 'old' 'new' && cargo test -p crate --lib
  cp /tmp/f.rs.orig f.rs && cmp -s /tmp/f.rs.orig f.rs && echo restored

The exit codes are meant to be checked: 2 means the mutation did not land, so
the test result that follows it says nothing about coverage.
"""

import pathlib
import sys


def main() -> int:
    if len(sys.argv) != 4:
        print(__doc__, file=sys.stderr)
        return 2

    path = pathlib.Path(sys.argv[1])
    old, new = sys.argv[2], sys.argv[3]
    if old == new:
        print(f"mutation is a no-op: old and new are identical", file=sys.stderr)
        return 2

    src = path.read_text()
    count = src.count(old)
    if count == 0:
        print(f"MUTATION MISSED: pattern not found in {path}", file=sys.stderr)
        return 2
    if count > 1:
        print(
            f"MUTATION AMBIGUOUS: {count} occurrences in {path} — a mutation that "
            f"lands in two places does not tell you which test caught which",
            file=sys.stderr,
        )
        return 2

    path.write_text(src.replace(old, new, 1))
    print(f"mutated {path}")
    return 0


if __name__ == "__main__":
    sys.exit(main())

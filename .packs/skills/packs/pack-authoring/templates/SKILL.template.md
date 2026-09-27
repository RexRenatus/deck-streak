---
name: example-pack
description: >-
  Checks that every Markdown page under docs/ opens with a title and stays short, in any
  repository through --root. Use when adding, reviewing or linting documentation pages.
requires_phxd_schema: phxd.pack.probe.v1
---

# packs/example-pack

Say what the pack judges and why, in two or three sentences, and quote the words of whoever
asked for it. Name the standard or official document behind each block row: a block row states
a firm requirement, and an advisory row a heuristic.

```
phxd pack probe --pack example-pack --root PATH --format json
```

Which seats consume this pack is its catalog row's `consumes`, the one record of that edge, so
this body names none.

## The rows

Two rows, both `tree`-scoped, one per class of the pack's probe script. Each runs
`python3 {skills}/../scripts/example-probe.py --root {root} check <class>` under a 60-second
wall. `{skills}` is the skills directory the pack ships from, and `{root}` is the tree it judges.

The `example` stage: 2 rows (1 block, 1 advisory).

| row | severity | reason | refuses when |
|---|---|---|---|
| `example-title` | block | `title-missing` | a page under `docs/` has no level-one heading |
| `example-length` | advisory | `page-too-long` | a page is over 400 lines; it reports and never refuses |

Every class prints one line per finding and ends with `examined N`, the population it read. It
exits 0 green, 1 on a finding, 2 on a usage error and 3 VOID when it examined nothing. VOID is
never a pass.

## How a repository adopts this

1. Keep the pages under `docs/`, one level-one heading each.
2. Run the pack against the tree with the command above, or run each class directly in the
   repository's own CI: `python3 scripts/example-probe.py --root . check example-title`.
3. Read what refuses: a page with no title. A long page only advises.

## References

Accessed on the date of the research. Record every URL with its access date, and every Context7
library id that answered, in the SPEC's `## References`.

# Schematic: the architecture code map census

Drawn with SPEC-623 and ADR-645, over the two pages as dev held them at the cut.

The census in `scripts/tests/test_architecture_map.py` reads two sources from the tree and two
pages. The crate folders come from the file system under `crates/`, and the `ios/` folders from
`ios/`. ARCHITECTURE.md gives the code map (the table under its `## Code map` heading) and the
drawing (its first mermaid block); docs/CONTEXT-MAP.md gives the fence (each row's name, path,
description and edge list) and its prose. Each page's text is cut into units (a fence row's
description, a table row, a list item, a paragraph), with code spans removed before any match.
Every population passes through `examined` and is refused at zero.

```mermaid
flowchart LR
  crates["crates/ folders"] --> names["every folder named once"]
  codemap["ARCHITECTURE.md code map"] --> names
  codemap --> iosrow["the ios/ row"]
  iosdirs["ios/ folders"] --> iosrow
  codemap --> paths["every code-map path in the tree or planned"]
  fence["docs/CONTEXT-MAP.md fence"] --> paths
  archunits["ARCHITECTURE.md units"] --> archdep["Telegram units say deprecated"]
  drawing["ARCHITECTURE.md drawing"] --> twoapps["the two apps and no Telegram node"]
  fence --> mapunits["context-map units"]
  prose["docs/CONTEXT-MAP.md prose"] --> mapunits
  mapunits --> mapdep["Telegram units say deprecated"]
  fence --> harness["the ios-harness row names the app and the harness"]
  iosdirs --> harness
  fence --> pathrows["every fence path row in the tree or planned"]
  names --> findings["findings: folders, rows, units, paths"]
  iosrow --> findings
  paths --> findings
  archdep --> findings
  twoapps --> findings
  mapdep --> findings
  harness --> findings
  pathrows --> findings
```

Each of the eight criteria is one node above, left to right in the order SPEC-623 numbers them,
and each red names what it found: a crate folder, a row, a unit's first line or a path. A unit
passes when it names no Telegram surface or holds the stem `deprecat` (ADR-645 D2); a fence row's
name and edge list are set aside, so a kept identifier such as `deck-streak-bot` is never read as
a mention.

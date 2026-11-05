# The front door, the docs index and the gate's docs check

A data-flow schematic of SPEC-622 (ADR-644). Every `path:line` citation was read at dev `8620664`;
the new paths are SPEC-622's.

## 1. The docs map

A reader enters at `README.md`, which links the docs index; the index links every living doc and
each record folder (R3). `AGENTS.md` is the agent's entry and points at `CLAUDE.md` and the same
index through the README.

```mermaid
flowchart LR
  README["README.md: install, usage, contributing"] --> INDEX["docs/README.md: the docs index"]
  AGENTS["AGENTS.md: the agent's short form"] --> CLAUDE["CLAUDE.md: the full rules"]
  README --> AGENTS
  README --> CONTRIB["CONTRIBUTING.md: the branch model and the merge queue"]
  INDEX --> TUT["Tutorials: the README"]
  INDEX --> HOW["How-to guides: contribute, deploy, run a runbook, release"]
  INDEX --> REF["Reference: style, testing, design, context map, policies"]
  INDEX --> EXP["Explanation: charter, product, architecture"]
  INDEX --> REC["record folders: decisions, specs, specs/planned, schematics, red-first, rulings, changelog.d"]
  HOW --> LIVING["every living doc: a tracked md file outside the record folders, tests and fixtures"]
  REF --> LIVING
  EXP --> LIVING
```

## 2. The gate's flow

`scripts/check.sh` lines 185 to 186 run `python3 -m unittest discover` over `scripts/tests`, so the
new module joins the Python stage with no workflow change (R4). The queue file feeds A3 and A4, so
the merge queue text follows `.mergify.yml`.

```mermaid
flowchart TD
  CHECK["scripts/check.sh: the Python stage"] --> MOD["scripts/tests/test_front_door_docs.py"]
  LS["git ls-files: names only"] --> RULE["the living-doc rule (R3)"]
  RULE --> A1["A1: the index links every living doc and record folder"]
  RULE --> A2["A2: every relative link resolves, anchors by the GitHub rule"]
  QUEUE[".mergify.yml: label, checks, batch size, merge method"] --> READER["a small reader for its block form"]
  READER --> A3["A3: the branch model in CONTRIBUTING.md"]
  READER --> A4["A4: the merge queue in README.md and AGENTS.md"]
  PAGES["README.md, AGENTS.md, CONTRIBUTING.md, docs/README.md"] --> A5["A5: the web app and the iPhone and iPad app"]
  PAGES --> A6["A6: the Mini App and bot only as deprecated"]
  MOD --> A1
  MOD --> A2
  MOD --> A3
  MOD --> A4
  MOD --> A5
  MOD --> A6
```

## 3. What each test opens

Only living docs are opened, and a link's target only when its destination carries a fragment
and the target is a Markdown file; every other target is judged by its name in the tracked list.
A planted tree in a temporary folder takes the same functions with its own name list, so the planted
controls need no git.

---
status: "proposed"
date: "2026-09-29"
decision-makers: "@RexRenatus (owner), the DeckStreak architect"
---

# The box run builds the landing page from the commit it judges, and the built site is never committed

## Context and Problem Statement

ADR-014 gates the landing page with the seo-pipeline and web-launch packs over `web/site/dist`,
and SPEC-056's box run judges a commit by exporting it with `git archive`. A build product is not
in a commit, so today the run finds no site to judge, and SPEC-030 left building it for the run to
#59. The web-launch pack also reads `vitals.json`, which must hold measured values, never invented
ones. How does the judged tree come to hold a built site?

## Decision Drivers

- The verdict judges exactly what a commit names, and `--rev` can judge an older one.
- `vitals.json` holds only what was measured, on the tree being judged.
- A build that fails is never a pass.
- The public gate and the box run build the same site from the same source.

## Considered Options (the alternatives it was chosen against)

- The box run builds `web/site` inside the exported commit, before the box section: chosen,
  because the site it judges is then the commit's own, built and measured in the same run.
- Commit `web/site/dist`: rejected because a build product in the tree drifts from its source, and a
  committed `vitals.json` would read as current long after it was measured.
- Hand the run a directory built elsewhere: rejected because the verdict would then judge a tree no
  commit names.
- Take the site from a CI artifact of the same commit: rejected because an older revision judged
  with `--rev` may have no artifact left, and the run would wait on another gate.

## Decision Outcome

Chosen option: "the box run builds the site in the export", because it is the only option in which
the judged site and its vitals always belong to the judged commit.

- **The build.** When the export holds `web/site/package.json`, the run installs with the frozen
  lockfile, builds `web/site` with the neutral example origin, and runs the site's vitals
  measurement, all inside the export.
- **A failure** leaves no `web/site/dist`, which the run already reads as pending on the issue its
  wiring names, or as VOID and failed by name when it names none.
- **The tree.** `.gitignore` names `web/site/dist/` and `web/site/.astro/`.

### Consequences

- Good, because the page's gates run on every box run from the commit alone.
- Bad, because a box run now installs the web packages, which lengthens it.

### Confirmation

SPEC-139 §3 (the box run's build criteria and the landing rules), §3a, and its rows in
`S13900-S13999`.

## What would make this wrong

- The packs gain a way to build a site themselves: the run would hand them the source instead.
- A site whose build needs a secret: it could not be built in the box run's export, and would need
  its own decision.

## More Information

SPEC-139, SPEC-056, SPEC-030, ADR-014, ADR-030, ADR-069.

---
status: "accepted"
---

# ADR-674: each web route's JavaScript is held to a recorded budget with a ratchet, and the check starts at warn

Decides SPEC-652. Path:
`docs/decisions/ADR-674-each-web-routes-javascript-is-held-to-a-recorded-budget-with-a-ratchet-and-starts-at-warn.md`.
Amends nothing. Cites ADR-450 (a new check starts at warn and moves to fail only by its own
delivery, and its D6 kill criteria), ADR-336 (the web engine's budget, in bytes `gzip -9`) and
ADR-349 (its arithmetic: each file compressed, then summed). Read at dev `b9a08e60`.

## Context

- **The web engine has a budget.** `scripts/web-engine-size.py` holds the module a browser loads,
  its bindings and the owner's companion files to 8000000 bytes `gzip -9 -n`, summed
  (`scripts/web-engine-size.py:38`; SPEC-338 R8, SPEC-511 R11). Its figure is a constant of the
  script, restated apart in each of its two tests and pinned by rows; a change to it is a decision
  of its own.
- **The Mini App's own JavaScript has none.** The app has 23 pages under one root layout
  (SPEC-652 section 1). CI's web job builds it and reads nothing it built.
- **The build's manifest.** The pinned Vite writes a manifest of a build when asked: one entry per
  chunk, keyed by its source path, naming its `file`, its static `imports` and its
  `dynamicImports` as keys of the manifest, and its `css`. The pinned SvelteKit builds each page and
  each layout as a node, an entry of the client build, and preloads the JavaScript and CSS it finds
  for a page while constructing the chunks. Where it leaves the manifest and the generated nodes is
  read from this delivery's first CI run (SPEC-652 section 6).

## Decision Drivers

- No rule already in force is lowered: the web engine's budget, its tests and its rows stay as
  they are.
- A new rule starts at warn and moves to fail after three iterations of the rule review at
  precision of at least 0.8 with at least five hits, by its own delivery; demoting a fail rule is a
  signed ruling (ADR-450's drivers).
- Every check prints what it examined and fails on an empty population, which is VOID, never a
  pass.
- A figure is a measurement: every entry of the budget file names the job whose run measured it.

## Decisions

**D1. The population is every page, by its node (SPEC-652 R1, R2).** The check reads the manifest
the client build leaves under the app, and each entry whose source is a generated node for the
route file that node re-exports. A node of a `+page.svelte` is a route, named by a slash and its
page's folder under `src/routes`, a group's folder included, so no two pages share a name. Every
page under `src/routes` must have its node: a page without one, two nodes for one route, and no
route at all are VOID, so a build whose form the check does not read is refused by name.

**D2. A route's figure is its own static closure, beside the shell (R3, R4).** A route's files are
its page's node and the node of every layout between it and the root layout, with every chunk each
imports statically, transitively: the JavaScript the route needs before it renders. A dynamic
import is not followed. The shell, every other entry (the client's start and app, the root layout
and the error page) with its static closure, loads before any route and is one entry of its own,
`(shell)`; a route's figure leaves the shell's files out. Stylesheets are not counted.

**D3. The unit is the web engine's (R5).** Each file is compressed by `gzip -9 -n`, measured by
gzip itself, and a figure is the sum of its files: the bytes a browser downloads, file by file, as
ADR-349 sums them for the web engine.

**D4. The budget file (R6, R11).** `config/web-route-budgets.json` holds one entry for each route
and one for the shell, each `{"bytes": N, "job": J}`. Its first entries are written by this
delivery's second push from the web job of its first push, which prints each route's figure in a
`NO BUDGET` line. An entry is lowered when the check prints `RATCHET` for it, to the figure that
line names, with the job that printed it; an entry is raised only by the pull request that grows its
route, which says why.

**D5. The findings and the level (R7, R8, R9).** Four findings: over its entry, no entry, an entry
for a route that is gone, and a route more than 1024 bytes under its entry, the ratchet's request
to lower the entry. The check carries a `LEVEL` constant. At `warn` each finding prints a `WARN`
line and the check exits 0; at `fail` each prints a `FAIL` line and it exits 1. VOID exits 2 at
both levels. It lands at `warn`.

**D6. Kill criteria.** The promotion rule of the Decision Drivers: `LEVEL` moves from warn to fail
after three iterations of the rule review at precision of at least 0.8 with at least five hits, by
its own delivery, which also pins the web job's step in a workflow test; demoting it once at fail
is a weakening, a signed ruling.
- **Live precision** is fixes ÷ (fixes + raised entries) over the last 30 merged PRs, where a fix
  answers a finding without raising an entry. Below 0.8 with at least 5 hits, propose a demotion.
- **First hits:** if 2 or more of the rule's first 5 `OVER BUDGET` hits name a route whose own
  files did not change, propose a demotion.
- **Demotion** is a weakening, and goes to the owner as a signed ruling.
- **Tooling:** `examined 0 route(s)`, a manifest or node the check cannot read, or a gzip that is
  absent or fails, is VOID, which blocks at both levels. It is never a pass.

**D7. Not in this delivery.** No move to fail, no workflow-test pin of the step, no figure for a
dynamically imported chunk or the engine Worker's bundle, no stylesheet budget, and no change to
the web engine's budget: SPEC-652 section 5 names each with its issue.

**D8. Tests and rows (SPEC-652 sections 3 and 9).** One test per acceptance criterion, red first:
the red commit holds the tests and a stub that keeps every input but the missing behaviour (it
reads its two options and finds nothing, measures nothing and refuses nothing), so each red is an
assertion on the missing behaviour over a planted build. Rows S65200 to S65220 in
`scripts/mutation-rows.d/S65200-S65299.json`, hand-written over the check's arms, each killed by a
test of its own module by full id.

## What it was chosen against

### D1

- Budgets keyed by a node's number: rejected because a node's number moves whenever a route is
  added or removed, so every entry after it would change.

### D2

- Following dynamic imports: rejected because the client app imports every route's node
  dynamically, so each route's figure would hold every route.
- Counting the shell in every route's figure: rejected because one change to the shell would then
  report every route at once, where its own entry reports it once.
- Counting stylesheets: rejected because the rule is the route's JavaScript, and the app's styles
  come from the sheet the root layout imports, which a figure per route would repeat.

### D3

- Brotli beside gzip: rejected because the web engine's check prints brotli through a second tool
  and gates gzip alone; this check gates one figure and needs no second tool.
- Raw bytes: rejected because a browser downloads the compressed bytes, which the web engine's
  budget measures.

### D4

- Figures from a build on another machine: rejected because its gzip and toolchain can differ from
  the runner's, and it names no job.
- A constant per route in the script, as the web engine's budget is: rejected because every route's
  change would then be an edit of the check's own code and its rows.

### D5

- Failing on an entry for a gone route at warn, as ADR-450 D5's stale listing does: rejected
  because a removed route would then fail CI before the rule's own promotion.
- A margin of zero: rejected because a route that shrank by one byte would ask an edit of the table.

### D6

- Moving to fail in this delivery: rejected because the rule has no review iteration yet, and
  ADR-450's driver moves a rule to fail only by its own delivery after three.

## Consequences

- Good: every route's JavaScript and the shell's are measured on each run of the web job, and a
  route that grows is printed with its figure and its entry.
- Good: the table follows the routes down: a route that shrinks past the margin asks its entry
  lowered, so the next growth is measured from the lower figure.
- Good: a build the check cannot read is refused by name, never passed.
- Bad: at warn a route over its entry passes; it is visible in the web job's output only.
- Bad: a dynamically imported chunk and the engine Worker's bundle are not measured until their
  issue lands.

## Confirmation

SPEC-652's acceptance criteria A1 to A8, each read red then green, or disclosed not red, recorded
in `docs/red-first/SPEC-652.md`. The rows S65200 to S65220. The web job's `web-route-size` step
in the delivery's own CI runs, read by its name and job id.

## What would make this wrong

- The client build leaves its manifest or its nodes in another form. Then the check reads VOID by
  name, and its constants follow the build's form in the same delivery.
- A route's first render needs a file no static import reaches. Then its figure is low, and the
  closure's rule is a new decision.
- The rule review finds more than 1 in 5 hits false. Then its precision falls, and D6 proposes a
  demotion to the owner.

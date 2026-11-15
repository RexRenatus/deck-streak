# ADR-636: Formal entries are checked in a workflow of their own, and the TLA+ models stay hand-run

| field | value |
|---|---|
| status | accepted |
| spec | SPEC-614 |
| issue | #806 |
| builds on | ADR-400 (its confirmation names the Lean entry `lean/ReplayHistory`) |
| measured at | dev `b065fcee` |

## Context

Issue #806 says no CI job checks the Lean proofs under `formal/lean`. Its close condition is a CI
job that checks every formal entry the repository declares, with a pinned toolchain, and reads red
when an entry goes stale, or a recorded decision that formal entries stay hand-checked, naming when
they are run. The facts, each read at `b065fcee`:

- No workflow names Lean, Lake, TLC or a formal check (`git grep` over `.github/workflows`).
- 332 `@phx covers` lines in the Lean and TLA+ entries and the two Kani harness modules, over 98
  covered paths; one of them is stale on dev (`tla/HabitXpFollowsItsLog`, `announce_level_up`).
- 195 declared theorems and witnesses in 31 Lean modules, 21 of them unreachable from the root
  `Formal.lean`, which imports 10; `config/formal.json` allows three axioms.
- 29 TLA+ entries, 12 of them with a budget of their own in `config/formal.json`, the largest
  `tla/OpLog`'s.
- 8 Kani harnesses in 2 modules; no workflow installs a Kani verifier, and `config/formal.json`
  holds no Kani budget at this base.
- The required checks of `.github/rulesets/dev.json` are `ci`, `fragment` and `dependency-review`.

## D1. A workflow of its own, and what it was chosen against

`.github/workflows/formal.yml` holds one job, `formal`, on every pull request into dev or main and
every push to them, with a read-only token, no cache and no secret. It is a check of its own, so a
red in it names the formal entries alone, and whether it becomes required is the owner's ruleset
decision.

Chosen against:
- A job in `ci.yml`: many open pull requests change that file, and the aggregate `ci` check would
  turn a formal red into a red of the whole gate before the owner rules that it should be required.
- A step in an existing `ci.yml` job: the Lean install and build would lengthen a job that already
  runs the gate's stages, and its log would mix two checks.

## D2. Covers read by a port of the rule, and what it was chosen against

`scripts/formal_ci.py covers` reads every cover of every Lean, TLA+ and Kani entry and recomputes
its item's digest by the rule the formal checker stamps with: the span from the definition line to
the item's end, each line followed by a newline, a name defined twice resolving to nothing. The
covers that are stale at the base are listed in `config/formal-stale.json` with both digests, so a
listed cover that moves again, reads fresh, or is declared by no entry is itself a finding.

Chosen against:
- Running the formal checker in CI: it is not published with this repository, and CI would install
  a second toolchain to run one rule.
- No list, reading the stale cover red: the check would be red on dev from its first run, and a red
  that is always there hides the next one.
- A list of entries rather than covers, or of covers without digests: either would hide a further
  move of a listed span.

## D3. The TLA+ models and the Kani harnesses stay hand-run, and what it was chosen against

TLC over the TLA+ models and the Kani verifier over the harnesses are run by the formal checker by
hand, at every delivery that changes a TLA+ entry, a Kani harness module or a span either covers.
CI reads their covers on every pull request, so such a change cannot land silently: a moved span
reads stale by entry, path and anchor. This is the close condition's other arm for those entries.

Chosen against:
- TLC per entry in CI, against the per-entry budgets in `config/formal.json`: 29 entries with
  the per-entry budgets summed would make the job the longest in CI, and the budgets were set
  for the hand checker's machine, not a hosted runner. Not now; a later delivery can add it.
- The Kani harnesses in CI: CI installs no Kani verifier, and at this base `config/formal.json`
  holds no Kani budget, so a Kani job needs its own pinned install and its own budget, which is not
  this delivery. This reads the base commit; an open pull request (#1131) adds a Kani budget to that
  file and may land first.

## D4. The Lean toolchain is the pin, checksum verified, and what it was chosen against

The job installs the Lean release that `formal/lean/lean-toolchain` names, from its exact-version
archive, and refuses it unless its sha256 matches; the step fails before the download when the pin
names another version. It builds every module by `lake build`, then audits the axioms of every
declared theorem and witness through one generated file that imports each declaring module, so the
21 modules the root does not import are audited too.

Chosen against:
- The toolchain manager's install script: it fetches whatever the pin names with no checksum this
  repository records.
- Auditing through `import Formal`: it reaches 10 of the 31 declaring modules.

## Consequences

- Every pull request reads the covers of every formal entry; a moved span is red by name.
- The stale list must shrink: a re-stamp deletes its row, or the check reads red.
- A change to `lean-toolchain` must change the workflow's version and checksum in the same pull
  request, or the job fails.

## Confirmation

- SPEC-614 A1 to A9, each red first, green at the head.
- The `formal` job on the pull request examines every cover, builds every Lean entry and audits
  every declaration.

## What would make this wrong

- The formal checker's span rule changes: the port then reads otherwise, and the reader proof is
  repeated against the new rule.
- A hosted runner checks a TLA+ entry inside its budget: D3's first alternative becomes the cheaper
  way, and the models join CI.
- The owner makes `formal` required: nothing here changes, and the ruleset names it.

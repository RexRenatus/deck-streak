---
status: accepted
date: "2026-09-28"
decision-makers: "@RexRenatus (owner, through the maintainer), the DeckStreak architect"
---

# The equivalence record excuses exactly a recorded mutant

## Context and Problem Statement

The first release pull request judged by mutation testing carries every mutant of the tree in its
merge diff (SPEC-039 R3 and R18), and it fails on every one that survives. The weekly battery's
dispatch run 36384080819 left 310 missed Rust mutants in 33 files and, in the Mini App, 53 survived
and 28 uncovered (#240). The owner ruled on 2026-09-28 that each is killed by a real test or
recorded equivalent with a one-line reason, with no cap and no skip, and that a table per crate
reads 0 unexplained before the release (SPEC-057). ADR-057 D6 records an equivalent as an anchored
`exclude_re` entry in `.cargo/mutants.toml`, or as a `Stryker disable` comment. Where does an
equivalence claim live, what does it carry, and how does CI's verdict read it, so that the verdict
excuses exactly the recorded mutant and a reviewer can audit every claim?

## Decision Drivers

- **Auditability.** An excused mutant should stay in the tool's listing and in every run's report,
  so that each run that reaches it tests the claim again. cargo-mutants filters an `exclude_re`
  match out of its listing (its `--list` previews what the filters leave), and StrykerJS never runs
  a disabled mutant: it reports it `Ignored`.
- **Exactness.** One record excuses one mutant: never a pattern, a file or a count.
- **Stability.** A claim should break when its own code changes, not when unrelated lines move.
  Between the run's tree, 4082551, and `dev` 16ed8e2, 157 of the 310 survivors changed their
  `file:line:column` name, counted by binding each to its mutant, every one because lines moved,
  and none left the listing (SPEC-057 §1).
- **Uniqueness without a position.** Keyed by the file and the tool's description, the fields a
  record carries besides its anchor, 667 of the 2,207 mutants at `dev` 16ed8e2 share their key with
  another mutant, in 188 shared keys; adding the mutated span's text still leaves 665.
- **A countable table.** The table per crate is counted from the tool's own reports and one
  committed record.
- **No collisions.** Deliveries that run at the same time must not write one file, the reason
  ADR-057 D5 split the rows into band fragments.
- **Evidence.** A wrong equivalence claim is the campaign's main risk, so each claim carries what a
  reviewer checks it against.

## Considered Options (the alternatives it was chosen against)

**D1, where a claim lives.**

- A committed record, a fragment per package: chosen, because the tool keeps listing and running
  the mutant, the record is one reviewable file per crate that the table counts, and a crate's
  delivery writes only its own fragment. The fragments are
  `scripts/mutation-equivalent.d/<package>.json`, one named for each Cargo package and
  `miniapp.json` for the Mini App, and `scripts/mutation-verdict.py` reads them.
- One file, `scripts/mutation-equivalent.json`: rejected, because every delivery that records a
  claim would append to the same file, the collision ADR-057 D5 ended for the rows.
- cargo-mutants' `exclude_re` in `.cargo/mutants.toml` (ADR-057 D6): rejected, because it hides the
  mutant from the listing, so the plan, the shards, the reports and the table never see it, and no
  run tests the claim again. A pattern that carries the line breaks when lines move (157 of 310
  survivors moved in one day), and one without the line can excuse more than one mutant.
- `#[mutants::skip]` or `#[mutants::exclude_re]` on the item: rejected, because each hides its
  mutants from the listing too, `skip` hides every mutant of the item at once, and either puts the
  claim in production code with the `mutants` crate as a dependency.
- A `Stryker disable` comment: rejected, because StrykerJS never runs an ignored mutant, so its
  claim is never tested again, and the claim sits in production source, outside the one record the
  table counts.
- A table inside the rows' band fragments (`scripts/mutation-rows.d/`): rejected, because a row is
  a mutant that a named test kills, and the census and the runner prove every row KILLED; an
  equivalent mutant is one that no test kills.

**D2, what identifies the mutant.**

- The file, the description and an anchor: chosen by measurement. The description is the tool's
  own, without its location; the anchor is a text that occurs exactly once in the file, inside whose
  occurrence the mutant's span starts; and the mutated text is added only when two mutants of that
  description start at one position. Over the 310 survivors, a whole-line anchor bound 269 and a
  narrower window the other 41, each to exactly one mutant, at the run's tree and at `dev` 16ed8e2
  alike. Over the Mini App's 81 survived and uncovered mutants, lines bound 71, windows 4, and 6
  needed the mutated text, because a `ConditionalExpression` over `a && b` starts where the one over
  `a` starts. It is the rows' anchor rule (SPEC-039 R8), so the reader's code and the reviewer's
  habit carry over.
- The tool's full name, `file:line:column: description`: rejected by measurement, because 157 of the
  310 survivors' names moved in one day with no change to their code, and each move would break a
  claim the move did not touch. The repair would be a mechanical re-key that re-examines nothing.
  Worse than breaking, a move can re-bind: in `unquote` the two survivors at `rails.rs:846:16` moved
  up three lines and the two from `849:16` took their name, so a record keyed `846:16` would
  silently excuse a different survivor.
- The file and the description, with no position: rejected by measurement, because 667 of the
  2,207 mutants at `dev` 16ed8e2 share that key with another mutant, in 188 shared keys, so a
  record could not say which one it excuses; adding the mutated span's text still leaves 665.

**D3, what a claim carries.**

- A reason, evidence, a test that reaches the code, and an issue: chosen, because a claim is only
  as good as what a reviewer can check. The `reason` is one line, why no test can tell the mutant
  apart; the `evidence` is the code fact that makes it so, stated where a reviewer can check it;
  `reached_by`, in a Rust claim, is a test of the mutant's own package that runs the mutated code,
  resolved like a row's killer; and `issue` is the issue its delivery closes. A missed Rust mutant
  says nothing about whether any test ran its code, and a mutant no test reaches is untested, not
  equivalent. A survived Stryker mutant was run by at least one test, which Stryker's per-test
  coverage reports, so a Mini App claim needs no `reached_by`.
- A reason alone, as `EQUIVALENT: <reason> (#N)` (ADR-057 D6): rejected, because a reason with
  nothing to check it against is a claim nobody reviews.

**D4, how the verdict reads it.**

- The mutant keeps running, and the verdict binds each record to it: chosen, because each run that
  reaches a recorded mutant tests its claim again, and a claim whose code changed, or that a new
  test refutes, fails where that happened. A missed mutant (Stryker: survived) that exactly one
  record binds is excused and counted `equivalent`, apart from `unexplained`. A record whose mutant
  a test caught, or that timed out, is REFUTED; one whose mutant is unviable is UNNEEDED; one bound
  to no listed mutant is STALE, and to two, AMBIGUOUS; an uncovered Stryker mutant is never excused
  (UNCOVERED). Each fails by name. Rust records are bound against the whole tree's listing
  (`cargo mutants --list --json`, which builds nothing) on every run that lists a diff and on every
  battery, while the gate's census holds the fields and the anchors on every pull request.
- Excusing a recorded mutant without running it: rejected, because that is `exclude_re` again, with
  the record in another file.
- Judging a record only when its mutant is in the diff: rejected, because a pull request that moves
  or rewrites a recorded mutant's code would pass while the claim went stale unseen.

**D5, a survivor budget.**

- No budget: chosen, by the owner's ruling (no cap). Every survivor is killed or recorded.
- A numeric budget, N survivors per crate or per run allowed to pass: rejected, because every
  survivor under the budget passes unnamed and unexplained, and nothing holds a budget from growing.

**D6, skipping files.**

- Nothing skipped: chosen, by the owner's ruling (no skip). `.cargo/mutants.toml` holds no
  `exclude_globs`, `examine_globs`, `examine_re` or `skip_calls` key, and SPEC-039 R2's production
  classes stay whole.
- Skipping a hard file, such as `rails.rs` with 145 of the 310: rejected, because it hides every
  mutant of the file, the killable ones included, while the file stays production code that the
  release ships.

## Decision Outcome

Chosen: D1 to D6's first options. SPEC-057 R4 to R14 state them as requirements.

This amends ADR-057 D6 (the form an equivalent is recorded in) and SPEC-039 R5, and nothing else of
either; ADR-057 carries a note that says so. It takes effect when SPEC-057's first delivery, the
vault's, builds the reader and sets this ADR `accepted`. Until then ADR-057 D6 stands, and there is
nothing to migrate: at `dev` 16ed8e2 the tree holds no exclusion, no `mutants::` attribute and no
`Stryker disable` comment (SPEC-057 §1).

### Consequences

- Good, because every excused mutant is still listed, run and reported, so the first test that
  kills it refutes a wrong claim.
- Good, because a claim survives unrelated edits and breaks only when its own code changes, which
  is when it should be examined again.
- Good, because the table per crate is counted from the tool's reports and one record per crate.
- Bad, because an equivalent mutant costs its run time on every run that reaches it (an `ingest`
  mutant costs about two minutes on GitHub's runners, SPEC-039 R18), where an exclusion cost
  nothing.
- Bad, because the reader grows: the census, the binding and five failure states are DeckStreak's
  own code, with their own tests.
- Bad, because a flaky test can read a recorded mutant as caught, which fails as REFUTED until the
  run is repeated.
- Bad, because the census checks that `reached_by` names one test of the mutant's package, not that
  the test runs the mutated code; the reviewer checks that.

### Confirmation

SPEC-057's A1 to A15, red first in the vault's delivery; each crate's opening and closing sweeps;
and A25, the release rehearsal on a synthetic merge of the last delivery's head into `main`.

## What would make this wrong

- A cargo-mutants or StrykerJS release that renames its mutants' descriptions: every record then
  goes STALE when the pin moves, and the records are bound again in the same change as the pin.
- A checker that proves two programs equivalent: a claim could then carry a proof rather than
  evidence.
- The campaign records far more survivors than it kills: the rule would then be a skip in practice,
  and the owner would reconsider it.

## More Information

SPEC-057; SPEC-039 (R2, R3, R5, R8, R18); ADR-057 (D5, D6); ADR-069 (a criterion retired
insert-only); #240. cargo-mutants: https://mutants.rs/filter_mutants.html,
https://mutants.rs/skip.html, https://mutants.rs/attrs.html and https://mutants.rs/workspaces.html.
StrykerJS: https://github.com/stryker-mutator/stryker-js/blob/master/docs/disable-mutants.md.

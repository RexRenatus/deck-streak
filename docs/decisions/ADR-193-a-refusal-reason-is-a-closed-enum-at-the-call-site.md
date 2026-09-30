---
status: "accepted"
date: "2026-09-29"
decision-makers: "@RexRenatus (owner), the DeckStreak architect"
---

# A refusal's reason is a closed enum at the call site

## Context and Problem Statement

SPEC-128 records the reason a sync was refused as one of eight codes. The set was closed where a
reason is stored and read (`RefusalReason`, and a `CHECK` in the migration), but where a reason
was produced it was a string, and a test scanned the sources of the owner cycle for code literals.
A code defined in a new file and recorded through the composition root was not read by that scan
(issue #396), so the criterion passed while the code bypassed it. Where does the set have to be
closed so that an unknown code cannot be produced at all?

## Decision Drivers

- A guard that reads sources has a population, and a new file can leave it.
- The stored strings must not change, so that no migration is needed.
- The context map allows no new dependency edge, and the enum already lives in `deck-streak-ingest`,
  which the daemon (recorder and answer) depends on.

## Considered Options (the alternatives it was chosen against)

- The closed enum at the call site: `refused` and the cycle's error take `RefusalReason`. Chosen because an unknown code then fails to compile, the job records the value it was given with no parse, and a test over the variants replaces the scan.
- Keep the source scan and extend it to every file of every crate. Rejected because a scan still has a population, a producer written in a shape the scan does not match (a constant, a format, a moved module) passes it, and each new file has to be remembered.
- A string newtype with a constant list of codes. Rejected because a newtype built from any string admits an unknown code at run time, and the list is checked by a test, not by the compiler.
- Leave `parse` at the recording site and log the unknown code. Rejected because it is today's behaviour: an unknown code is dropped, the flag stays set and every job run retries the request.

## Decision Outcome

Chosen option: "the closed enum at the call site", because the type is the guard and needs no
population. `RefusalReason` stays in `deck-streak-ingest` (no new edge), `OwnerSyncCycle::run`
returns it as its error, and `parse` remains only for reading a stored row.

### Consequences

- Good, because a code outside the eight is a compile error instead of a scan finding.
- Good, because the silent drop at the recording site is gone.
- Bad, because the bot's own two reasons for a request that could not be asked stay strings: they are not refusals the job records.

### Confirmation

SPEC-128's amendment adds A14 (a test over the variants: each string equals the code stored today,
no two share one, and the cycle's refusal type is the enum), and A13's body reads the migration's
`CHECK` in place of the sources. Rows `S12809` to `S12811` prove the mapping, and rows `S12812` to
`S12828` prove each step's refusal and the step table (SPEC-128 section 7; the amendment below).

## Amendment, 2026-09-30: the refusal names its step (SPEC-128 A16, A17)

### Context and Problem Statement

A refusal's code does not say which step failed. The cycle's one site refuses for eight steps: five
give `recompute_failed` and two give `sync_record_failed`. A test that asserts only the code passes
when its fault trips a sibling step, and a table of (site, code) pairs counts a shared code once. So
a sibling step's failure could be remapped or answered with no test driving it (#396). How does a
test tell, through `run`, which step it made fail?

### Considered Options (the alternatives it was chosen against)

- The step named in the refusal's log: `refused` takes a private `Step`, whose `reason` gives the code and whose name is logged beside it. Chosen because a test then asserts the step as well as the code, a fault that trips a sibling step fails the test instead of passing for it, the test is red at the head by an assertion on the missing name, and no public type, stored column or migration changes.
- A table-driven behaviour test on the code and the answer alone, the code unchanged: rejected because it cannot tell two steps that share a code apart (a window fault that trips the gate passes for the window, and the window's remap then goes undriven), and because it is green at the head, so it has no red for its criterion's reason.
- The step in the public answer or the stored record (a refusal type holding the step and the code, or a new column the job writes): rejected because it widens the daemon's public surface and the job's record, with a migration and a `.sqlx/` change for the column, for a fact only the tests and the log need; its red at the head is also a compile error, not an assertion.
- One step per `CycleError` variant, six at the cycle's site: rejected because the gate and the window each fail in two ways under one code, so a fault for one way passes for the other, and a gate that cannot record its anchor answered with an `Ok`, or a window that cannot be read remapped, goes undriven while the table stays green.
- One step per cause, splitting `KernelError` and `ReadError` as well: rejected because a cause names the store or the copy that failed under any step, not a part of the step, so it adds rows that no step's fault tells apart.
- The test also asserts the cause's text as the log prints it: rejected because two steps print one text (the gate's and the window's read of the copy share `ReadError`'s), and the text belongs to other crates' `Display`, so a rewording there turns the table red with no refusal changed.

### Decision Outcome

Chosen option: "the step named in the refusal's log". `Step` is private to `wiring.rs`. It has four
steps for the reads `run` makes before the cycle, and eight for the cycle's errors, through
`Step::of`: one for each `CycleError` variant whose error is a shared cause, and one for each kind of
the sync's, the gate's and the window's own errors. `Step::of` names every kind, so a kind added to
one of those errors does not compile until it is given a step. `cycle_reason` is removed, and
`Step::reason` gives each step's code.

A16's table iterates the twelve steps with the code SPEC-128 gives each, never read from
`Step::reason`, so a remap in the code is a failure and not a new expectation. It drives each step
twice, and its check fails a step driven once, so a code that changes on a retry is caught. A17
counts the `?` in `run` and the kinds `Step::of` names, so a new step with no row fails.

### Consequences

- Good, because every step is driven through `run` by its own fault, and none is disclosed as unreachable.
- Good, because the refusal's log now names the step that failed, which the code alone did not.
- Bad, because the table's check reads the log. It installs a subscriber and keeps a second dispatcher alive: while only one is registered, `tracing` asks only the reaching thread's, and a callsite another test's thread reached first stays disabled.
- Bad, because A17 reads `wiring.rs` as text: a reshaped `run` (a `?` in a closure, or an arm split over two lines) moves its counts, and the count is fixed in the same change.

## More Information

Issue #396; SPEC-128 (amendment of 2026-09-29); ADR-128; SPEC-059.

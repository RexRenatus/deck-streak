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
`CHECK` in place of the sources. Rows `S12809` to `S12811` prove the mapping.

## More Information

Issue #396; SPEC-128 (amendment of 2026-09-29); ADR-128; SPEC-059.

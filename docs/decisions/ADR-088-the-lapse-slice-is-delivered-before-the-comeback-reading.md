---
status: accepted
date: "2026-09-29"
decision-makers: "@RexRenatus (owner), the DeckStreak architect"
---

# SPEC-049's lapse slice is delivered first, on its own, and SPEC-076 waits for the slice alone

## Context and Problem Statement

SPEC-049 (W1) builds the comeback reading and, ahead of W3's governor, the lapse episode alone:
R12 to R15, proved by A11 to A13 against the golden of the predecessor's lapse detection
(ADR-049). SPEC-076 (W3) extends that slice, and it gates SPEC-073 and SPEC-082, which gate about
45 of the 67 planned SPECs. As one delivery, SPEC-049 lands after SPEC-046, 047, 048, 051 and 052
(its prerequisites and build order), so SPEC-076 is seventh on a serial chain, and a build of it
halted because no `lapse.rs` and no `lapse_episode` golden exist on `dev`.

Measured on `dev` d9fc074: the slice's lines read only delivered work (SPEC-020's `StudyDay` and
study-day rule, SPEC-023's study reviews, SPEC-029's registry and reader, SPEC-041's lapse context,
SPEC-071's window, `economy.json`'s `governor.lapse_after_silent_days`). The predecessor's
`pipeline_layers/governor.py:GovernorLayer._update_governor` finds the lapse from the study days,
the skip days and its stored state, and reads no readings state. No file the slice writes is in
the manifest of SPEC-046, 047, 048, 051 or 052, and SPEC-046's branch shares only `Cargo.lock` with
it. SPEC-076 names no readings SPEC.

## Decision Drivers

- The W3 critical path runs through SPEC-076.
- Nothing another SPEC owns is built early, and SPEC-076 is not built in part.
- A SPEC is promoted with every test its fence names (ADR-016).
- Each delivery's own tests kill its diff's mutants (SPEC-039).
- The references already written (ADR-049, SPEC-041, 076, 080, 081, 083 and three schematics) name
  SPEC-049's slice, and a golden is registered under one SPEC's number.

## Considered Options (the alternatives it was chosen against)

- Split SPEC-049's delivery in two by an insert-only amendment (the slice, R12 to R15 with A11 to A13 and a new A15, first; the remainder after SPEC-052), and amend SPEC-076 to wait for the slice alone — chosen: SPEC-076 can start once one small delivery lands, and every citation stays true.
- Keep the serial chain as written: SPEC-046, 047, 048, 051, 052, 049, then 076 — rejected because SPEC-076 and the SPECs behind it would wait on five readings deliveries whose work the slice never reads.
- Build SPEC-076 in part and defer its lapse lines — rejected because its A13 and A14 can be neither red nor green without the slice's golden and module, its verdict, standby rule and relight all read the open lapse, and the orchestrator refused a partial build.
- Give the slice a new SPEC number — rejected because ADR-049 already decides the slice under SPEC-049, SPEC-076 section 7 has SPEC-049 register `lapse_episode.json` and says it is "not registered again", and eight documents cite SPEC-049's slice; a new number would move the golden to another registry module and rewrite those citations while building the same code.
- Let the slice move SPEC-049 to `docs/specs/` — rejected because the tdd pack's `acceptance-has-a-test` would then find A1 to A10 and A14 without tests on `dev`.

## Decision Outcome

Chosen option. SPEC-049 section 7 names its two deliveries. The slice builds R12 to R14 and R15's
module, adds A15 for that module, writes rows S04901 to S04909 of SPEC-049's band, starts
`docs/red-first/SPEC-049.md` and leaves the SPEC planned. The remainder builds the comeback reading
after SPEC-052, appends its red-first section, accepts ADR-049 and promotes the SPEC. SPEC-076
section 10 reads its prerequisite as the slice, met when its four files are on `dev` and its
criteria pass.

### Consequences

- Good, because SPEC-076 becomes buildable when the slice lands, and the slice can be built now,
  beside SPEC-046's pull request.
- Good, because no file of SPEC-046, 047, 048, 051 or 052 is built early.
- Bad, because the slice's criteria are not judged by the tdd pack until the remainder promotes the
  SPEC; CI still runs their tests, and the diff-scoped mutation run judges the slice.
- Bad, because the remainder, if SPEC-076 lands first, reads the open lapse through a module
  SPEC-076 has extended with the stored anchor.

### Confirmation

SPEC-049's A11, A12, A13 and A15 in the slice's delivery; SPEC-076's A13 and A14 at its build;
SPEC-047's A10 when SPEC-047 lands.

## What would make this wrong

- A slice line is found to read a readings table or a file another planned SPEC adds.
- The owner wants the comeback reading and the lapse episode to land together.

## More Information

SPEC-049 section 7; SPEC-076 section 10; ADR-016; ADR-049; SPEC-057, a planned SPEC built by
several deliveries.

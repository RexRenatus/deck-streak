# ADR-522: The sync protocol is modelled and proved ahead of its code, at report, with an ADR of its own

| field | value |
|---|---|
| status | accepted |
| spec | SPEC-504 |
| builds on | ADR-514 (the formal ledger; not amended) |
| measured at | `d3b09d5e` |

## Context

The sync core that will run the op log is not in the tree. Its protocol, the clock, the review key, the field merge
and the push judge are designed. A model written after the code models what was built; a model written first is a
specification the code is built against. The ledger (ADR-514) already lists the model and the four proofs as planned.

## Decision

1. **Ahead of the code.** `tla/OpLog` and the Lean entries `SyncClock`, `ReviewKey`, `FieldMerge` and `PushJudge`
   land before the sync core, every property and theorem at `ramp=report`. They carry no `covers` line, because no
   span exists to cover; the sync core's delivery stamps each one and adds the Rust tests that read the vectors.
2. **Witnesses are the design's defects.** No code at the merge-base runs the protocol, so each witness switches on a
   recorded design defect (`Bug` in the model; a named defective function in each Lean entry), not a port of old code.
3. **Its own ADR.** This decision is recorded here rather than in the sync core's ADR, which lands after it; the
   record the design names for this row ("a SPEC; no ADR beyond the sync design's") is read as satisfied by this ADR,
   since the decision to model first is new.
4. **Bounds that fit the budget.** The check runs one model-checker pass per property per configuration. At the
   design's main bound with one crash per device the space measured 2,130,199 distinct states (434 s for all
   properties in one pass), so the main configuration takes no device crash (140,340 states) and one revocable
   device; the device crash's only effect in the model, losing unread answers, is carried by the server crash. Even
   so the entry's check measured 391 s, past the default 300 s, so `tla/OpLog` takes 600 s in `config/formal.json`
   (391 x 1.5, rounded up to the minute).
5. **The ledger records them present.** Each named property is a present artifact under every obligation that named
   the entry, and each entry's row is `re-stamp`.

## Consequences

- Each entry is checked by the formal check from the day it lands, and a change to the protocol's design is a change
  to a checked model.
- Until the sync core lands, nothing binds the model or the proofs to code; the ledger keeps O1, O2, O3 and O16 at
  `planned`, and their gap lines say so.
- A later delivery that raises the main bound pays its measured time in `config/formal.json`.

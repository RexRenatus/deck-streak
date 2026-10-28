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
   device; the device crash is checked at a configuration of its own (decision 7). The entry's check measured 391 s
   before CausalHlcAtMake and `MCDeviceCrash.cfg`, and 631 s, 670 s and, on a loaded box, 839 s with them, past the
   earlier 600 s, so `tla/OpLog` takes 1260 s in `config/formal.json` (the worst, 839 x 1.5, rounded up to the
   minute).
5. **The ledger records them present.** Each named property is a present artifact under every obligation that named
   the entry, and each entry's row is `re-stamp`.
6. **The causal claim is read from the state.** CausalHlc's first half compares an operation's clock with its own
   `saw`, which the step that makes it writes from the same reading of the pulled copy, so a wrong reading agrees with
   itself: a hand mutant that reads the least clock as the seen one passed every configuration. CausalHlcAtMake, an
   action property, states the claim from the state instead: the step that makes an operation gives it a clock past
   every clock in its device's pulled copy at that step. It is checked at `MCOpLog.cfg`, `MCOpLogThree.cfg` and
   `MCDeviceCrash.cfg`, and its witness switches on that least-clock reading. CausalHlc stays unchanged, its half
   that a device's own clock only rises included.
7. **The device crash has a configuration of its own.** `MCDeviceCrash.cfg` takes the main configuration's constants
   with one crash per device (767,800 distinct states) and checks NoLostReview, ExactlyOnce and CausalHlcAtMake
   there, so the crash step is checked and a crash that drops the device's queue is caught. The other properties stay
   at the configurations without a device crash, by decision 4's budget.

## Consequences

- Each entry is checked by the formal check from the day it lands, and a change to the protocol's design is a change
  to a checked model.
- Until the sync core lands, nothing binds the model or the proofs to code; the ledger keeps O1, O2, O3 and O16 at
  `planned`, and their gap lines say so.
- A later delivery that raises the main bound pays its measured time in `config/formal.json`.

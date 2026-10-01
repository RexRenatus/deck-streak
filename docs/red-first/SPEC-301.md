# Red-first record: SPEC-301

Recorded 2026-10-01. The seven tests of A1 to A7 were committed alone (bd95b60), before ADR-301
and before the three amendment notes, so each failed by assertion. A1 to A4 read ADR-301 through
a loader that raises an `AssertionError` naming the missing file. A5 to A7 found each amended
document's earlier text kept, then found no dated note naming ADR-301. The commit that added
ADR-301 and appended the three notes (760ebfa) turned them green.

```red-first
A1: red at bd95b60: AssertionError: 0 files match docs/decisions/ADR-301-*.md, not one
A1: green at 760ebfa
A2: red at bd95b60: AssertionError: 0 files match docs/decisions/ADR-301-*.md, not one
A2: green at 760ebfa
A3: red at bd95b60: AssertionError: 0 files match docs/decisions/ADR-301-*.md, not one
A3: green at 760ebfa
A4: red at bd95b60: AssertionError: 0 files match docs/decisions/ADR-301-*.md, not one
A4: green at 760ebfa
A5: red at bd95b60: AssertionError: Lists differ: ['0 dated notes name ADR-301, not one'] != []
A5: green at 760ebfa
A6: red at bd95b60: AssertionError: Lists differ: ['0 dated notes name ADR-301, not one'] != []
A6: green at 760ebfa
A7: red at bd95b60: AssertionError: Lists differ: ['0 dated notes name ADR-301, not one'] != []
A7: green at 760ebfa
```

The red run reads `Ran 7 tests` and `FAILED (failures=7)`. The green run reads `Ran 7 tests ... OK`
and prints `examined 5 named documents and duties`, `examined 7 parts of the decision`,
`examined 35 terms the parts state`, `examined 8 never-list entries` and
`examined 6 option bullets`. Each note's reader prints `examined 3 terms the note names` for the
charter, `examined 6 terms the note names` for ADR-089 and `examined 4 terms the note names` for
ADR-037.

Each test also runs its reader over a planted document it must refuse: a rule reworded, a part
with a term removed, a never-list entry with a blank protection and one #514 does not state, and
each amended document with its earlier text removed. A reader that sees nothing cannot pass.

The delivery changes no `crates/**/*.rs` path, so it is out of the domain of the Rust red-first
replay. Its criteria are the documents' own, decided by the test module above, which reads
documents only. The ruling the owner signs (R8) is decided by the signature, not by a test.

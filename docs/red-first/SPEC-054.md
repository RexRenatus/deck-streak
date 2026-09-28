# Red-first record: SPEC-054

The SPEC was committed alone (a117bcf), then the tests (ff25fe2), with one stub: the rails guard's
cargo call moved into a helper, `bounded()`, that still passed no timeout, as the base did. Every
other criterion ran against the base's own code. Each was run at ff25fe2 for its own reason, and
again at the implementation (3a03c5d). R7 and A13 came later in the delivery (SPEC §7): A13 was
committed red against the unchanged `scripts/check.sh` (c8ffe05), and green with the stage's fix
(c81eac7). Every sha was re-run from a `git archive` export, with the maintainer's private list
dropped, so each verdict rests on the public shapes alone; at c81eac7 all thirteen criteria pass.

```red-first
A1: red at ff25fe2: AssertionError: 3 != 1 : examined 0 file(s) against public shapes only; 0 finding(s) (the file given as its subject was walked as a directory, and examined nothing)
A1: green at 3a03c5d
A2: red at ff25fe2: AssertionError: 3 != 2 : examined 0 file(s) against public shapes only; 0 finding(s) (the missing subject was not refused, only counted as nothing)
A2: green at 3a03c5d
A3: red at ff25fe2: AssertionError: 0 != 3 : examined 1 file(s) against public shapes only; 0 finding(s) (the empty subject passed beside the tree)
A3: green at 3a03c5d
A4: red at ff25fe2: AssertionError: 1 != 0 : public-scrub: <the subject>/note.md:1: email (each of the eight unit names was refused as an address)
A4: green at 3a03c5d
A5: not red: the base refuses every address, these included; A5 pins the admission's edges (a unit word as another label, a longer last label, a last label that ends in a unit word, a unit type in capitals, a unit word inside a label), and the hand-proved mutants below show that it fails each looser admission
A6: red at ff25fe2: AssertionError: Lists differ: [] != ['scrub.rules'] (the vendoring composed the deny lists itself, with its own load_deny calls)
A6: green at 3a03c5d
A7: red at ff25fe2: AssertionError: 1 != 2 : Traceback (most recent call last): ... persona_core_probe.ContractError: <scratch>/malformed.json: not JSON (the scrub crashed, and its exit read as a finding)
A7: green at 3a03c5d
A8: red at ff25fe2: AssertionError: False is not true : ok       alpha                probe  examined 3: unexpected 0, expected 1, stale 0 (the expectation's closed issue was never read)
A8: green at 3a03c5d
A9: red at ff25fe2: AssertionError: 0 != 2 : box-packs: judging <sha> (HEAD) without the vendored rule code (in each of the four subtests, no gh, not logged in, offline and no state, the run never asked and passed)
A9: green at 3a03c5d
A10: red at ff25fe2: AssertionError: AssertionError not raised (the planted command ran its 10 s to the end)
A10: green at 3a03c5d
A11: red at ff25fe2: AssertionError: None != 900 (the cargo call passed no timeout)
A11: green at 3a03c5d
A12: red at ff25fe2: AssertionError: examined 0 README day-token example(s): the population is empty, so nothing was judged
A12: green at 3a03c5d
A13: red at c8ffe05: AssertionError: 'Ran 2 tests' not found in the python stage's log, which held only the planted guard suite's `Ran 1 test` and `FAILED (failures=1)` (the stage stopped before the oracle suite ran)
A13: green at c81eac7
```

A13's case of a suite that exits 0 having run no test, as Python 3.11's unittest does, was added
with the fix: Python 3.12's unittest exits 5 on an empty run, so without that case a mutant that
drops the stage's own no-test check survived. On the base that case is red too, because the stage
passes both empty suites.

## The vendoring is byte-identical (R3)

At c0dbf2a and at 3a03c5d, `scripts/vendor-packs.py` was run against the vendored phoenix-v2
commit, each into its own scratch copy of `dev` c0dbf2a and with the maintainer's private list.
Both runs exited 0 and printed the same three lines, whose sha256 is `60c0dbb9…2be2455` both times:
the upstream packs left out (18, not vendored), the subscription-proxy pack excluded whole, and
`vendor-packs: examined 186 file(s), changed 0, new 0, excluded 71, from
e54f39c5ad586fc59da33b56bd8e0044af277c25`. The two copies were byte-identical afterwards. The two
refusals, a private list that is not a file and one that is not JSON, each printed the same line
and exited 2 before and after (sha256 `451098db…300cd095` and `27eb3bbb…348df46d`).

## Mutants of the changed code

No mutation runner judges this repository's Python yet (#217, #218), so the changed code's mutants
were proved by hand at c81eac7, on an export. Each mutant replaced an anchor that occurs exactly
once, and one criterion's test selected alone was green on the unmutated file and red on the
mutant. The file was restored byte for byte after each. All 37 were killed:

| requirement | mutant | killed by |
|---|---|---|
| R2 | the last label compared by `endswith`, by prefix, or case-folded; the first label after the at sign compared instead | A5 |
| R2 | the admission removed; `scope` dropped from the types; the admission judged per line instead of per match | A4 |
| R1 | a file no longer its own subject; no refusal of a missing subject; an empty subject not VOID; VOID exiting 0 | A1, A2, A3 |
| R3 | no private-list check; a probe refusal escaping as a traceback; privacy-gdpr's list dropped; the private list dropped; the vendoring letting the scrub's refusal escape | A7, A6 |
| R4 | a closed row issue, or a closed pending issue, not stale; a pending pack or the scan ignoring its stale entries; `gh` asked outside ROOT; the scan's pending never asked | A8 |
| R4 | a missing `gh` passing; not logged in read as another failure; a failed `gh` passing; any answer taken as a state | A9 |
| R5 | no timeout; a failure that names nothing | A10 |
| R5 | the cargo call unbounded | A11 |
| R6 | the example's token off by one day | A12 |
| R7 | a red suite passing; a suite that ran nothing passing; the first red suite stopping the stage; only the first suite run; the suites' output not logged; no verdict line; the stage always passing | A13 |

# ADR-510: equivalent survivors on a census-held file are cured by behaviour-neutral edits, and every kill runs in its file's mapped suite

**For:** SPEC-492.

## Context

The release run left 813 survivors in the scripts class. The scripts judge excuses a survivor only by exactly one binding record.
The mapped suites of `scripts/mutation-verdict.py` and `scripts/mutation_rows.py` run the record census, which refuses a record
whose anchor does not occur once, and a record's anchor holds its mutant's token: the mutant edits the anchor out, the census
fails, and the record is refuted. So no record can excuse a survivor on those two files. Meanwhile
`scripts/tests/test_mutation_workflows.py` already kills 154 of the verdict script's survivors, but the map gives it no file, so
no mutant run executes it.

## Decision

D1. Map `test_mutation_workflows` to `scripts/mutation-verdict.py`, and put the new kills in two modules mapped to their own
files, `test_mutation_verdict_survivor_kills` and `test_mutation_rows_survivor_kills`, with the scrub's new kills in
`test_public_scrub`.

D2. A survivor equivalent on a census-held file is cured by an edit that removes its site and changes no output, exit or file;
one on a file whose suite runs no census is recorded.

D3. A Rust record's `reached_by` is resolved through a resolver that takes the crate and the killer alone, so no placeholder row
carries fields no reader reads.

D4. The two temporary directories drop their name prefix, which no output, exit or kept file reads.

## What it was chosen AGAINST

| alternative | why it lost |
|---|---|
| porting `test_mutation_workflows`' configuration and exclusion assertions into a new module | it copies 154 kills' text, while a mapped module only adds tests to its file's suite |
| records on the census-held files | the census refutes each one, since the mutant edits its own record's anchor out |
| waiving the class | it narrows the gate, and only the owner may |
| keeping the prefixes as an aid to whoever reads a scratch directory | both stay survivors, and nothing reads them |
| keeping the placeholder row and pinning each placeholder field | the pins would assert values no reader uses |

## Consequences

- Each mutant of the verdict script runs two more modules, and each of the rows runner's one more; the price tables keep their
  recorded figures until a recorded run re-derives them.
- `locate_killer` and `resolve_killer` keep their signatures and every caller; two rows hold the crate each forwards.
- Two formal covers move with their spans, re-read with no model or port change.

## What would make this wrong

- An edit of D2 that changes an output: the existing tests and the kill modules would fail on it, and the release run would read
  a new survivor.
- A mapped module whose time pushes a release leg past its bound: the legs line would name the leg VOID.

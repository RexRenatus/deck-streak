---
status: "proposed"
decision-makers: "@RexRenatus (owner), the DeckStreak architect"
---

# ADR-634: the dynamic-imports check takes its binding kinds from the census, reads augmented register sites, decodes strictly and resolves HEAD once

Decides SPEC-612 (R1 to R5).

## Context and Problem Statement

`scripts/dynamic-imports-check.py` (SPEC-406, ADR-420) refuses, before the push, a changed
`scripts/tests` module that loads code while the `DYNAMIC_IMPORTS` register lacks it; the census
in the `hygiene` job stays the judge. #804 measured where the two read differently at `dev`
b065fce: the check reads `exec` as a module's own from 7 binding kinds where the census's
`bindings()` reads 16, so it refuses a module that binds `exec` by `except ... as exec`; it reads
the register through plain assignments only, so a site added by `|=` is unseen; it decodes a blob
inside `subprocess.run`, so a blob that is not UTF-8 ends in a traceback rather than VOID; and it
names `HEAD` in three reads. How should the check close each gap without becoming a second census?

## Decision Drivers

- The check never refuses a module the census admits (ADR-420), so it never blocks a correct push.
- The census stays the judge, read as text only; the check imports and runs nothing it judges.
- A run that cannot read its inputs is VOID, never OK and never a traceback.
- A verdict is over one commit, the one `HEAD` names when the run starts.

## Considered Options (the alternatives it was chosen against)

### D1. The check reads every binding kind the census reads, module-wide

- Chosen: the 16 kinds of `bindings()`, read anywhere in the module, because the module-wide
  reading can only admit more than the census, whose scoping in `hygiene` stays the judge (ADR-420
  D2).
- A scope-aware reading, a copy of the census's scoping in the check: rejected because two copies
  of one scoping rule drift, and a module-wide reading never refuses what the census admits.
- Importing the census to call its `bindings()`: rejected because ADR-420 reads the census as text
  only; instead A1 holds the two kind sets equal from the census's syntax tree.

### D2. A module-level `|=` of `allowed` groups is read, and every other write is VOID

- Chosen: read each `DYNAMIC_IMPORTS |= {**allowed(...)}` after the plain assignment by SPEC-406
  R3's rules, and stop on any other write, because that is the one growth shape the census admits
  that the check can read without running the register.
- VOID on every augmented assignment: rejected because the census admits that shape, so VOID
  would stop a correct push.
- Evaluating other write shapes: rejected because the check never runs the register.

### D3. Strict UTF-8, and VOID on a blob that is not

- Chosen: decode every git output in the check, strictly, and stop with a VOID that names the
  path, because the census reads every module as UTF-8 and fails there on the same file.
- Decoding with replacement: rejected because it would judge text that is not the file.
- Honouring an encoding declaration: rejected because the census reads every module as UTF-8, so
  such a module fails in `hygiene` whatever the check reads.

### D4. `HEAD` resolved once, to a commit

- Chosen: `git rev-parse --verify HEAD^{commit}` first, and that commit named in every later read,
  because a ref another process moves can no longer split one verdict over two commits.
- A `--head` option: rejected because it is a surface the issue does not ask for.
- Reading the work tree: rejected because ADR-420 D1 judges what the commit holds.

### D5. FORMAL is not applicable, by surface

- Chosen: no model, as ADR-420 D4 found: the check is one process that reads immutable objects of
  one commit and prints a verdict, and resolving `HEAD` once removes its one dependence on a ref
  another process may move.
- A TLA+ model of the check beside a process that moves `HEAD`: rejected because after D4 no read
  names the ref, so every interleaving gives the verdict of the first commit, which A6 shows.

## Decision Outcome

Chosen options: D1 to D5 as above. The check reads `exec` as the census does, reads augmented
register sites and stops on writes it cannot read, stops on text that is not UTF-8, and judges one
commit; its printed lines and SPEC-406's eight tests are unchanged.

### Consequences

- Good: a module that binds `exec` by a capture, a type parameter, a `global` or a `nonlocal` is no
  longer refused by the check while the census admits it.
- Good: a register grown by `|=` is read, and a write the check cannot read stops the run rather
  than passing unseen.
- Good: a file that is not UTF-8 reads VOID with its path, never a traceback.
- Bad: a register write through another name bound to the register is not read; the census still
  counts its sites in `hygiene`.
- Neutral: the census, its register and every row anchored there are unchanged.

### Confirmation

SPEC-612's A1 to A6 in `scripts/tests/test_dynamic_imports_check.py`, the `hygiene` check-run, and
the `mutation-python` legs that judge the check's mutants.

## What would make this wrong

- The census gains a binding kind: A1 reds in `hygiene` on that delivery, since it reads the
  census's kinds from its syntax tree.
- The register grows through another name bound to it: the check does not read that write, and the
  census counts the sites it adds.

## More Information

- SPEC-612 (this ADR's SPEC); SPEC-406 and ADR-420 (the check); #804.

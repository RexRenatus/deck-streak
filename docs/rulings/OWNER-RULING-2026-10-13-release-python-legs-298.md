By signing the commit that adds this file, the owner admits one thing: the dev-to-main release pull request's mutation run, and only that run, may plan up to 298 Python legs, above `PYTHON_SHARE_CAP` 128 (`scripts/mutation-verdict.py:127`). Every mutant runs, every shard is counted, there is no skipped-mutant record, and `RUN_JOBS` stays 256 per matrix (`scripts/tests/test_dispatch_shards.py:218`). 298 is a ceiling and fails closed: a later replay that reads more than 298 needs a new owner signature. Pull requests into `dev` keep the share of 32 and the cap of 128.

# OWNER RULING 2026-10-13: the release pull request may plan up to 298 Python legs

Cited by the later delivery that applies this admission, which carries the code change and cites this file; this delivery changes no code. Nothing that depends on this admission lands before the signed commit of this file is on `dev`.

## What was held

The release pull request into `main`, #822, is held at its mutation plan: its Python listing needs more legs than the share, and no signed share may pass the cap. At `dev` c726af67 the places that hold it are on these lines:

| place | what it holds |
|---|---|
| `scripts/mutation-verdict.py:125` | `PYTHON_MAX_SHARDS = 32`, the share: the most Python legs a pull request's run deals |
| `scripts/mutation-verdict.py:127` | `PYTHON_SHARE_CAP = 128`, the ceiling no signed share may pass |
| `scripts/mutation-verdict.py:130` | `PYTHON_LEG_BOUND_SECONDS = 1800`, the most a Python leg's projection may reach |
| `scripts/mutation-verdict.py:1014` | the refusal of a share outside 1 to `PYTHON_SHARE_CAP`, before any deal |
| `scripts/mutation-verdict.py:1113` | the refusal of a listing that needs more legs than the share, whole and by name |
| `scripts/tests/test_dispatch_shards.py:218` | `RUN_JOBS = 256`, GitHub's matrix limit |
| `RELEASING.md:67`, `:68`, `:72` | the release runbook's statement of the share, its refusal and the cap |

## What was measured

Scoped per-file prices from one dispatched price run over all 16 Python files, replayed offline over the release's union tree by the plan's own dealing, read the release's 6381 Python mutants as needing 298 priced legs and 0 unpriced, the slowest projected at 1793 s of its 1800 s bound (prices from run 38432909235).

Dealt into matrices of at most 256 legs, the 298 legs need 2. Against the share of 32 the plan reads, verbatim:

```
mutation: shards: REFUSED: 6381 python mutant(s) need 298 leg(s) priced at most 1800 s each and 0 unpriced leg(s) of at most 20, more than the pull request run's 32 python legs: split the change, since a run is never capped
```

The projection made before the full price run read 268 legs; the full run's prices read 298, 30 more. The ceiling is the measured figure, never the projection.

## What it closes

At the share of 32 the release's plan is refused whole, so no leg starts and the release cannot be judged. A share of 128, the most any signed share may be, still holds fewer legs than the release needs, and a share above it is refused before any deal (`scripts/mutation-verdict.py:1014`).

At up to 298 the release's run plans every leg the replay reads: every mutant runs once, in a leg the verdict counts. A later replay of the release that reads more than 298 is not admitted by this ruling, and needs a new owner signature.

## What replaces it

- For the dev-to-main release pull request's mutation run only, the plan may deal up to 298 Python legs, above `PYTHON_SHARE_CAP` 128. Every mutant runs, every shard is counted, and there is no skipped-mutant record.
- `RUN_JOBS` stays 256 per matrix (`scripts/tests/test_dispatch_shards.py:218`).
- `PYTHON_LEG_BOUND_SECONDS` stays 1800 (`scripts/mutation-verdict.py:130`); the 298 is read at that bound.
- Pull requests into `dev` keep the share of 32 (`scripts/mutation-verdict.py:125`) and the cap of 128 (`scripts/mutation-verdict.py:127`).
- 298 is a ceiling and fails closed: a later replay above it needs a new owner signature.
- The code change that applies this admission rides a later delivery that cites this signed ruling; this delivery changes no code.

## Why it is admitted

The admission is a weakening by the cap's letter, because one run deals more legs than `PYTHON_SHARE_CAP` lets any share deal (SPEC-420, ADR-434). In effect it examines more, never less: every mutant of the release still runs and is counted, each leg keeps its bound, and what grows is runner time. It is held to one run and to a measured ceiling. The rejected alternatives:

- **Splitting the release.** Each part judges only its own merge diff, so a later part's change that deletes the test killing an earlier part's mutant is never judged with that mutant, and a part's head is not `dev`'s tip (ADR-373).
- **A one-time admission of the release past its mutation gate.** It lands a tree whose mutants were never examined (ADR-373).
- **A global raise of the cap.** It lets a share signed for any pull request into `dev` deal past 128 when only the release's run needs it, and the cap is what refuses a share signed in error, whoever signs it (SPEC-420 R2).

## Signature

The owner signs the commit that adds this file. The signature is this ruling's authority; a copy of this file in any unsigned commit carries none.

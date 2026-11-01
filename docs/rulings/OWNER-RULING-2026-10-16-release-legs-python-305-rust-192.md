By signing the commit that adds this file, the owner admits two things for the dev-to-main release pull request's mutation run, and only that run: it may plan up to 305 Python legs, above `PYTHON_SHARE_CAP` 128 (`scripts/mutation-verdict.py:129`), and up to 192 Rust legs, above `LEG_CEILING["ci"]` 190 (`scripts/mutation-verdict.py:961`).

# OWNER RULING 2026-10-16: the release pull request may plan up to 305 Python legs and 192 Rust legs

Cited by the later delivery that applies these admissions (#849), which carries the code change and cites this file; this delivery changes no code. Nothing that depends on this ruling lands before the signed commit of this file is on `dev`.

It supersedes the Python figure of `docs/rulings/OWNER-RULING-2026-10-15-release-python-legs-303.md`, which was measured before the change that holds the release's Rust legs (#849) joined the release's union. That ruling's other terms stand. Its own text says a later replay above 303 needs a new owner signature; this is that signature.

## What was held

The release pull request into `main`, #822, is held at its mutation plan: its Python listing needs more legs than the signed release ceiling, and its Rust listing needs more legs than a `ci` run holds. At `dev` a3a052b2 the places that hold it are on these lines:

| place | what it holds |
|---|---|
| `scripts/mutation-verdict.py:129` | `PYTHON_SHARE_CAP = 128`, the ceiling no signed share may pass |
| `scripts/mutation-verdict.py:131` | `PYTHON_RELEASE_CEILING = 303`, signed in the 2026-10-15 ruling |
| `scripts/mutation-verdict.py:134` | `PYTHON_LEG_BOUND_SECONDS = 1800`, the most a Python leg's projection may reach |
| `scripts/mutation-verdict.py:136` | `PYTHON_MATRIX_LEGS = 256`, the most Python legs one matrix deals |
| `scripts/mutation-verdict.py:1231` | the refusal of a release plan above `PYTHON_RELEASE_CEILING`, which asks for a new measurement and a new owner-signed ruling |
| `scripts/mutation-verdict.py:958` | `SHARD_BOUND_SECONDS = 10800`, the most a Rust leg's projection may reach |
| `scripts/mutation-verdict.py:961` | `LEG_CEILING = {"ci": 190, ...}`, the most Rust legs a `ci` run holds |

The applying change (#849) adds `RUST_RELEASE_CEILING`, which holds the release run's Rust legs in place of `LEG_CEILING["ci"]` only when it is above 190, and `RUST_MATRIX_LEGS = 256`. It lands with the ceiling at 190, so it admits no Rust leg by itself.

## What was measured

Both figures are measured on the release's final union: `dev` at a3a052b2 together with #849 at a8a72b24, which is 4 commits ahead of `dev` and 0 behind.

- **Python: 305.** Scoped per-file prices from the dispatched price run 38432909235, replayed offline over the union tree by the plan's own dealing, read the release's 6463 Python mutants as needing 305 priced legs and 0 unpriced. The slowest leg projects 1793 s of the 1800 s bound. Dealt into matrices of at most 256 legs, the 305 legs need 2: 256 and 49.
- **The replay is held to its control.** The same replay over `dev` alone reads 6442 mutants and 303 legs, the figure the 2026-10-15 ruling signed. All 21 added mutants and both added legs are in `scripts/mutation-verdict.py`, the file #849 edits.
- **Rust: 192.** The plan's own Rust pricing over the union's 9385 Rust mutants reads 192 legs as the fewest whose slowest projection fits the 10800 s bound. At 192 the slowest leg projects 10461 s, the median 10218 s and the fastest 9849 s. At 191 the slowest projects 11659 s, over the bound by 859 s. The 192 legs fit one Rust matrix of at most 256.

**One price is known to read low.** The Python price row for `scripts/mutation-verdict.py` comes from run 38432909235, before #849 added tests that exercise that file. On #849's own run (38842293718), that file's three legs ran their control at 1.12 to 1.55 times the row's price and their mutants at 1.12 times its mean. Two legs recorded 1834 s and 2077 s against the 1800 s projection bound. That bound is half the Python job's `timeout-minutes` of 60, so both legs finished, and every mutant was read. A price refresh that raises the row may move the replay past 305. The ceiling then fails closed, and the new figure needs a new owner signature.

## What it closes

At 303 the union's Python plan is refused, because it needs 305 legs. At `LEG_CEILING["ci"]` 190 its Rust plan is refused, because it needs 192. Either refusal stops every leg, so the release cannot be judged.

At up to 305 and 192 the release's run plans every leg the replay reads: every mutant runs once, in a leg the verdict counts. A later replay of the release that reads more than either figure is not admitted by this ruling, and needs a new owner signature.

## What replaces it

- For the dev-to-main release pull request's mutation run only:
  - the plan may deal up to 305 Python legs, above `PYTHON_SHARE_CAP` 128;
  - the plan may deal up to 192 Rust legs, above `LEG_CEILING["ci"]` 190.
- Every mutant runs, every shard is counted, and there is no skipped-mutant record.
- 305 replaces 303 as the release's Python ceiling.
- `PYTHON_LEG_BOUND_SECONDS` stays 1800 and `SHARD_BOUND_SECONDS` stays 10800. Both figures are read at those bounds.
- Each matrix stays at most 256 legs (`PYTHON_MATRIX_LEGS`, and `RUST_MATRIX_LEGS` in #849).
- Pull requests into `dev` keep:
  - the share of 32 (`scripts/mutation-verdict.py:127`);
  - the cap of 128 (`scripts/mutation-verdict.py:129`);
  - `LEG_CEILING["ci"]` 190 (`scripts/mutation-verdict.py:961`).
- Both figures are ceilings and fail closed: a later replay above either needs a new owner signature.
- The code change that applies this ruling rides #849:
  - it sets `PYTHON_RELEASE_CEILING` to 305 and `RUST_RELEASE_CEILING` to 192;
  - it cites this signed ruling;
  - it lands only after the signed commit of this file is on `dev`.

## Why it is admitted

The admission is a weakening by the letter of the cap and of `LEG_CEILING["ci"]`, because one run deals more legs than either lets a pull request's run deal. In effect it examines more, never less: every mutant of the release still runs and is counted. It was chosen against these alternatives:

- **Splitting the release.** Each part judges only its own merge diff. A later part's change that deletes the test killing an earlier part's mutant is then never judged with that mutant, and a part's head is not `dev`'s tip (ADR-373).
- **A one-time admission of the release past its mutation gate.** It lands a tree whose mutants were never examined (ADR-373).
- **A global raise of the cap or of `LEG_CEILING["ci"]`.** It lets every pull request into `dev` deal past them when only the release's run needs it. The cap is what refuses a share signed in error, whoever signs it (SPEC-420 R2).
- **A ceiling with headroom above the measured figures.** It admits legs no measurement read.
- **Refreshing the low price row before signing.** It costs another price run and another measure of the union. It moves the Python figure without changing what is admitted: a ceiling read on the final union that fails closed above it.

## Signature

The owner signs the commit that adds this file. The signature is this ruling's authority; a copy of this file in any unsigned commit carries none.

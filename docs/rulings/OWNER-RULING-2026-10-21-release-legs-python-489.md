By signing the commit that adds this file, the owner admits one figure for the dev-to-main release pull request's mutation run, and only that run: it may plan up to 489 Python legs (`PYTHON_RELEASE_CEILING`, `scripts/mutation-verdict.py:132`) in place of the 305 that `docs/rulings/OWNER-RULING-2026-10-16-release-legs-python-305-rust-192.md` signed, the fewest that deal the release's 7741 Python mutants within the kept 1800 s leg bound. `PYTHON_SHARE_CAP` 128, `PYTHON_LEG_BOUND_SECONDS` 1800, `PYTHON_MATRIX_LEGS` 256, `PYTHON_MATRICES` 2, `RUST_RELEASE_CEILING` 192, `LEG_CEILING` and every job's `timeout-minutes` are unchanged.

# OWNER RULING: the release pull request may plan up to 489 Python legs

The owner chose this figure on 2026-10-21, answering the measurement of the release pull request's refused plan with "(A) Signed raise to 489". This file is that figure, for the owner to sign.

It amends three places in two signed rulings and leaves both signed files as signed:

- In `docs/rulings/OWNER-RULING-2026-10-16-release-legs-python-305-rust-192.md`, under "What replaces it", two places. One is the sub-bullet "the plan may deal up to 305 Python legs" (line 44), under the bullet "For the dev-to-main release pull request's mutation run only" (line 43). The other is the bullet "305 replaces 303 as the release's Python ceiling" (line 47). Each reads 489 in place of 305. That ruling's own line 54 asks for this file: "Both figures are ceilings and fail closed: a later replay above either needs a new owner signature." Everything else that ruling says stands, its Rust ceiling of 192 included.
- In `docs/rulings/OWNER-RULING-2026-10-21-release-carry.md`, under "What stays", the bullet "Every bound, price, timeout, ceiling and per-mutant budget stays as `dev` holds it" (line 12). It reads with 489 as the release's Python ceiling. Every other term of that ruling stands.

The code change that applies the figure, and its changelog fragment, are in the same signed commit as this file.

## What was held

At `dev` 2b826491, the head of the release pull request's run 39582059592, the figures are on these lines:

| place | what it holds |
|---|---|
| `scripts/mutation-verdict.py:130` | `PYTHON_SHARE_CAP = 128`, the ceiling no signed share may pass |
| `scripts/mutation-verdict.py:132` | `PYTHON_RELEASE_CEILING = 305`, signed in the 2026-10-16 ruling |
| `scripts/mutation-verdict.py:135` | `PYTHON_LEG_BOUND_SECONDS = 1800`, the most a Python leg's projection may reach |
| `scripts/mutation-verdict.py:137` | `PYTHON_MATRIX_LEGS = 256`, the most Python legs one matrix deals |
| `scripts/mutation-verdict.py:139` | `PYTHON_MATRICES = 2`, so a release ceiling outside 128..512 is refused before any deal (`python_ceiling_refused`, `scripts/mutation-verdict.py:1096`) |
| `scripts/mutation-verdict.py:967` | `RUST_RELEASE_CEILING = 192`, the most Rust legs the release run deals |

The run's `mutation-plan` job, 119161786304, refused the plan before any leg was dealt: `mutation: shards: REFUSED: 7741 python mutant(s) need 489 leg(s) priced at most 1800 s each and 0 unpriced leg(s) of at most 20, more than the release run's 305 python legs (PYTHON_RELEASE_CEILING): a larger figure needs a new measurement and a new owner-signed ruling, since a run is never capped`.

## What was measured

Every figure is read at `dev` 2b826491 with the tree's own functions, over the `mutation-plan` artifact 12338045445 of run 39582059592: its `python-rerun.json` holds 7741 mutants, equal to its `python-listed.json`.

- **489 is the fewest.** `python_by_cost` and `shards()` at 489 deal 489 priced legs and 0 unpriced. Their priced seconds sum to 831802, and the slowest leg is priced at 1799 s of its 1800 s bound. At 488 they refuse. 489 lies inside `python_ceiling_refused`'s window of 128..512.
- **The measurement is held to its control.** The same `python_by_cost` at 305 gives a refusal byte-equal to job 119161786304's line.
- **No carry lowers it.** No recorded release run before 39582059592 is a base. The newest, 39437750892, was cancelled, and `select_base` refuses a cancelled run. The measurement also tried that run as a base, without changing the tree. The carry's own `carry()` then carries 0 of the 7741 mutants: that run's tree predates `scripts/mutation-inputs.json`, so the carry refuses it whole before it judges any file. Past that refusal, each of the 15 read sets the measurement computed still differs. Run 39582059592 itself ran no Python leg, so it carries none either. So the figure is 489 with or without an amendment that would carry a cancelled run's completed legs.
- **It fits GitHub's matrix and job limits as written.** The plan splits the 489 legs into 256 in `mutation-python` (legs 0..255) and 233 in `mutation-python-release` (legs 256..488). Each matrix is within the 256 jobs one matrix may generate. Rust deals 192 legs, at its ceiling of 192, with no margin. Each Python job keeps `timeout-minutes: 60`, under the 6-hour limit of a hosted job. Two things were not measured: whether the verdict job downloads and judges all 681 legs within its `timeout-minutes` of 10, and whether those downloads stay within the job token's 1,000 requests an hour.
- **Its cost.** The priced figure is 231.06 Python runner-hours. At the 1933.4 s mean of the cancelled run's 112 completed Python legs it is 262.62 h. At the 40 concurrent jobs the cancelled run reached, the Python and Rust legs alone take at least about 19.4 h of wall clock at their priced projections. At 20 concurrent jobs they take at least about 38.7 h. A push to `dev` while the run is in flight cancels it, so `dev` takes no push until the run ends.

## What it closes

At 305 the release's plan refuses its Python legs whole, every leg stops, and the release cannot be judged. At 489 the plan deals every Python mutant once, in a leg the verdict counts, within the kept 1800 s bound. A later plan that needs a 490th leg is still refused whole, by name, and never cut.

## What replaces it

- For the dev-to-main release pull request's mutation run only, the plan may deal up to 489 Python legs, above `PYTHON_SHARE_CAP` 128. 489 replaces 305 as the release's Python ceiling.
- Every listed mutant is judged once, as `docs/rulings/OWNER-RULING-2026-10-21-release-carry.md` reads that term. Every shard is counted, and there is no skipped-mutant record.
- `PYTHON_LEG_BOUND_SECONDS` stays 1800, `PYTHON_MATRIX_LEGS` 256 and `PYTHON_MATRICES` 2. The figure is read at those bounds.
- `RUST_RELEASE_CEILING` stays 192, and `LEG_CEILING` stays `{"ci": 189, "battery": 234}`.
- Pull requests into `dev` keep the share of 32 (`scripts/mutation-verdict.py:128`) and the cap of 128 (`scripts/mutation-verdict.py:130`).
- The figure is a ceiling and fails closed: a later plan above it needs a new measurement and a new owner signature.
- The code change that applies this ruling is in the same signed commit as this file. It sets `PYTHON_RELEASE_CEILING` to 489, and its `#:` line cites this file in place of the 2026-10-16 ruling (SPEC-422 R4), as the tree's own ceiling check requires. It changes no other line. The commit also adds one changelog fragment, `changelog.d/ci-release-ceiling-489.md`.

## Why it is admitted

The raise is a weakening by the letter of the 2026-10-16 ruling's ceiling, because the release run deals more Python legs than that ruling admitted. In effect it examines more, never less: every Python mutant of the release runs once, in a leg priced within the kept bound, and a plan past 489 legs is still refused whole by name. It was chosen against these alternatives:

- **Keeping 305.** The release's plan refuses, so the release cannot be judged at all (run 39582059592).
- **Carrying the cancelled run's completed legs.** The carry needs a base whose read sets are byte-identical. The cancelled run's tree has no read-set map, so it carries 0 of the 7741 and the figure stays 489.
- **Raising the leg bound, so that each leg holds more mutants and fewer legs are needed.** It narrows the margin each leg keeps under its job's `timeout-minutes`. The cancelled run's completed legs already took 1933.4 s on average against the 1800 s price bound, and up to 3123 s of the 3600 s timeout.
- **Fewer Python mutants for the release, or a carry with looser read sets.** It narrows what the release's mutation gate proves.
- **A ceiling with headroom, up to the window's 512.** It admits legs no measurement read.

## Signature

The owner signs the commit that adds this file. The signature is this ruling's authority; a copy of this file in any unsigned commit carries none.

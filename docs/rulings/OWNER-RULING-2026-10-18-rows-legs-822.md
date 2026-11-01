By signing the commit that adds this file, the owner admits two figures for the mutation plan, and only these, both read from the release pull request's own plan when it refused its rows: `ROWS_MAX_LEGS` becomes 18 in place of 17 (`scripts/mutation_rows.py:116`), the fewest most legs that deal that plan's 2688 rows within the rows legs' kept bound, and `LEG_CEILING["ci"]` becomes 189 in place of 190 (`scripts/mutation-verdict.py:963`), because the pull request run's ceiling for Rust legs counts the rows job at `ROWS_MAX_LEGS` (SPEC-417 R11). `ROW_SECONDS` 17, `ROWS_BOUND_SECONDS` 2700, `LEG_CEILING["battery"]` 234, `RUST_RELEASE_CEILING` 192, `RUST_MATRIX_LEGS` 256 and every job's `timeout-minutes` are unchanged.

# OWNER RULING: the rows are dealt into at most 18 legs, and the pull request run's Rust ceiling is 189

The owner's answer decides this case in advance, in the owner's own words: "no change now, the most legs read and signed on the next release final union when its plan refuses". The release pull request's plan has refused, and this file is that reading, for the owner to sign.

Cited by the amendment sections that the build carrying these figures appends to SPEC-417 and ADR-431. It amends one place of a signed ruling, `docs/rulings/OWNER-RULING-2026-10-16-release-legs-python-305-rust-192.md`, the one that admits the release pull request's 305 Python legs and 192 Rust legs: its line 53, under "What replaces it", where pull requests into `dev` keep `LEG_CEILING["ci"]` 190. They keep 189. Everything else that ruling says stands, its two release ceilings included. Nothing that depends on 18 or 189 lands before the signed commit of this file is on `dev`.

## What was held

At `dev` 1461fa01, the head of the release pull request's run 39151539183, the figures are on these lines:

| place | what it holds |
|---|---|
| `scripts/mutation_rows.py:110` | `ROW_SECONDS = 17`, a row's price, the measured mean rounded up (ADR-431 D2) |
| `scripts/mutation_rows.py:113` | `ROWS_BOUND_SECONDS = 2700`, half of the rows job's `timeout-minutes` of 90 (SPEC-417 R10) |
| `scripts/mutation_rows.py:116` | `ROWS_MAX_LEGS = 17`, the most legs the rows are dealt into (SPEC-417 R4) |
| `scripts/mutation-verdict.py:963` | `LEG_CEILING = {"ci": 190, "battery": 234}`: 256 jobs less 66 other `ci.yml` jobs, the rows job counted at 17 legs (SPEC-417 R11) |
| `scripts/mutation-verdict.py:965` | `RUST_RELEASE_CEILING = 192`, the most Rust legs the release run deals (SPEC-423) |

At 17, 17 legs of at most 158 rows hold 2686 rows.

## What was measured

| run | job | what it read |
|---|---|---|
| 39151539183, attempt 1, `dev` 1461fa01 | `mutation-plan`, 117746024485 | its `legs` step: `legs: REFUSED: 2688 row(s) need more than 17 legs of at most 158 row(s), each projected within 2700 s at 17 s a row`, exit 2 |
| 39151539183, attempt 1 | `mutation-verdict`, 117746155367 | `mutation_rows.py merge` exits 2 on an empty `--legs` (`invalid int value: ''`), so the rows are never merged and no class is judged |
| 39141497433, `dev` a2f47ae4 | `mutation-plan`, 117717787744 | the same refusal |

- The `mutation-plan` artifact of run 39151539183 holds a `plan.json` marked `"release": true` whose rows are 2688 ids, all 2688 distinct.
- The same plan job's shards step passed: 192 Rust legs of the release ceiling 192 and 304 Python legs of 305 (run 39151539183).
- The plan's own fewest-legs function, `leg_count` (`scripts/mutation_rows.py:1253`), taken from that `dev`'s text with its three constants and run over 2688 rows, returns no count at `ROWS_MAX_LEGS` 17 and 18 legs at 18 (run 39151539183's population).
- At 18 legs the largest leg holds 150 rows, projected at 2550 s of the 2700 s half at the 17 s price of job 114529704482.
- By the same function, 17 legs hold 2686 rows and 2687 are refused; 18 legs hold 2844 rows and 2845 are refused.

## What it closes

At 17 the release's plan refuses its rows whole, its verdict merges no leg, and the release cannot be judged at all: every mutant leg's report waits on a verdict that never judges it (run 39151539183). At 18 every selected row is dealt into a leg within the kept bound, proved once and judged by the unchanged verdict, and a population past 2844 rows is still refused whole by name, never cut.

## What replaces it

- `ROWS_MAX_LEGS = 18`. 18 legs of at most 158 rows hold 2844 rows, 156 more than the 2688 read; the release's rows run as 18 legs, a matrix of their own, within the 256 legs one matrix generates.
- `LEG_CEILING["ci"]` is 189: the run's 256 jobs less 67 other `ci.yml` jobs, the rows job counted at 18 legs, by SPEC-417 R11's unchanged rule. A pull request whose Rust mutants need more than 189 legs is refused whole by name.
- `LEG_CEILING["battery"]` stays 234: `mutation-weekly.yml` deals no rows legs, so its count does not move.
- `RUST_RELEASE_CEILING` stays 192. It is held between `LEG_CEILING["ci"]` and `RUST_MATRIX_LEGS` (`scripts/mutation-verdict.py:1117`), and 192 is above 189, so the release run's Rust legs are held to 192 as before (`scripts/mutation-verdict.py:1131`).
- The pull request run's joint line of fixed shares follows the ceiling, 32 Python and 189 Rust legs; the release run's line keeps 32 and 192 (run 39151539183).
- `ROW_SECONDS` stays 17 and `ROWS_BOUND_SECONDS` 2700, and the rows job keeps `timeout-minutes: 90`, now each of 18 legs'.

## Why it is admitted

The raise of the most legs examines more, never less: each selected row is still proved once, in a leg whose projection stays within half of its kept bound, and a population past the most legs is still refused whole by name. The fall of the pull request run's ceiling is a tightening: a pull request that needs 190 Rust legs, dealt before, is refused whole by name and split, so no mutant runs unexamined. It was chosen against these alternatives:

- **Keeping 17.** The release's plan refuses its 2688 rows, so the release cannot be judged at all (run 39151539183).
- **Re-measuring `ROW_SECONDS` so that 2688 rows fit 17 legs.** It narrows the margin each leg keeps under its bound, and the owner named the most legs as the figure to sign.
- **19 legs or more.** It is headroom beyond the population read: 18 is the fewest that deal it, and ADR-431 D2 derives the most legs from a counted population.
- **Keeping `LEG_CEILING["ci"]` at 190 by counting the rows legs outside the pull request run's jobs.** It changes SPEC-417 R11's rule, and the ceiling would no longer hold every job a pull request's run can generate.
- **Raising the rows job's `timeout-minutes`.** A job's bound is the maintainer's own decision (ADR-431 D1), and the kept bound proves every row at 18 legs.

## Signature

The owner signs the commit that adds this file. The signature is this ruling's authority; a copy of this file in any unsigned commit carries none.

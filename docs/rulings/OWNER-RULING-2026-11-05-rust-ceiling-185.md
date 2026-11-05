By signing the commit that adds this file, the owner admits one figure for the mutation plan, and only this one: `LEG_CEILING["ci"]` becomes 185 in place of 189 (`scripts/mutation-verdict.py:969`), and with it the Rust share of the pull request run's joint line of fixed shares, which follows that ceiling (`scripts/mutation-verdict.py:1447`), because the pull request run's ceiling for Rust legs is the run's 256 jobs less every other job a `ci.yml` run generates (SPEC-417 R11), and the CodSpeed bench lanes add four jobs to `ci.yml` (SPEC-618). `LEG_CEILING["battery"]` 234, `RUST_RELEASE_CEILING` 192, `RUST_MATRIX_LEGS` 256, `ROWS_MAX_LEGS` 18, `PYTHON_MAX_SHARDS` 32 and every job's `timeout-minutes` are unchanged.

# OWNER RULING: the pull request run's Rust ceiling is 185

The owner chose this figure in answer to the bench lanes' census red, in the owner's own words: "Amend the ceiling to 185". This file is that figure, for the owner to sign.

It amends two places of a signed ruling, `docs/rulings/OWNER-RULING-2026-10-18-rows-legs-822.md`, both bullets under "What replaces it": its line 44, the bullet that begins "`LEG_CEILING["ci"]` is 189", and its line 47, the bullet that begins "The pull request run's joint line of fixed shares follows the ceiling", whose 189 Rust legs follow the ceiling to 185. Pull requests into `dev` keep 185 in place of 189, by line 44's own rule, and their joint line reads 32 Python and 185 Rust legs, by line 47's. Everything else that ruling says stands, its 18 rows legs included, and so does every ruling it amends. Nothing that depends on 185 lands before the signed commit of this file is on `dev`.

## What was held

At the bench lanes' pull request head ab8ee5ee, whose `scripts/mutation-verdict.py` equals `dev`'s, the figures are on these lines:

| place | what it holds |
|---|---|
| `scripts/mutation-verdict.py:969` | `LEG_CEILING = {"ci": 189, "battery": 234}`: 256 jobs less 67 other `ci.yml` jobs, the rows job counted at 18 legs (SPEC-417 R11) |
| `scripts/mutation-verdict.py:971` | `RUST_RELEASE_CEILING = 192`, the most Rust legs the release run deals (SPEC-423) |
| `scripts/mutation-verdict.py:973` | `RUST_MATRIX_LEGS = 256`, the most legs one matrix generates |
| `scripts/mutation-verdict.py:1364` | `rust_ceiling = rust_release or LEG_CEILING["ci"]`: a plan not marked `"release": true` holds its Rust legs to `LEG_CEILING["ci"]` (SPEC-423 R3, R4) |
| `scripts/mutation-verdict.py:1447` | `share = PYTHON_MAX_SHARDS + rust_ceiling`: the joint line's share is the 32 Python legs plus that Rust ceiling (SPEC-418 R4) |
| `scripts/mutation_rows.py:116` | `ROWS_MAX_LEGS = 18`, the most legs the rows are dealt into (SPEC-417 R4) |

## What was measured

| run | job | what it read |
|---|---|---|
| 41972799077, attempt 1, head ab8ee5ee | `hygiene`, 127064675575 | `test_dispatch_shards.TheReleaseFitsOneRun.test_the_ceiling_is_the_runs_job_budget`: `{'ci': 189, 'battery': 234} != {'ci': 185, 'battery': 234}` |
| 41972799077, attempt 1 | `bench-rust-build`, `bench-rust`, `bench-web-build`, `bench-web` | the four bench jobs, each one job inside `ci`'s needs (SPEC-618) |
| 41972799077, attempt 1 | `mutation-plan`, 127064675541 | the Rust shards line ends `legs 1 of ceiling 189`; no joint line is printed, since the run's Python legs were not dealt by cost |

- The test derives each ceiling as 256 less every job the workflow generates besides its Rust leg, the rows job counted at `ROWS_MAX_LEGS` and the Python legs at `PYTHON_MAX_SHARDS`. At the head it counts 71 other `ci.yml` jobs, 67 plus the four bench jobs, and derives 185 (run 41972799077).
- `mutation-weekly.yml` gains no job, so the battery figure stays 234 (run 41972799077, the same test).
- `dev`'s later changes to `ci.yml` add three steps to existing jobs and no job, so the count at `dev` with the bench lanes merged is the same 71.
- The joint line follows the ceiling. `rust_release_ceiling` returns a figure only for a plan marked `"release": true` (`scripts/mutation-verdict.py:1143`), so a pull request plan's Rust legs are held to `LEG_CEILING["ci"]` (line 1364), and the joint line's share is `PYTHON_MAX_SHARDS` plus that ceiling (line 1447). The line prints only when the Python legs are dealt by cost (line 1444), which is why run 41972799077 printed none. At 185 a pull request run's joint line reads 32 Python and 185 Rust legs, and the release run's line keeps 32 and 192.

## What it closes

At 189, with the bench lanes in `ci.yml`, a pull request run that deals 189 Rust legs generates 189 and 71 other jobs, 260 in all, past the 256 one run holds. At 185 every job a pull request run can generate fits its 256, and a population past 185 legs is refused whole by name, never cut.

## What replaces it

- `LEG_CEILING["ci"]` is 185: the run's 256 jobs less 71 other `ci.yml` jobs, the rows job counted at 18 legs and the four bench jobs counted, by SPEC-417 R11's unchanged rule. A pull request whose Rust mutants need more than 185 legs is refused whole by name.
- `LEG_CEILING["battery"]` stays 234: `mutation-weekly.yml` runs no bench job, so its count does not move.
- `RUST_RELEASE_CEILING` stays 192. It is held between `LEG_CEILING["ci"]` and `RUST_MATRIX_LEGS` (`scripts/mutation-verdict.py:1129`), and 192 is above 185, so the release run's Rust legs are held to 192 as before (`scripts/mutation-verdict.py:1143`).
- The pull request run's joint line of fixed shares follows the ceiling, 32 Python and 185 Rust legs; the release run's line keeps 32 and 192 (run 39151539183).

## Why it is admitted

The fall of the pull request run's ceiling is a tightening: a pull request that needs 186 to 189 Rust legs, dealt before, is refused whole by name and split, so no mutant runs unexamined and no run passes its job budget. It was chosen against these alternatives:

- **Keeping 189 by moving the bench lanes out of `ci.yml`.** The lanes run inside the required `ci` check by design (SPEC-618), so a kept slower change is judged where every merge is.
- **Keeping 189 by counting the bench jobs outside the pull request run's jobs.** It changes SPEC-417 R11's rule, and the ceiling would no longer hold every job a pull request's run can generate.
- **Keeping 189 and removing four other jobs.** Each of those jobs carries its own check, and the bench lanes need none of them gone.

## Signature

The owner signs the commit that adds this file. The signature is this ruling's authority; a copy of this file in any unsigned commit carries none.

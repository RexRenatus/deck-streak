# SPEC-621: the short Apple jobs run on the self-hosted runner only when the repository variable is on

- **Issue:** none. **Context(s):** the repository's continuous integration: the Apple job body
  (`.github/workflows/xcframework.yml`) and the workflow hardening test
  (`scripts/tests/test_ci_workflows.py`).
- **Decided by:** ADR-643 (the short Apple jobs move to a self-hosted runner behind a repository
  variable, and the long jobs stay hosted).
- **Status:** accepted
- **Schematic:** none: no component, data flow or state changes shape; one value of three jobs
  becomes an expression.

## 1. The problem, measured

Every Apple job waits for a hosted macOS runner, the three short ones included. Measured on dev
before this change:

- `grep -n 'runs-on' .github/workflows/xcframework.yml` prints six lines, each
  `runs-on: xcode-27`: `xcframework` (line 31), `harness-wire` (185), `card-isolation` (274),
  `harness` (364), `release-and-app` (754) and `verdict` (1391).
- The file runs on `workflow_call` and `workflow_dispatch` (lines 22 to 24).
  `apple-on-change.yml` calls it for a pull request into dev, `apple-on-tag.yml` for a release
  tag, and `testflight-internal.yml` and `testflight-release.yml` for their `framework` job.
- The workflow hardening test admits `xcode-27` to the whole file by its name (`ADMITTED_RUNNERS`,
  line 45 of `scripts/tests/test_ci_workflows.py`), refuses every other runner of every workflow
  through `admitted_runner` (the assertion on line 163), and holds each job of the file to that one
  runner (line 919).
- The two long jobs, `harness` and `release-and-app`, each boot an iPhone and an iPad simulator for
  a second pass, and that pass is not yet green on the self-hosted runner, so they stay hosted.
  `verdict` reads the result of every other job and needs no Apple tool.

## 2. Requirements

R1. **The short jobs reach the self-hosted runner on this repository's own events, behind the
    variable.** `xcframework`, `harness-wire` and `card-isolation` run on the labels `self-hosted`,
    `macOS`, `ARM64` and `ds-mini` for a push, a `workflow_dispatch` and a pull request whose head
    repository is this repository, when the repository variable `DS_MINI` is `on` and the actor is
    not `dependabot[bot]`.
R2. **Every other case runs on `xcode-27`.** In every other case those three jobs run on
    `xcode-27`, so an unset variable, or any value but `on`, keeps today's runners on every event.
R3. **The long jobs and the verdict stay hosted.** `harness`, `release-and-app` and `verdict` run
    on `xcode-27` on every event, their `runs-on` lines kept byte for byte.
R4. **A fork never reaches the self-hosted runner.** A pull request whose head repository is
    another one, or gone, runs every job on `xcode-27`.
R5. **A called workflow routes by its caller's event.** A called workflow reads its caller's event,
    actor and pull request, so each caller's runs are routed by R1 and R2 with no change to the
    callers.

## 3. Acceptance criteria of SPEC-621

Each criterion is one test of the class `TheShortAppleJobsReachTheMiniOnlyBehindTheVariable` in
`scripts/tests/test_ci_workflows.py`, named in the block below.

| criterion | what it decides | test |
|---|---|---|
| A1 | over every event (a push, a dispatch, this repository's pull request, a fork's pull request and a scheduled run), every value of `DS_MINI` (`on`, `ON`, unset and `off`) and both actors (a person and `dependabot[bot]`), the three short jobs resolve to the four labels exactly when the variable is on, the actor is not Dependabot and the event is a push, a dispatch or this repository's pull request, and to `xcode-27` otherwise; the scenario count is asserted (R1, R2, R4, R5) | `test_ci_workflows.py` |
| A2 | `harness`, `release-and-app` and `verdict`, the file's every other job, resolve to `xcode-27` in every scenario (R3) | `test_ci_workflows.py` |
| A3 | planted copies of the expression without the variable clause, without the Dependabot clause, without the same-repository clause, with another label list, and with `harness` routed to the runner are each refused by a scenario that tells it apart (R1 to R4) | `test_ci_workflows.py` |
| A4 | the expression reader evaluates `fromJSON` of a string literal alone, and refuses every other `fromJSON` form by name (R1) | `test_ci_workflows.py` |

```acceptance
A1: python3 -m unittest discover -s scripts/tests -p test_ci_workflows.py -k test_the_short_apple_jobs_reach_the_mini_only_on_this_repositorys_own_events_with_the_variable_on
A2: python3 -m unittest discover -s scripts/tests -p test_ci_workflows.py -k test_the_long_apple_jobs_and_the_verdict_stay_hosted_on_every_event
A3: python3 -m unittest discover -s scripts/tests -p test_ci_workflows.py -k test_a_planted_route_is_refused_by_its_scenario
A4: python3 -m unittest discover -s scripts/tests -p test_ci_workflows.py -k test_the_reader_reads_fromjson_of_a_string_literal_only
```

The runner admission of the hardening test changes with them: `admitted_runner` admits the
expression, and it alone, to the three short jobs by file and job, and `xcode-27` stays the
file's runner for every other job; each job of the file is held to its one runner, and the card
view's host job to the same. Every other workflow's admission is kept.

## 4. File manifest

| path | part | change |
|---|---|---|
| `docs/specs/SPEC-621-the-short-apple-jobs-run-on-the-self-hosted-runner-only-when-the-repository-variable-is-on.md` | C1 | new |
| `docs/decisions/ADR-643-the-short-apple-jobs-move-to-a-self-hosted-runner-behind-a-repository-variable-and-the-long-jobs-stay-hosted.md` | C1 | new |
| `docs/specs/SPEC-002-skeleton-and-local-gate.md` | C1 | amended insert-only: section 9, since R10 holds every workflow to hosted runners |
| `docs/specs/SPEC-336-a-native-client-reaches-the-engine-through-one-allow-listed-ffi-crate.md` | C1 | amended insert-only: one bullet under section 8, since R9 refuses any other runner for the file |
| `scripts/tests/test_ci_workflows.py` | C2 | changed: the runner admission by file and job, the expression reader, and the new class with A1 to A4 |
| `docs/red-first/SPEC-621.md` | C2, C3, C4 | new at C2, appended at C3 and C4 |
| `.github/workflows/xcframework.yml` | C3 | changed: the `runs-on` of the three short jobs, the header comment and the summary line |
| `changelog.d/ci-apple-short-jobs-on-ds-mini.md` | C3 | new |

## 5. What this does NOT cover

- Registering a self-hosted runner with the labels: it is done outside the tree, and no file
  here names a machine.
- Setting the repository variable `DS_MINI`: it is set outside the tree, and until it reads `on`
  every job runs where it runs today.
- Moving `harness` and `release-and-app` to the self-hosted runner: their second simulator pass
  must be green there first, and that move is a later change with its own criteria.

## 6. Risks

- **The routing is not a boundary by itself.** A pull request can edit the workflow file, so the
  expression only routes. The boundaries are the repository's setting that only collaborators
  open pull requests and its rule that a run from an outside contributor waits for approval; A3
  refuses any copy of the expression that would send a fork's pull request to the runner.
- **A runner that is offline.** With the variable on and no runner carrying all four labels, a
  short job waits in the queue and is cancelled after a day; detected as a queued job in the run,
  and cleared by unsetting the variable.
- **A Dependabot run.** A Dependabot pull request comes from this repository, so the actor clause
  keeps it hosted; a planted copy without the clause is refused by A3.

## 7. Amendment: SPEC-688 routes the verdict and the long jobs

SPEC-688, decided by ADR-708, amends this SPEC insert-only. R3 now reads as SPEC-688 R1 to R3:
`verdict` takes this SPEC's expression, and `harness` and `release-and-app` take one expression of
their own that reaches the self-hosted labels only when `DS_MINI` and a second repository
variable, `DS_MINI_LONG`, both read `on`, the actor is not `dependabot[bot]` and the event is a
pull request whose head repository is this repository, and `xcode-27` otherwise. The criterion
that the long jobs and the verdict resolve to `xcode-27` in every scenario is replaced by
SPEC-688's routes, and its test is replaced in `scripts/tests/test_ci_workflows.py`; the class's
other three tests stand, with the verdict among the jobs their short route reads.

A2: superseded by SPEC-688's criteria; its acceptance command above, `-k test_the_long_apple_jobs_and_the_verdict_stay_hosted_on_every_event`, names the test SPEC-688 replaces.

# SPEC-663: actionlint reads every workflow at its pinned release, and every gate job installs one tool set, both at warn

Decided by ADR-685. Follows SPEC-436 and ADR-450 (the promotion rule of ADR-450 D6) and SPEC-428
(the workflow lint job and its pins). Closes #1381.

## 1. The problem, measured

- No workflow is read by actionlint: a fenced `git grep -n -i actionlint` over the tree finds 0
  lines. The one workflow linter, zizmor, runs in the `workflow-lint` job over `.github`
  (SPEC-428 R4).
- `.github/workflows/` holds the workflow files and `.github/actions/` holds none (`git ls-tree -r
  --name-only` filtered to each folder).
- The gate jobs are the aggregate job `ci` of `ci.yml` and every job it needs, plus `fragment`
  (`changelog.yml`) and `dependency-review` (`dependency-review.yml`), the three required checks
  of `.github/rulesets/dev.json`, each job on one hosted runner image. Nothing compares the
  tools they install; read by the routes of R8, each tool two or more of them install is held at
  one version by one route.
- No actionlint run is measured here: this delivery's own first `workflow-lint` run measures
  today's findings, and R5 lists them.

## 2. Requirements

R1. The `workflow-lint` job runs actionlint once, as `actionlint -shellcheck= -pyflakes= -format
    '{{json .}}'`, over every workflow file it finds, with no `-ignore` option and no
    configuration file, so the run depends on the pinned binary alone.
R2. The job installs actionlint at one pinned release by the tree's checked-download route: the
    archive is fetched from the release, checked against the full digest its step holds by
    `sha256sum -c -`, and put on the path; a digest that does not match fails the step.
R3. The run's JSON output and its exit are saved beside each other. The census reads them; an
    output or exit that is missing, an exit other than 0 or 1, an output that is not a list of
    findings each holding its file, line, column, kind and message, an exit of 0 beside a finding,
    an exit of 1 beside none, and a finding outside the workflow files read are each VOID, which
    fails.
R4. Each finding, by file, line and kind, that the listing does not hold prints one warning line,
    and each listed finding no longer reported prints one; neither fails at warn.
R5. The listing holds today's findings at the delivery's base, each with its reason, read from the
    delivery's own first `workflow-lint` run and named with that run's id.
R6. Both checks hold the level `warn`. A move to `fail` is its own delivery under ADR-450 D6's
    promotion rule; a move back is a weakening.
R7. The census reads the gate jobs through the reader `test_ci_workflows.py` holds: `ci`, every job
    its `needs` names, and every job of `changelog.yml` and `dependency-review.yml`. It prints
    `examined N job(s)` and refuses 0; it prints `examined N workflow(s)` over the workflow files and
    refuses 0.
R8. A tool is what one step installs: an install-action entry, a setup action, a checked download
    named by its digest variable, a `cargo install`, the toolchain file `rustup show` reads, or any
    other action. A tool two or more gate jobs install at two versions, or by two routes, is a
    parity site and prints one warning line unless the listing holds it; a tool one job installs is
    never a site.
R9. Each arm is shown on a planted tree under `scripts/tests/fixtures/workflow-lint/`: a clean
    saved output, a saved output with a finding, a fatal exit, two jobs that install one tool at
    one version, two jobs that install it at two versions, two jobs that install it by two routes,
    one job that downloads two tools, each checked by its own saved file, and installs one again,
    and an empty tree, which is VOID. No planted case runs actionlint.
R10. Every existing test, pin, row and refusal stands; `test_ci_workflows.py` gains only the
    register entry for the census's one read site, inserted.

## 3. Acceptance criteria

| id | criterion | red it must show first | decided by |
|---|---|---|---|
| A1 | the `workflow-lint` job runs the one actionlint command (R1) | red at the base, where no job runs it | `test_the_workflow_lint_job_runs_the_one_actionlint_command` |
| A2 | a finding no listing holds warns, and a listed one does not (R4) | red at the red commit, whose arm reports nothing | `test_a_saved_finding_warns_unless_it_is_listed` |
| A3 | a clean saved output reads no finding (R3) | not red: the stub reads every output as clean | `test_a_clean_saved_output_reads_no_finding` |
| A4 | a fatal exit and a missing output are VOID (R3) | red at the red commit, whose reader reads both as clean | `test_an_unread_saved_output_is_void` |
| A5 | one tool at one version holds no site (R8) | not red: the stub already finds none | `test_two_jobs_that_install_one_tool_at_one_version_hold_no_site` |
| A6 | one tool at two versions is a site (R8) | red at the red commit, whose arm finds no site | `test_two_jobs_that_install_one_tool_at_two_versions_are_a_site` |
| A7 | one tool by two routes is a site (R8) | red at the red commit, whose arm finds no site | `test_two_jobs_that_install_one_tool_by_two_routes_are_a_site` |
| A8 | an empty tree is VOID (R7) | not red: the house reader refuses it at the base | `test_an_empty_population_is_void` |
| A9 | a download takes the tool of the check that names its saved file, and a tool one job holds two ways is no site (R8) | not red: the red commit pairs each download and the stub finds no site | `test_one_job_that_downloads_two_tools_names_each_by_its_check_and_holds_no_site` |

```acceptance
A1: python3 -m unittest discover -s scripts/tests -p test_workflow_lint.py -k test_the_workflow_lint_job_runs_the_one_actionlint_command
A2: python3 -m unittest discover -s scripts/tests -p test_workflow_lint.py -k test_a_saved_finding_warns_unless_it_is_listed
A3: python3 -m unittest discover -s scripts/tests -p test_workflow_lint.py -k test_a_clean_saved_output_reads_no_finding
A4: python3 -m unittest discover -s scripts/tests -p test_workflow_lint.py -k test_an_unread_saved_output_is_void
A5: python3 -m unittest discover -s scripts/tests -p test_workflow_lint.py -k test_two_jobs_that_install_one_tool_at_one_version_hold_no_site
A6: python3 -m unittest discover -s scripts/tests -p test_workflow_lint.py -k test_two_jobs_that_install_one_tool_at_two_versions_are_a_site
A7: python3 -m unittest discover -s scripts/tests -p test_workflow_lint.py -k test_two_jobs_that_install_one_tool_by_two_routes_are_a_site
A8: python3 -m unittest discover -s scripts/tests -p test_workflow_lint.py -k test_an_empty_population_is_void
A9: python3 -m unittest discover -s scripts/tests -p test_workflow_lint.py -k test_one_job_that_downloads_two_tools_names_each_by_its_check_and_holds_no_site
```

## 4. File manifest

| path | change |
|---|---|
| `.github/workflows/ci.yml` | changed: three steps inserted at the end of `workflow-lint` |
| `scripts/tests/test_workflow_lint.py` | added: the census |
| `scripts/tests/test_ci_workflows.py` | changed: one register entry inserted in `NOT_WORKFLOW_READS` |
| `scripts/tests/fixtures/workflow-lint/clean/actionlint.json` | added: planted |
| `scripts/tests/fixtures/workflow-lint/clean/actionlint.exit` | added: planted |
| `scripts/tests/fixtures/workflow-lint/finding/actionlint.json` | added: planted |
| `scripts/tests/fixtures/workflow-lint/finding/actionlint.exit` | added: planted |
| `scripts/tests/fixtures/workflow-lint/fatal/actionlint.json` | added: planted, empty |
| `scripts/tests/fixtures/workflow-lint/fatal/actionlint.exit` | added: planted |
| `scripts/tests/fixtures/workflow-lint/one-version/ci.yml` | added: planted |
| `scripts/tests/fixtures/workflow-lint/two-versions/ci.yml` | added: planted |
| `scripts/tests/fixtures/workflow-lint/two-routes/ci.yml` | added: planted |
| `scripts/tests/fixtures/workflow-lint/one-job-two-tools/ci.yml` | added: planted |
| `scripts/tests/fixtures/workflow-lint/empty/README.md` | added: planted |
| `scripts/mutation-rows.d/S66300-S66399.json` | added: the rows of section 9 |
| `docs/specs/SPEC-663-actionlint-reads-every-workflow-at-its-pinned-release-and-every-gate-job-installs-one-tool-set-both-at-warn.md` | added: this file |
| `docs/decisions/ADR-685-actionlint-runs-from-a-checksum-verified-download-and-its-findings-and-the-gate-jobs-tool-parity-start-at-warn.md` | added: the decision |
| `docs/red-first/SPEC-663.md` | added: the record |
| `changelog.d/ci-actionlint-and-tool-parity.md` | added: the fragment |

## 5. What this does NOT cover

- No finding is cured and no parity site is changed: each is listed with its reason (#1381).
- No check moves to `fail`; that is a later delivery of its own under ADR-450 D6 (#1381).
- No other workflow linter, no shellcheck or pyflakes run inside actionlint, and no further zizmor
  rule (#862).
- No install route outside the gate jobs is read, and no image tool is pinned (#860).
- No shared download helper with any other pinned download; each step keeps its own (#1381).

## 6. Risks

- **actionlint's JSON keys differ from the census's.** The first run reads VOID by name, and the
  key list is corrected before the listing commit.
- **A finding names a line that later moves.** The old entry prints as no longer reported and the
  new one as unlisted, both at warn; the listing is re-derived, never widened.
- **The release stops being fetchable or its digest changes.** The download step fails by name;
  the cure is a re-pin by a reviewed edit.
- **The register entry's text differs from the read site's.** The reader census in
  `test_ci_workflows.py` refuses it by module, name and text, in the `hygiene` job.

## 7. What only CI proves

Today's findings, from the first `workflow-lint` run of this delivery, read by job name; the
live parity lines, from the same job and from `hygiene`; the rows, in `mutation-rows`.

## 8. Formal model

None: the census is a function of the tree's text and two saved files, and the steps add no
writer of any shared state.

## 9. Mutation rows

The band file `scripts/mutation-rows.d/S66300-S66399.json` holds eight rows on the census's arms and the
command, each killed by its planted test by full id.

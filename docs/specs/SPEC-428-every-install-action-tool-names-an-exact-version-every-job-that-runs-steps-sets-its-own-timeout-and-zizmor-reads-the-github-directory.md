# SPEC-428: every install-action tool names an exact version, every job that runs steps sets its own timeout, and zizmor reads the `.github` directory

- **Issue:** none; this delivery lands on dev after the release pull request #822 and closes
  nothing.
  **Context(s):** CI (`.github/workflows/`) and its tests (`scripts/tests/`); no crate, no Mini
  App source, no script and no ruleset changes.
- **Decided by:** ADR-442 (exact versions read as a name, `@` and three numbers, at every site the
  install action holds; a timeout on every job that runs steps, call jobs counted apart; zizmor's
  whole command pinned with `.github` as its operand; each site pinned to the version its own job
  installed; the checks in one class of the workflow test module; a newest-release pin, a later
  cure, a step-level timeout, the repository root as zizmor's operand, a new module and an
  external lint rule are rejected).
- **Schematic:** `docs/schematics/ci-tool-pins-and-job-timeouts.md` (data flow: the workflow
  files, the three checks with their populations, matchers and examined counts, the sites cured,
  and where each check runs).
- **Status:** delivered by the pull request that adds this file, with its tests and
  `docs/red-first/SPEC-428.md`. **Mutation band:** `S42800-S42899` (section 9). **Model:** none
  (section 8). Amends ADR-055 by a note in its own file.

## 1. The problem, measured

Every figure was read at `ac7ebd752f3022da6e5f6b5d7beb5dfc36872e27`.

- **Tools with no version.** `git grep -n -A 5 -e 'taiki-e/install-action' ac7ebd75 --
  .github/workflows` names 20 sites in 6 workflow files. 12 name an exact version for every tool
  (`cargo-mutants` at 6 and `cargo-nextest` at 6, all in the mutation jobs). 8 sites name 10
  entries with none:

  | file:line | job | `tool` |
  |---|---|---|
  | `ci.yml:44` | `rust` | `cargo-nextest,cargo-deny` |
  | `ci.yml:114` | `engine` | `cargo-nextest` |
  | `ci.yml:166` | `release` | `cargo-nextest` |
  | `ci.yml:318` | `hygiene` | `cargo-deny` |
  | `ci.yml:994` | `workflow-lint` | `zizmor` |
  | `engine-measure.yml:72` | `measure` | `cargo-deny` |
  | `release.yml:183` | `audit-sync-server` | `cargo-deny` |
  | `rust-cache.yml:58` | `warm` | `cargo-nextest,cargo-deny` |

  Each such install takes whatever the action's own commit names as the newest release; each job
  log at the base reads `installing <tool>@latest`. So a bump of the action's sha, which
  Dependabot proposes on its schedule, changes the tool, and no line of this repository shows it.
  Nothing reads the `tool` input today: SPEC-002 R10's check reads each `uses` ref only.
- **Jobs with no timeout.** `git grep -n -E '^jobs:|^  [A-Za-z0-9_-]+:[[:space:]]*$|^    timeout-minutes:|^    uses:' ac7ebd75 -- .github/workflows`
  names 52 jobs in 15 workflow files. 4 call a reusable workflow by their own `uses`
  (`apple-on-change.yml`'s and `apple-on-tag.yml`'s `apple`, and both TestFlight lanes'
  `framework`). Of the other 48, 46 set `timeout-minutes` and 2 set none: `changelog.yml`'s
  `fragment` (`:20`) and `testflight-rerelease-check.yml`'s `rerelease` (`:16`). `git grep -c
  -e 'timeout-minutes'` counts 46 lines, every one a job's, so no step sets a timeout.
- **What zizmor audits.** `ci.yml:996` runs `zizmor --no-online-audits --persona regular
  .github/workflows`. Its log at job 116733923569 (9 s) names 15 completed inputs, every one a
  workflow file, and no finding. `.github/dependabot.yml` (30 lines, 3 update entries) is never
  audited. `git ls-tree -r --name-only ac7ebd75` names no action definition, no zizmor
  configuration and no other `.github` directory, so the inputs `.github` reaches are the 15
  workflow files and `.github/dependabot.yml`.
- **What the cures must keep.** The versions each unversioned site installs today are read from
  its job's log (ADR-442 D4 names each job with its duration). `release.yml` and `rust-cache.yml`
  have never run, since neither is on the default branch; they use the same action commit as
  `ci.yml`. `changelog.yml`'s `fragment` ran 5 to 8 s over its last 30 runs (slowest successful
  run: job 116499075615, 8 s). `testflight-rerelease-check.yml` has never run, for the same
  reason.

## 2. Requirements

R1. Every mapping in every workflow file under `.github/workflows/`, `.yml` and `.yaml` alike,
    whose `uses` names `taiki-e/install-action` in any letter case, carries a `tool` input, and
    every entry of that input, split on commas and whitespace, is a tool name, `@` and three
    dot-separated numbers with nothing after them. A bare name, `latest`, one or two numbers, a
    range operator, a `v` prefix, a pre-release or build suffix and an expression are each refused
    by a line naming the file, the job, the step and the entry.
R2. The pin check prints the sites and the entries it examined and refuses 0, and its site count
    equals the count of the action's `uses` lines in the files' text outside comments.
R3. Every job of every workflow file that does not call a reusable workflow by its own `uses` sets
    its own `timeout-minutes` to a whole number of at least 1. A step's timeout does not count. A
    call job is counted apart and never checked. The check prints the jobs it checked and the call
    jobs it counted, and refuses 0.
R4. The `workflow-lint` job runs zizmor as exactly `zizmor --no-online-audits --persona regular
    .github`, and that is the only zizmor command in any workflow. The inputs that command
    reaches, derived from the repository's workflow files and its Dependabot configuration,
    include `.github/dependabot.yml` and number the workflow files plus one. The check prints the
    commands and the inputs it examined and refuses 0.
R5. At the base, each of the 8 sites in section 1 names each of its 10 entries at the version its
    own job installed (ADR-442 D4), `rust-cache.yml`'s `warm` input stays equal to `ci.yml`'s
    `rust` input, `fragment` sets `timeout-minutes: 5`, `rerelease` sets `timeout-minutes: 10`,
    and zizmor's operand is `.github`. No other line of any workflow changes.
R6. Each check refuses each defect planted into a copy of the workflows' text by the planted
    defect's own line, after the unplanted copy reads clean; the plants include a `.yaml` file and
    a form each check must accept.
R7. `engine_job_problems` counts the `engine` job's test runner (SPEC-038 R14) only when an
    entry of its install's `tool` input, split on commas and stripped, equals the one exact
    string the pin record `ENGINE_NEXTEST_PIN` in R8's block holds: `cargo-nextest` at the
    version R5 pins and A1 holds. The check accepts exactly that one string where it accepted
    exactly the bare name, so the bare name and every other version read as no test runner,
    and no check is widened.
R8. The checks of R1 to R7 live in `scripts/tests/test_ci_workflows.py`, in the one class
    `CiToolsArePinnedAndJobsBounded` and one block of helpers of their own, placed before
    `TheAppleBuildRunsFromOneBody`. They read workflows only through `read_hardened` and
    `workflow_file_text`, and the filesystem only by names, never a file's contents. No existing
    test function changes but `engine_job_problems`' one matcher line (R7) and the expected tool
    strings of `test_rust_cache_workflow.py`'s `TOOLS` and `test_release_workflow.py`'s audit
    job.
R9. ADR-055 gains a note, inserted after its last line, that records zizmor's new operand; no
    line of ADR-055 above it changes.

## 3. Acceptance criteria of SPEC-428

| id | criterion | red it must show first | decided by |
|---|---|---|---|
| A1 | every install-action site in every workflow file names every tool entry at an exact version, and the sites examined equal the action's `uses` lines in the text (R1, R2, R5) | red at the base, where 8 sites name 10 entries with no version | `test_every_install_action_tool_names_an_exact_version` |
| A2 | each pin defect planted into a copy is refused by its own line: a bare name, `latest`, one number, two numbers, a range, a `v` prefix, a pre-release suffix, an expression, a missing `tool` input, a floating second entry of a comma list, a floating entry of a whitespace list, the action spelt in another letter case, and a bare name in a `.yaml` file; and an exact whitespace list is accepted (R1, R6) | red at the base, whose control holds 10 entries with no version | `test_each_planted_tool_pin_defect_is_refused_by_its_own_line` |
| A3 | every job that runs steps sets its own whole-number timeout of at least 1, `fragment` and `rerelease` among those checked, the 4 call jobs among those counted apart (R3, R5) | red at the base, where `fragment` and `rerelease` set none | `test_every_job_that_runs_steps_sets_its_own_timeout` |
| A4 | each timeout defect planted into a copy is refused by its own line: a timeout removed from a job of run steps and from a job of action steps, a timeout of 0, an expression, a word, a number with a unit after it, a timeout moved onto a step, and an untimed job in a `.yaml` file; and an untimed call job is accepted (R3, R6) | red at the base, whose control holds 2 untimed jobs | `test_each_planted_timeout_defect_is_refused_by_its_own_line` |
| A5 | the one zizmor command is `zizmor --no-online-audits --persona regular .github`, and its inputs include `.github/dependabot.yml` and number the workflow files plus one (R4, R5) | red at the base, whose operand `.github/workflows` holds no Dependabot configuration | `test_zizmor_audits_the_github_directory_and_its_dependabot_configuration` |
| A6 | each zizmor defect planted into a copy is refused by its own line: the old operand, `--collect=workflows`, `--no-exit-codes`, `--format sarif`, the repository root as operand, a second zizmor command, the command removed, and its exit swallowed by `\|\| true` (R4, R6) | red at the base, whose control runs zizmor over `.github/workflows` | `test_each_planted_zizmor_defect_is_refused_by_its_own_line` |
| A7 | the engine job's check counts the pin record's exact `cargo-nextest` entry as its test runner, and refuses the bare name, another version and a longer string that starts with the pin (R7) | red at the base, whose check reads the pinned entry as another tool | `test_the_engine_check_counts_only_the_pinned_nextest` |

```acceptance
A1: python3 -m unittest discover -s scripts/tests -p test_ci_workflows.py -k test_every_install_action_tool_names_an_exact_version
A2: python3 -m unittest discover -s scripts/tests -p test_ci_workflows.py -k test_each_planted_tool_pin_defect_is_refused_by_its_own_line
A3: python3 -m unittest discover -s scripts/tests -p test_ci_workflows.py -k test_every_job_that_runs_steps_sets_its_own_timeout
A4: python3 -m unittest discover -s scripts/tests -p test_ci_workflows.py -k test_each_planted_timeout_defect_is_refused_by_its_own_line
A5: python3 -m unittest discover -s scripts/tests -p test_ci_workflows.py -k test_zizmor_audits_the_github_directory_and_its_dependabot_configuration
A6: python3 -m unittest discover -s scripts/tests -p test_ci_workflows.py -k test_each_planted_zizmor_defect_is_refused_by_its_own_line
A7: python3 -m unittest discover -s scripts/tests -p test_ci_workflows.py -k test_the_engine_check_counts_only_the_pinned_nextest
```

All seven are read red and green from CI's `hygiene` job, which runs `scripts/tests`.

## 4. File manifest

| path | part | change |
|---|---|---|
| `.github/workflows/ci.yml` | CI | changed: the `tool` input of the `rust`, `engine`, `release`, `hygiene` and `workflow-lint` sites names each tool's exact version; zizmor's operand is `.github` |
| `.github/workflows/engine-measure.yml` | CI | changed: the `measure` site's `tool` input names its exact version |
| `.github/workflows/release.yml` | CI | changed: the `audit-sync-server` site's `tool` input names its exact version |
| `.github/workflows/rust-cache.yml` | CI | changed: the `warm` site's `tool` input names each exact version, equal to `ci.yml`'s `rust` input |
| `.github/workflows/changelog.yml` | CI | changed: `fragment` sets `timeout-minutes: 5` |
| `.github/workflows/testflight-rerelease-check.yml` | CI | changed: `rerelease` sets `timeout-minutes: 10` |
| `scripts/tests/test_ci_workflows.py` | CI tests | changed: the class `CiToolsArePinnedAndJobsBounded` (A1 to A7) and its block of helpers before `TheAppleBuildRunsFromOneBody`; `engine_job_problems`' one matcher line (R7) |
| `scripts/tests/test_rust_cache_workflow.py` | CI tests | changed: `TOOLS` names the pinned entries |
| `scripts/tests/test_release_workflow.py` | CI tests | changed: the audit job's expected `tool` names its pinned entry |
| `scripts/mutation-rows.d/S42800-S42899.json` | CI tests | added: rows S42800 to S42828 (section 9) |
| `docs/specs/SPEC-428-every-install-action-tool-names-an-exact-version-every-job-that-runs-steps-sets-its-own-timeout-and-zizmor-reads-the-github-directory.md` | docs | added: this file |
| `docs/decisions/ADR-442-ci-tools-are-pinned-to-the-versions-ci-installs-today-every-job-that-runs-steps-is-timed-and-zizmor-reads-the-github-directory.md` | docs | added: the decision |
| `docs/decisions/ADR-055-the-gate-runs-in-parallel-jobs-and-only-a-push-saves-a-cache.md` | docs | changed: one note inserted after its last line (R9) |
| `docs/schematics/ci-tool-pins-and-job-timeouts.md` | docs | added: the schematic |
| `docs/red-first/SPEC-428.md` | docs | added: the red-first record |
| `changelog.d/ci-tool-pins-428.md` | docs | added: the fragment |

## 5. What this does NOT cover

- It does not change how actions are referenced: SPEC-002 R10's full-commit-sha check stands as it
  is, and no action ref, version comment or Dependabot setting changes (#861).
- It does not hold any install route but `taiki-e/install-action` to a version: `cargo install`,
  `pip`, a `curl` download, a setup action and a container image are not read by these checks
  (#860).
- It adds no further CI lint rule and no allow-list, exemption or suppression arm to any of the
  three checks (#862).
- It adds nothing that notices a newer release of a pinned tool: Dependabot reads action
  references, not the `tool` input, so a re-pin is a reviewed edit by hand (#863).

## 6. Risks

- **A pin falls behind a security fix.** The pinned tool keeps its version until someone re-pins
  it; ADR-442 D6 re-pins it, never removes it, once it blocks a fix for more than one release.
- **A pinned version stops being fetchable.** The install then fails by name in its job, and
  `ci` reads red: the pin working. The cure is a re-pin.
- **The action changes how it splits `tool`.** The check would then read entries the action does
  not install. The site count still agrees with the text, so the entry count is the signal.
- **zizmor stops collecting the Dependabot configuration from a directory.** The test's model
  would still count the file in; the completion line in the `workflow-lint` log is the guard, and
  this delivery's own run reads it.
- **A finding on `.github/dependabot.yml`.** It fails `workflow-lint` and so `ci`. No run has
  audited the file yet; this delivery's own run is the first, read before it lands.
- **A row's anchor moves.** The engine and release sites hold byte-identical install steps, so the
  rows on them anchor from a line unique to each job; a find that no longer occurs exactly once is
  refused by the rows census by stem.

## 7. What only CI proves

A1 to A7's red and green, read in the `hygiene` job by name; the rows' census and proof in the
`mutation-rows` job; the `workflow-lint` log naming `.github/dependabot.yml` among its completed
inputs and no finding; and each install job's log naming its pinned version. `release.yml` and
`rust-cache.yml` run only once their files reach the default branch, so their two sites are proved
by the check and by the same action commit's installs in `ci.yml`, not by a run of their own.

## 8. Formal model

None. Each install writes only its own runner's tool directory, which no cache key or save path
holds; the checks are functions of the tree's text; and the two new timeouts add no actor and no
write. No `@phx covers` line under `formal/` names a file this delivery changes.

## 9. Mutation rows

| row | behaviour | mutant | killer |
|---|---|---|---|
| S42800 | the `rust` site's `cargo-nextest` names its version | the version dropped | A1 |
| S42801 | the `rust` site's `cargo-deny` names its version | the version dropped | A1 |
| S42802 | the `engine` site's `cargo-nextest` names its version | the version dropped | A1 |
| S42803 | the `release` site's `cargo-nextest` names its version | the version dropped | A1 |
| S42804 | the `hygiene` site's `cargo-deny` names its version | the version dropped | A1 |
| S42805 | the `workflow-lint` site's `zizmor` names its version | the version dropped | A1 |
| S42806 | `engine-measure.yml`'s `cargo-deny` names its version | the version dropped | A1 |
| S42807 | `release.yml`'s `cargo-deny` names its version | the version dropped | A1 |
| S42808 | `rust-cache.yml`'s `cargo-nextest` names its version | the version dropped | A1 |
| S42809 | `rust-cache.yml`'s `cargo-deny` names its version | the version dropped | A1 |
| S42810 | `fragment` sets its timeout | its `timeout-minutes` line removed | A3 |
| S42811 | `rerelease` sets its timeout | its `timeout-minutes` line removed | A3 |
| S42812 | zizmor reads `.github` | the operand back to `.github/workflows` | A5 |
| S42813 | zizmor collects every input kind | `--collect=workflows` added | A5 |
| S42814 | an exact version has three numbers | the second and third numbers made optional | A2 |
| S42815 | an exact version has no `v` prefix | a `v` admitted before the numbers | A2 |
| S42816 | an exact version ends at its third number | `fullmatch` to `match` | A2 |
| S42817 | entries split on commas and whitespace | split on commas only | A2 |
| S42818 | the action is matched in any letter case | the case fold dropped | A2 |
| S42819 | a site with no `tool` input is refused | the refusal's condition made false | A2 |
| S42820 | a call job is counted apart | the call-job test made false | A3 |
| S42821 | only a job's own `uses` makes it a call job | `uses` read anywhere in the job | A4 |
| S42822 | a timeout of 0 is refused | the first digit allowed to be 0 | A4 |
| S42823 | a timeout with anything after its digits is refused | `fullmatch` to `match` | A4 |
| S42824 | the Dependabot configuration is among zizmor's known inputs | that member dropped | A5 |
| S42825 | exactly one zizmor command runs | the count test admits none | A6 |
| S42826 | the engine check compares entries with the pin record | the compared operand back to the bare `"cargo-nextest"` | A7 |
| S42827 | the engine check compares whole entries, the version included | each entry cut to its name before `@` (`t.strip().split("@")[0]`) and compared with the bare name | A7 |
| S42828 | the pin record holds `cargo-nextest` at its exact version | its value made the bare `"cargo-nextest"` | A7 |

Each row's anchor is unique in its file and byte-identical to the line or lines the mutant
replaces; the rows on the `engine` and `release` sites anchor from a line unique to their job
down to the site's `tool` line. Each killer is the criterion's test, run by CI's `mutation-rows`
job. The fourteen rows on workflow files are killed by A1, A3 and A5, which read the live tree;
the fifteen on `scripts/tests/test_ci_workflows.py` by the planted tests and the live ones, all in a
module that runs only in CI. S42826 and S42827 anchor on `engine_job_problems`' matcher and
S42828 on the pin record's line, each find written from the cure's text: A7's record plant kills
S42826, its another-version plant S42827, and its bare-name plant S42828, which the live engine
test (SPEC-038 R14) reads red too.

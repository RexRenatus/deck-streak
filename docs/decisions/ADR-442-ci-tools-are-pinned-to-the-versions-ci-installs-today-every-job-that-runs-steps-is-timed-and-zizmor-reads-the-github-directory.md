---
status: "accepted"
decision-makers: "@RexRenatus (owner), the DeckStreak architect"
---

# ADR-442: CI tools are pinned to the versions CI installs today, every job that runs steps is timed, and zizmor reads the `.github` directory

Decides SPEC-428.

## Context and Problem Statement

`scripts/tests/test_ci_workflows.py` already holds every action reference to a full commit sha
(SPEC-002 R10), but nothing reads what an action is asked to install, how long a job may run, or
which files the workflow lint audits. At `ac7ebd752f3022da6e5f6b5d7beb5dfc36872e27`:

- `taiki-e/install-action` is used at 20 sites in 6 workflow files. 12 sites name an exact
  version for every tool. 8 sites name 10 tool entries with no version: `cargo-nextest` at 4,
  `cargo-deny` at 5 and `zizmor` at 1. Each of those installs whatever the action's own commit
  calls the newest release, so a bump of the action's sha, which Dependabot proposes on its
  schedule, moves the tool with it and no reviewer reads the change.
- The 15 workflow files hold 52 jobs. 4 call a reusable workflow by their own `uses`, 46 set
  `timeout-minutes`, and 2 set none: `changelog.yml`'s `fragment` and
  `testflight-rerelease-check.yml`'s `rerelease`. No step anywhere sets a timeout of its own.
- The `workflow-lint` job runs `zizmor --no-online-audits --persona regular .github/workflows`.
  Its run at job 116733923569 (9 s) completed 15 workflow files and no other input, so
  `.github/dependabot.yml` is never audited.

The question is how to hold all three so that they cannot erode silently, with no tool changing
under its pin and no allow-list arm.

## Decision Drivers

- A tool changes only by an edit that a reviewer reads in this repository.
- Every check prints what it examined and refuses an empty population, and its count is compared
  with a control the check did not build.
- A check reads workflows through the module's one loader, as the hardening checks do.
- Three rules, and nothing beyond them.
- The cure changes no tool, no job body and no check's name.

## Considered Options (the alternatives each was chosen against)

D1, what the pin check reads, and what counts as exact:

- Chosen: every mapping in a read workflow whose `uses` names `taiki-e/install-action` in any
  letter case, its `tool` input split on commas and whitespace, each entry a tool name, `@` and
  three dot-separated numbers with nothing after them, because that covers every form the tree
  uses (one tool, a comma list, `name@version`) and the whitespace list the action also reads,
  and every looser form names a moving target.
- A version of one or two numbers, a bare major or a major and minor: rejected because the
  action installs the newest release inside it, so the tool still moves.
- `latest`, a range operator or an expression: rejected because each resolves at run time, and
  an expression cannot be read from the tree at all.
- A `v` prefix, or a pre-release or build suffix: rejected because no site uses one and the
  matcher would have to trust the action to read each spelling; a tool that ships only such
  versions takes an amendment of this decision, not a looser matcher.
- Reading the action's ref as its tool, the ref-per-tool shorthand: rejected because SPEC-002
  R10's check already refuses any ref that is not a full commit sha, so the shorthand cannot reach
  a site; a site with no `tool` input is refused by name instead.
- One version per tool across every site: rejected because it is a fourth rule, and a re-pin of
  one site is a reviewed edit either way.

The known false positives: an exact version spelt with a `v` prefix or a suffix, and a `tool`
input given by an expression that would resolve to an exact version. The known false negatives: a
tool the action builds from source at an exact version under its default fallback, whose own
dependencies the entry does not pin; a tool installed by any other route; an install inside a
composite action definition (there is none in the tree); and a site in a file outside
`.github/workflows/`.

D2, which jobs must set `timeout-minutes`, and to what:

- Chosen: every job that does not call a reusable workflow by its own `uses` sets its own
  `timeout-minutes` to a whole number of at least 1, call jobs are counted apart and never checked,
  and a step's timeout does not count, because only the job's key bounds its set-up, every step
  and its post steps together.
- Counting a step-level timeout: rejected because a step's bound leaves the job's other steps,
  its set-up and its post steps unbounded.
- Judging call jobs too: rejected because a job that calls a reusable workflow takes no
  `timeout-minutes`; each called job is checked in its own file.
- A band for each job, as the engine and release jobs have: rejected because the rule is
  presence, and a band is a per-job decision with its own measurement.
- An expression value: rejected because the check cannot read what it bounds.

The values set: `fragment` 5 minutes, from its last 30 runs, which took 5 to 8 s each; its
slowest successful run is job 116499075615 (8 s). Five minutes is the bound `base-is-dev`, a job of
one script step, already carries. `rerelease` 10 minutes: it has never run, because its workflow
is not on the default branch, so neither its schedule nor its dispatch has fired; 10 minutes is
the bound each TestFlight lane's first job and `workflow-lint` carry, each a checkout and a
short script.

D3, how zizmor reads `.github/`:

- Chosen: `zizmor --no-online-audits --persona regular .github`, the only zizmor command in any
  workflow, with zizmor pinned at the version CI installs today, because a directory input
  collects every input kind under it, so the Dependabot configuration is collected beside the 15
  workflows, and CI's log names each input it completed.
- The repository root, `.`: rejected because it collects workflows and action definitions
  anywhere in the tree, a vendored one included, a wider population than the rule names.
- Two explicit inputs, `.github/workflows .github/dependabot.yml`: rejected because an input kind
  added under `.github/` later would be missed, and the rule names the directory.
- An allow-list of options in the test instead of the whole command: rejected because a new
  option that narrows the inputs or drops the exit code would pass until someone listed it; a
  whole-command pin makes every change a reviewed test edit.
- A version floor for zizmor in the test: rejected because the pin check already holds the
  version, and CI's completion line proves the audit by name.

The test proves the population two ways. It derives the inputs the operand reaches from the
repository's workflow files and its Dependabot configuration, and refuses unless
`.github/dependabot.yml` is among them and their count is the workflow files' plus one; and CI's
`workflow-lint` log at the cure names `.github/dependabot.yml` among the completed inputs. The
findings over the new population at the base are not measured: no CI run has audited the file,
and this delivery runs no tool outside CI. The file sets a cooldown on all three update entries
and allows no external code execution, which is what zizmor's two Dependabot audits read; the
cure's run is the measurement, and a finding there stops the delivery before it lands.

D4, the cures at the base:

- Chosen: each of the 8 sites at the exact version its own job installed at the base, so no tool
  changes under its pin, because a pin that changes the tool would land a tool upgrade inside a
  lint change.
- The newest release of each tool: rejected because the tool would change under the pin in the
  same delivery, so a new advisory reading, audit or test-runner behaviour would land with a lint
  change.
- Leaving the sites to a later delivery: rejected because the check would read red at land, or
  need an allow-list arm, and no arm exists.
- Adding `fallback: none` at each cured site: rejected because it changes how a missing build
  fails, which none of the three rules asks for.

Each installed version is read from its own job's log at the base.
The `rust` job, job 116733923694 (41m50s), installed `cargo-nextest` and `cargo-deny`.
The `engine` jobs, 116733923630 (3m07s) and 116733923715 (2m38s), installed `cargo-nextest`.
The `release` job, job 116733923753 (41m05s), installed `cargo-nextest`.
The `hygiene` job, job 116733923723 (19m03s), installed `cargo-deny`.
The `workflow-lint` job, job 116733923569 (9 s), installed `zizmor`.
`engine-measure.yml`'s `measure` job, job 116500686733 (5m10s), installed `cargo-deny`.
The two sites whose workflows have never run, `release.yml`'s `audit-sync-server` and
`rust-cache.yml`'s `warm`, take the versions the same action commit installed in the jobs above,
and the `warm` site's input stays equal to the `rust` job's, as its test holds.

D5, one home, and its population:

- Chosen: one class, `CiToolsArePinnedAndJobsBounded`, and one block of helpers of its own in
  `scripts/tests/test_ci_workflows.py`, placed together before `TheAppleBuildRunsFromOneBody`,
  reading workflows through `read_hardened`, each check printing its examined counts and failing
  on 0, because the hardening checks this extends live in that module, and a block of its own
  keeps a later delivery's checks in the same module from editing the same functions.
- A new test module: rejected because it would read workflows through this module's loader all
  the same, and join the read census as a new member to classify.
- A zizmor rule, or any external lint configuration, for the pins: rejected because the check
  would then live in a tool's configuration that no test of this repository reads, and it would
  print no examined count.
- New tests inside `WorkflowsAreHardened`: rejected because a later delivery adding checks to
  that class would edit the same lines, and the two diffs would collide.

Three existing checks move with the pins, each named in SPEC-428 section 4. The expected tool
strings of the `warm` site in `test_rust_cache_workflow.py` and the audit job in
`test_release_workflow.py` become the pinned ones, each an exact string for an exact string.
The third, `engine_job_problems` (SPEC-038 R14), counts the `engine` job's test runner by the
bare name `cargo-nextest` alone, so the pinned site would read to it as no runner at all. So D5
also decides how that check reads the pinned site:

- Chosen: the check compares each entry of the `engine` site's `tool` input, split on commas
  and stripped, with the one exact string the pin record `ENGINE_NEXTEST_PIN` holds,
  `cargo-nextest` at the version D4 pins and D1's check holds, because the check then reads the
  pinned string itself and accepts exactly one string, as it did before, so no check widens.
- Reading only an entry's name before `@`, the version stripped: rejected because it admits
  `cargo-nextest` at any version where the check admitted one string, a widening of an
  existing check that only an owner-signed ruling could make.
- Leaving the `engine` site unpinned: rejected because it breaks D1's rule at that one site,
  which would then need an allow-list arm, and none exists.

D6, kill criteria:

- Chosen: none for a pin. A pin that blocks a security update for more than one release is
  re-pinned, never removed. A check that examined 0, or a tool that is missing or off its pin, is
  VOID, which blocks and is never a pass. A demotion of any of the three checks is a weakening,
  and goes to the owner as a signed ruling. A window, wherever one is needed, reads "the last N
  merged PRs", never a number of days. These hold because a refusal of the pin check is always a
  site without an exact version, so there is no false-positive rate to measure.
- A precision kill (fixes against new suppressions over the last 30 merged PRs): rejected
  because the checks admit no suppression to count.
- Removing a pin that blocks an update: rejected because it reopens the moving target; the
  re-pin is the same one-line edit.

D7, what this delivery leaves out:

- Chosen: action references, install routes other than this action (`cargo install`, `pip`,
  `curl`, setup actions, images), further CI lint rules and any allow-list arm stay out, each
  under SPEC-428 section 5 with its issue, because none of them is one of the three rules.
- Folding any of them in: rejected because each is a rule of its own that this delivery was not
  asked for.

D8, tests and rows:

- Chosen: one test per acceptance criterion, read red at the red commit and green at the cure,
  each structural over the tree's text, never a wall-clock assertion; two pushes, the red commit
  alone and then the cure, the rows and the record; rows S42800-S42828; no formal model, because
  the checks are functions of the tree's text and no new actor or shared state appears.
- One push: rejected because the red would never be read alone, so no test would be shown red
  for its criterion's reason.
- A timeout test that runs a job: rejected because it would measure a runner, not the tree.
- A TLA+ model: rejected because the installs write only their own runner's tool directory, no
  cache key holds it, and the two new timeouts add no actor and no write.
- No row on the engine check's coupling: rejected because A7 and the live engine test alone
  would hold it, and no row would pin the matcher's operand, the version strip it refuses or the
  pin record's value; S42826 to S42828 pin those three.

D9, the shape of the record:

- Chosen: one delivery under SPEC-428 and this ADR, with ADR-055 amended insert-only by a note,
  because ADR-055's Confirmation names zizmor's old operand, and no SPEC states the operand or a
  tool input.
- Editing ADR-055's Confirmation line in place: rejected because a decision record changes
  insert-only.
- Amending SPEC-002 or SPEC-038: rejected because neither states a tool input, a job's timeout or
  zizmor's operand; SPEC-191's sentence that the `warm` job installs the same tools by the same
  action stays true.

## Decision Outcome

Chosen: D1 to D9 above. Every `taiki-e/install-action` site names each tool at an exact version,
the eight that did not now name the version their own jobs installed; every job that runs steps
sets its own `timeout-minutes`, the two that did not now set 5 and 10; and zizmor reads
`.github`, so the Dependabot configuration is audited too. Seven tests in one class of
`scripts/tests/test_ci_workflows.py` hold the three rules, each printing its examined counts and
refusing 0, and 29 rows show that each check observes what it holds.

### Consequences

- Good, because a bump of the install action no longer changes a tool; a tool moves only by a
  reviewed edit of a `tool` input.
- Good, because no job can run to the platform's default limit unseen.
- Good, because the Dependabot configuration is audited on every pull request.
- Good, because no existing check is widened: the engine job's check accepts the one pinned
  string where it accepted the one bare name, so this delivery's weakening table is empty.
- Bad, because each tool is re-pinned by hand: Dependabot reads action references, never the
  `tool` input.
- Bad, because a pin that falls behind a security fix holds until someone re-pins it.
- Bad, because the `engine` site's `cargo-nextest` pin is held twice, at the site and in the
  pin record, so its re-pin edits both in one change, or the engine job's check reads red.

### Confirmation

- `scripts/tests/test_ci_workflows.py`: SPEC-428 A1 to A7, run in CI's `hygiene` job.
- The cure's `workflow-lint` log: `.github/dependabot.yml` among the completed inputs, and no
  finding.
- Each install job's log at the cure: the installed version equals its pin.
- The rows S42800-S42828, run by CI's `mutation-rows` job.

## What would make this wrong

- The install action changes how it splits `tool`. The matcher would then read entries the
  action does not install; the site count would still agree with the text, so the entry count is
  the signal to re-read.
- zizmor stops collecting the Dependabot configuration from a directory input. The test's model
  would still count it in, so the completion line in CI's log is the guard.
- A tool stops publishing a build the action can fetch at its pinned version. The install then
  fails by name, which is the pin working; the cure is a re-pin.
- GitHub lets a call job set its own timeout. The call jobs would then be checked too.

## More Information

SPEC-428; SPEC-002 R10 (actions pinned by full commit sha); ADR-055 and its note (the
`workflow-lint` job); SPEC-038 R14 (the engine job's check); SPEC-191 (the warm cache job);
SPEC-340 (the server audit job).

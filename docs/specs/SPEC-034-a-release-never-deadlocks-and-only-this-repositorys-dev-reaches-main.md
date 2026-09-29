# SPEC-034: a release never deadlocks, only this repository's dev reaches main, and only GitHub Actions satisfies a required check

- **Wave:** W0. **Issue:** #23 (epic #1). **Context(s):** `repo` (`.github/`, `RELEASING.md`, the
  contributor and agent documents).
- **Decided by:**
  - ADR-034, this SPEC's own: main does not require an up-to-date head, nothing is merged back,
    and a hotfix goes through dev.
  - ADR-035, this SPEC's own: the required checks are pinned to GitHub Actions, and a fork never
    reaches main.
  - ADR-017 (the branch model) and ADR-033 (the committed rulesets are the enforced ones).
- **Status:** judged: delivered with its tests before the first release is tagged.

## 1. The problem, measured

The maintainer's review after the repository went public found three defects in the enforced
rulesets and the `ci` workflow (`gh api repos/RexRenatus/deck-streak/rulesets/<id>`, and
`.github/workflows/ci.yml` at `dev` c1f53c1).

- **The second release deadlocks.**
  - `main`'s ruleset requires an up-to-date head (`strict_required_status_checks_policy: true`),
    and both branches take pull requests only.
  - After the first release merge, `main` holds a merge commit that `dev` lacks, so the next
    release pull request from `dev` is out of date and strict blocks it.
  - Bringing `dev` up to date needs a pull request from `main` into `dev`. It changes no file, so
    `fragment` refuses it (`.github/workflows/changelog.yml`). Once `dev` has moved on, `dev`'s own
    strict rule blocks it too, because `main` cannot contain `dev`'s newer commits.
  - GitHub's "update branch" on the release pull request would merge `main` into `dev` without a
    pull request, which `dev`'s ruleset exists to refuse.
- **A fork's branch named `dev` passes `base-is-dev`.** The job compares `github.head_ref` with
  `dev` and reads nothing about the head's repository. The fork approval policy
  (`all_external_contributors`, set when the repository went public) holds workflow runs from forks
  for a maintainer's approval. That is a human step, not a check.
- **Any source can satisfy a required check.** Both branch rulesets list `ci` and `fragment` with
  `integration_id` unset, which means any app or any commit status with that context name. Measured
  on `dev`'s check runs: every `ci`, `fragment` and `base-is-dev` run carries `app.id` 15368
  (`github-actions`).

The fork rule the maintainer set at the flip also lives only in a message: agents never approve a
workflow run from a fork, and never merge a pull request whose head repository is not
`RexRenatus/deck-streak`.

## 2. Requirements

R1. `main`'s ruleset does not require an up-to-date head (`strict_required_status_checks_policy:
    false`). `dev`'s ruleset does (`true`).
    - `base-is-dev` admits one head into `main`: this repository's `dev`. GitHub keeps one open pull
      request per head and base, so `main` cannot move between a release pull request's checks and
      its merge.
    - Those checks run on the pull request's merge ref, which already contains `main`.
R2. Nothing is merged from `main` into `dev`. A hotfix is a pull request into `dev`, followed by a
    patch release. `RELEASING.md` has no back-merge step and says so (ADR-034).
R3. Every required status check in `.github/rulesets/main.json` and `dev.json` names
    `integration_id` 15368, the GitHub Actions app.
R4. `base-is-dev` fails a pull request into `main` unless its head branch is `dev` and its head
    repository is this repository (`github.event.pull_request.head.repo.full_name` equals
    `github.repository`). It reads both through `env`, never by expanding an expression inside the
    script.
R5. `AGENTS.md`, `CLAUDE.md` and `CONTRIBUTING.md` each state the fork rule. The maintainer re-lands
    an accepted outside contribution from a branch of this repository.
R6. After the first release merge, and before its tag, a probe release pull request from `dev` into
    `main` is opened while `dev` lacks `main`'s release merge commit.
    - Its merge state must not be `BEHIND`.
    - GitHub's update-branch is tried on it, and the result is recorded in `RELEASING.md`.
    - The probe is closed unmerged. It is an operational proof, recorded on the release pull request.
R7. No workflow reads a secret other than `GITHUB_TOKEN`, and none checks out or fetches another
    repository (ADR-017, ADR-056). One checker in `WorkflowsAreHardened` judges a directory of
    workflows, so the same code judges `.github/workflows/` and the planted workflows under
    `scripts/tests/fixtures/secrets-and-checkouts/`. A workflow is a `.yml` or `.yaml` file, as
    GitHub reads both, and the hardening tests beside the checker (a read-only token, SHA pins,
    hosted runners and no privileged trigger) read both too. The hardening tests read keys the
    way the checker does: each reads a workflow through the checker's reader, and a form the
    reader does not read fails the test. The checker refuses each of these by name, naming the
    file, the place in it and the secret, repository or command:
    - a read of any secret but `GITHUB_TOKEN`, in any expression form: `secrets.NAME` in any
      spacing or case, `secrets['NAME']`, a secret inside a larger expression, and the secrets
      context whole (`toJSON(secrets)`, or a secret named at run time);
    - `secrets: inherit` on a job;
    - an `actions/checkout`, its `uses` and its inputs read as the runner reads them, names in any
      case, whose `repository` is another repository or whose `github-server-url` is another
      server. An omitted or empty `repository`, `${{ github.repository }}` and this repository's
      own name are this repository when `github-server-url` is unset: omitted, empty or
      `${{ github.server_url }}`;
    - an `actions/checkout` whose inputs are not a mapping, such as a `with` that is one `${{ }}`
      expression, which GitHub evaluates when the step runs. An omitted or empty `with` is no
      inputs;
    - a `run` step that clones a repository (`git clone`, `gh repo clone`) or points git at a URL
      (a `git fetch` of a URL, or of git's scp-like `user@host:path` or `host:path` whose host is
      a dotted name), and the same command in any other string the workflow holds, such as a
      `BASH_ENV` that bash expands before it runs a step's script;
    - a `shell` that is not one of GitHub's built-in keywords as written (`bash`, `sh`, `pwsh`,
      `powershell`, `python`, `cmd`), at a step, a job's `defaults.run` or the workflow's: the
      runner runs any other as a command template, a command the checker does not read. A
      `defaults` or a `defaults.run` that is not a mapping, such as one `${{ }}` expression, is a
      shell the checker cannot read. An omitted or empty `shell` is none;
    - git configured from the environment: a key or a string, anywhere in the workflow, that names
      a variable whose name begins with `GIT_`, in any case, as git reads its configuration and the
      commands it runs from such variables (`GIT_CONFIG_COUNT`, `GIT_CONFIG_KEY_<n>`,
      `GIT_CONFIG_VALUE_<n>`, `GIT_CONFIG_PARAMETERS` and `GIT_SSH_COMMAND` among them); and an
      environment the checker cannot read: an `env` of the workflow, a job, a job's container or
      a step that is set and is not a mapping, such as one `${{ }}` expression, a job's
      `container` that is one, and a job's container whose `options`, `image`, `ports` or
      `volumes` is or holds a `${{ }}` expression, as the runner creates the container from each.
      A container's registry `credentials` are not read by its steps;
    - a character or a form the workflow reader does not read, named by its line: the reader fails
      closed on characters outside printable ASCII, quoting, escapes, anchors, aliases and tags
      (section 3).
    The checker judges every step of a job, a step inside a `parallel` block at any depth
    included, as GitHub's workflow schema defines a job's steps.
    `GITHUB_TOKEN` is admitted in either form and any case, because GitHub reads a secret's name
    without case. The checker reports how many workflow files, expressions, checkouts and run steps
    it examined, and a directory with no workflow file is VOID, never a pass.

## 3. Acceptance criteria

| id | criterion | decided by |
|---|---|---|
| A1 | main does not require an up-to-date head | `test_rulesets.py` |
| A2 | dev requires an up-to-date head | `test_rulesets.py` |
| A3 | every required check comes from GitHub Actions | `test_rulesets.py` |
| A4 | the release runbook merges nothing back into dev | `test_rulesets.py` |
| A5 | base-is-dev refuses a fork whose branch is named dev | `test_ci_workflows.py` |
| A6 | base-is-dev admits this repository's dev into main | `test_ci_workflows.py` |
| A7 | base-is-dev refuses this repository's other branches into main | `test_ci_workflows.py` |
| A8 | every agent document states the fork rule | `test_fork_rule.py` |
| A9 | no workflow reads a secret or checks out another repository | `test_ci_workflows.py` |
| A10 | each planted secret or foreign repository is refused by name | `test_ci_workflows.py` |
| A11 | this repository's token and checkout are admitted | `test_ci_workflows.py` |
| A12 | an empty workflow directory is refused | `test_ci_workflows.py` |
| A13 | a .yaml workflow is held to the same hardening rules, an action is pinned only in its plain form, and the hardening tests read keys the way the checker does | `test_ci_workflows.py` |

```acceptance
A1: python3 -m unittest discover -s scripts/tests -p test_rulesets.py -k main_does_not_require_an_up_to_date_head
A2: python3 -m unittest discover -s scripts/tests -p test_rulesets.py -k dev_requires_an_up_to_date_head
A3: python3 -m unittest discover -s scripts/tests -p test_rulesets.py -k every_required_check_comes_from_github_actions
A4: python3 -m unittest discover -s scripts/tests -p test_rulesets.py -k the_release_runbook_merges_nothing_back_into_dev
A5: python3 -m unittest discover -s scripts/tests -p test_ci_workflows.py -k base_is_dev_refuses_a_fork_whose_branch_is_named_dev
A6: python3 -m unittest discover -s scripts/tests -p test_ci_workflows.py -k base_is_dev_admits_this_repositorys_dev_into_main
A7: python3 -m unittest discover -s scripts/tests -p test_ci_workflows.py -k base_is_dev_refuses_this_repositorys_other_branches_into_main
A8: python3 -m unittest discover -s scripts/tests -p test_fork_rule.py -k every_agent_document_states_the_fork_rule
A9: python3 -m unittest discover -s scripts/tests -p test_ci_workflows.py -k no_workflow_reads_a_secret_or_checks_out_another_repository
A10: python3 -m unittest discover -s scripts/tests -p test_ci_workflows.py -k each_planted_secret_or_foreign_repository_is_refused_by_name
A11: python3 -m unittest discover -s scripts/tests -p test_ci_workflows.py -k this_repositorys_token_and_checkout_are_admitted
A12: python3 -m unittest discover -s scripts/tests -p test_ci_workflows.py -k an_empty_workflow_directory_is_refused
A13: python3 -m unittest discover -s scripts/tests -p test_ci_workflows.py -k a_yaml_workflow_is_held_to_the_same_hardening_rules
```

A5 to A7 run `base-is-dev`'s own script, as extracted from `ci.yml`, under `bash -e`.
- Each of its `env` expressions is given a scenario's value.
- A step that reads an expression the scenarios do not model fails the test, rather than running
  with an empty value.

A9 to A12 run one checker, `secret_and_checkout_problems`, over a directory: A9 over
`.github/workflows/`, A10 and A11 over the planted workflows in
`scripts/tests/fixtures/secrets-and-checkouts/refused/` and `admitted/`, and A12 over a directory
that holds no workflow. A10 also writes planted workflows to a scratch directory at test time: each
character the reader refuses, from a Python escape so that no committed file holds one, and lines
the reader cannot place, each of which refuses its whole file. Among them are `|` blocks that each
hold a line of one such character: the block's end and its indent read only a space or a tab as
white space, so a secret the block names over two lines keeps its indent.
- It reads each workflow with the test file's own reader (`read_workflow`), chosen over a YAML
  library the repository does not depend on: the guard tests run on the standard library alone,
  and a library would be the repository's first Python dependency, with a pin, an install step in
  CI and an ADR of its own.
- The reader fails closed. It ends a line only at a line feed or a carriage return, reads a space
  or a tab as white space and nothing else, as YAML does, and reads a quote doubled inside single
  quotes as one quote. It refuses by its line, rather than guess at, a line that holds a character
  other than a tab or printable ASCII, a double-quoted value that holds an escape, a quoted value
  that does not end at its closing quote, an anchor, alias or tag, a flow mapping, a flow list
  whose items are not plain, and a key that is not a plain name. The checker names each refusal as
  a finding and judges the rest of the file as read. A line the reader cannot place refuses the
  whole file at once.
- It delimits each `${{ }}` expression as GitHub does: a `}}` inside a quoted string does not close
  one.
- It reads each expression's text and errs toward refusing: any mention of the secrets context that
  does not name `GITHUB_TOKEN` is refused, a mention inside a quoted string included. A property
  named `secrets` of another context, such as a step's output, and a longer word that begins with
  it, such as `secrets-scan.toml`, are not the secrets context.
- It reads a checkout's `uses` and its inputs as the runner reads them, the owner, the name and each
  input's name in any case. A checkout whose inputs are not a mapping is refused: GitHub evaluates a
  `with` that is one `${{ }}` expression when the step runs, so no reading of the file names the
  repository it checks out. An omitted or empty `with` is no inputs. A job or a step that is not a
  mapping is not judged: the reader has named its line, or GitHub refuses the workflow.
- It judges every step of a job, a step inside a `parallel` block at any depth included: GitHub's
  workflow schema reads a `parallel` step's list as steps, and one of them may be another `parallel`
  step.
- It reads a shell as the runner does. A built-in keyword, as written, runs the script with the
  keyword's own arguments; the runner runs any other `shell` as a command, its first word the
  command and `{0}` the script's path, so the checker refuses it by its place: a step's `shell`,
  a job's `defaults.run.shell` and the workflow's. A `defaults` or a `defaults.run` that is not a
  mapping is refused too, as GitHub evaluates one `${{ }}` expression when the job runs. An
  omitted or empty `shell` is none: the runner falls back to the defaults, judged where they are.
- It reads every string the workflow holds for the commands it refuses in a `run` step's script,
  as a value a shell runs holds a command as a script does: bash expands `BASH_ENV`, running each
  command it substitutes, before a step's script. A string that only names such a command, such
  as a step's name, is refused too, which errs toward refusing.
- It refuses git configured from the environment: a key or a string that names a variable whose
  name begins with `GIT_`, in any case, wherever it sits, as an `env` key, in a container's
  options or in a script that exports one. It refuses an environment it cannot read: an `env` of
  the workflow, a job, a job's container or a step that is not a mapping, a job's `container`
  that is one `${{ }}` expression, and a job's container whose `options`, `image`, `ports` or
  `volumes` is or holds one, since the steps of a job with a container run in its environment and
  the runner creates the container from each of those. A name that holds those letters only
  inside a longer word, such as `DIGIT_COUNT`, names no such variable, a container named by its
  image alone has no `env` to read, and a container's registry `credentials`, read to pull its image, are
  not read by its steps.

A13 runs the three hardening tests, each through its own setUp, over
`scripts/tests/fixtures/workflow-hardening/`, a hardened `.yml` control beside an unhardened
`.yaml` workflow, and each refuses the `.yaml` one by its name. It then runs the SHA-pin test over
the control with its action written in forms other than its plain one, and with its SHA cut short,
and the test refuses each: an action is pinned only as its owner, its name and any path, each joined
by a single slash, then a full commit SHA. The tests and the checker take their files from one
function, `workflow_files`, which reads both extensions.
- The hardening tests read keys the way the checker does. Each reads a workflow through
  `read_workflow`, the checker's reader, and a form the reader does not read, or a line it cannot
  place, fails the test by the file's name. The token test reads the default permissions as the
  reader reads them, the SHA-pin test every `uses` the reader reads, and the runner test every
  `runs-on`. The last two read through one walk, `entries`, which collects every value a key holds
  wherever it sits, a value under the same key nested in it included.
- A13 plants keys in the control, one line at a time, and judges it beside the live workflows:
  keys the reader reads, a trigger, and forms the reader does not read. The test that judges each
  plant refuses it by the file's name.

## 4. File manifest

| file | context | change |
|---|---|---|
| `.github/rulesets/main.json` | `repo` | changed: strict off; checks pinned to 15368 |
| `.github/rulesets/dev.json` | `repo` | changed: checks pinned to 15368 |
| `.github/workflows/ci.yml` | `repo` | changed: `base-is-dev` reads the head repository |
| `scripts/tests/test_rulesets.py` | `repo` | changed: A1 to A4; SPEC-033's A9 no longer asserts main's strictness |
| `scripts/tests/test_ci_workflows.py` | `repo` | changed: A5 to A7 |
| `scripts/tests/test_fork_rule.py` | `repo` | added: A8 |
| `RELEASING.md` | `repo` | changed: the release is a pull request into dev, then a release pull request; no back-merge; hotfixes through dev |
| `AGENTS.md`, `CLAUDE.md`, `CONTRIBUTING.md` | `repo` | changed: the fork rule |
| `docs/decisions/ADR-034-main-is-never-merged-back-and-a-hotfix-goes-through-dev.md` | `repo` | added |
| `docs/decisions/ADR-035-required-checks-come-from-actions-and-forks-never-reach-main.md` | `repo` | added |
| `docs/decisions/ADR-017-branch-model-and-hosted-ci.md` | `repo` | changed: a note that ADR-034 supersedes part of its outcome |
| `docs/specs/SPEC-034-a-release-never-deadlocks-and-only-this-repositorys-dev-reaches-main.md` | `repo` | added |
| `docs/red-first/SPEC-034.md` | `repo` | added |
| `changelog.d/fix-release-flow-and-forks.md` | `repo` | added |
| `scripts/tests/test_ci_workflows.py` | `repo` | changed by the amendment (section 7): A9 to A13, the checker they run, its workflow reader, `workflow_files`, the SHA-pin pattern and the hardening tests' reading of each workflow |
| `scripts/tests/fixtures/secrets-and-checkouts/refused/` | `repo` | added by the amendment: the twenty planted workflows the checker refuses, `another-repository-in-other-forms.yml`, `checkout-of-another-repository.yml`, `checkout-whose-inputs-are-one-expression.yml`, `clone-in-a-custom-shell.yml`, `clone-of-another-repository.yml`, `clone-outside-a-run-step.yml`, `environment-the-checker-does-not-read.yml`, `every-secret.yml`, `fetch-of-a-url.yml`, `git-configured-from-the-environment.yml`, `key-the-reader-refuses.yml`, `run-by-alias.yml`, `second-of-each.yml`, `secret-in-a-form-the-reader-refuses.yml`, `secret-in-a-larger-expression.yml`, `secret-in-a-quoted-value.yml`, `secret-in-any-spacing.yml`, `secret-in-brackets.yml`, `secrets-inherited.yaml` and `steps-in-a-parallel-block.yml` |
| `scripts/tests/fixtures/secrets-and-checkouts/admitted/` | `repo` | added by the amendment: the eight planted workflows it admits, `built-in-shells.yml`, `checkout-of-this-repository.yml`, `environment-the-checker-reads.yml`, `github-token.yml`, `not-the-secrets-context.yml`, `quoted-values.yml`, `refspec-of-this-repository.yml` and `secrets-github-token.yml` |
| `scripts/tests/fixtures/workflow-hardening/` | `repo` | added by the amendment: A13's planted workflows, `hardened.yml` and `unhardened.yaml` |
| `docs/specs/SPEC-034-a-release-never-deadlocks-and-only-this-repositorys-dev-reaches-main.md` | `repo` | changed by the amendment: the insertions section 7 lists |
| `docs/red-first/SPEC-034.md` | `repo` | changed by the amendment: A9 to A13 |
| `changelog.d/test-ci-no-secrets-216.md` | `repo` | added by the amendment |

## 5. What this does NOT do

- It builds no release workflow: the tag-triggered build and its draft release are the first
  deploy's work (#42).
- It does not tag v0.1.0 or merge the first release. That is the release pull request's work, and
  R6's probe runs between the two (#23).
- It gives no bypass to anyone, including the owner's account: every ruleset keeps an empty bypass
  list (#160).
- It admits no fork contribution automatically. The maintainer re-lands accepted outside work from
  an internal branch (#160).
- The amendment changes no workflow: the workflows already comply, and A9 keeps them so (#216).

## 6. Risks

- **Two release pull requests at once.** GitHub refuses a second open pull request with the same
  head and base, and `base-is-dev` refuses any other head. A5 to A7 hold the second half.
- **A strict-off `main` merges a stale head.** Only `dev` reaches `main`, and a pull request's
  checks run on its merge ref, so what merges is what was tested. R6's probe measures it.
- **The integration id changes.** It is GitHub's own app, and A3 pins it. A check from any other
  source is refused, which is the intent. A change would show as every pull request blocked on
  `ci`.
- **A form of secret read the checker does not know.** It errs toward refusing: an expression that
  mentions the secrets context without naming `GITHUB_TOKEN` is refused whatever its form, and the
  workflow reader refuses by its line a character or a value it does not read as YAML does, a
  character outside printable ASCII, a quoted escape, an anchor or an alias among them, rather than
  guess at it. A new form therefore fails A9 rather than passing it.

## 7. Amendment, 2026-09-28: no workflow reads a secret or checks out another repository

Made on issue #216, which ADR-056's Confirmation cites, insert-only under ruling (i) of SPEC-038
section 8: every earlier byte is kept in order. It inserts:

- section 2: R7, after R6;
- section 3: rows A9 to A13 of the criteria table, lines A9 to A13 of the acceptance fence, the
  paragraph that begins "A9 to A12", and the one that begins "A13";
- section 4: the seven rows marked "by the amendment";
- section 5: the bullet that begins "The amendment";
- section 6: the risk "A form of secret read the checker does not know";
- this section.

ADR-017 allows no secret in a pull-request workflow, and ADR-056 keeps public CI from fetching or
building a private runner. Both held by review: `test_ci_workflows.py` judged the token's
permissions, the SHA pins, the runners, `pull_request_target`, the gate's stages, the aggregate job,
the triggers and `base-is-dev`, and no test judged either property. At `dev` c3d769b the four
workflows read no secret (their one token is `github.token`, which the survivors job of
`mutation-weekly.yml` reads), pass none to another workflow, and check out only this repository,
17 times, none with `repository`. A9 holds them there, and the planted workflows prove the checker
red (A10, A12 and A13).

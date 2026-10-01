# SPEC-190: every workflow with a pull-request trigger cancels the run a newer push supersedes

- **Wave:** W4. **Issue:** #369. **Context(s):** `repo` (`.github/workflows/`, its tests and `docs/`).
- **Decided by:** ADR-190 (this SPEC's own: one concurrency rule for every workflow, and what it
  was chosen against), ADR-055 (the rule for `ci.yml` and why GitHub's one pending run per group
  shapes it) and SPEC-038 R8 (the pull-request concurrency rule; it takes an insert-only amendment).
- **Status:** delivered. It waited in `docs/specs/planned/` from its own commit until its tests
  were green, and the delivery moved it to `docs/specs/` (ADR-016). It holds
  `docs/red-first/SPEC-190.md`.

## 1. The problem, measured

- **`ci.yml` cancels a superseded pull-request run and two other workflows do not.** `ci.yml` groups
  a pull request's runs by its ref with `cancel-in-progress` true for a pull request (SPEC-038 R8).
  `changelog.yml` sets no concurrency block, so every superseded run finishes. `engine-measure.yml`
  groups by ref with `cancel-in-progress: false`, so a superseded run finishes and the next one
  waits behind it. Each holds a job slot for a result nobody reads (#369).
- **Source:** `grep -n -A3 '^concurrency:' .github/workflows/*.yml` lists the four blocks that exist
  (`ci.yml`, `engine-measure.yml`, `mutation-weekly.yml`, `release.yml`), and `changelog.yml` has none.
- **The blocks that already follow the rule are `ci.yml` and `mutation-weekly.yml`,** whose group is
  `<workflow>-${{ github.event_name == 'pull_request' && github.ref || github.run_id }}`. Nothing
  holds a new or changed workflow to it.

## 2. Requirements

R1. **Every workflow with a `pull_request` trigger carries `ci.yml`'s workflow-level block,** with
the workflow's own name as the group's prefix: `group: <workflow name>-${{ github.event_name ==
'pull_request' && github.ref || github.run_id }}` and `cancel-in-progress: ${{ github.event_name ==
'pull_request' }}`. No job sets a group of its own (SPEC-038 R8).

R2. **`changelog.yml` gains the block.** It had none.

R3. **`engine-measure.yml`'s block becomes the rule's.** Its group was `engine-measure-${{ github.ref }}`
and its `cancel-in-progress` was `false`.

R4. **A run that is not a pull request's is never cancelled.** For every workflow with a
`pull_request` trigger, two push, tag, schedule or dispatch runs have two different groups (the run id arm), and
`cancel-in-progress` is false for each.

R5. **`release.yml` keeps `cancel-in-progress: false`.** It runs on a push of a tag only, so it has
no pull-request trigger and the rule does not apply; it never cancels a run in progress. Its group
stays its tag's ref, so GitHub's one pending run per group still applies: a third run of one tag
replaces a second that is waiting, as before this SPEC (#377).

R6. **`ci.yml`'s and `mutation-weekly.yml`'s blocks are unchanged,** byte for byte.

R7. **A test holds every workflow file in the directory to R1, R4, R5, R8 and R9,** present and
future, and prints how many workflows it examined.

R8. **No two workflows share a pull request's group.** GitHub reads a group's name without case and
across every workflow, so two workflows of one name would cancel each other's newest run.

R9. **No workflow file cancels a push, tag, schedule or dispatch run in progress, and no job sets a
concurrency block,** whether or not the workflow has a pull-request trigger.

## 3. Acceptance criteria

| id | criterion | decided by |
|---|---|---|
| A1 | every workflow with a `pull_request` trigger carries the block of R1, with its own name as the prefix, and sets no job-level group; in the scenarios a newer run of a pull request cancels the one it supersedes, two pull requests do not share a group, and a pull request never shares a push's group | `test_workflow_concurrency.py` `every_workflow_with_a_pull_request_trigger_follows_the_rule` |
| A2 | for every workflow with a `pull_request` trigger, two runs of a push, a tag, a schedule or a dispatch have different groups, and none is cancelled | `test_workflow_concurrency.py` `a_run_that_is_not_a_pull_requests_is_unique_and_never_cancelled` |
| A3 | `release.yml` has no pull-request trigger and its `cancel-in-progress` is false | `test_workflow_concurrency.py` `the_release_workflow_never_cancels` |
| A4 | `ci.yml`'s and `mutation-weekly.yml`'s blocks are the rule's, unchanged | `test_workflow_concurrency.py` `the_blocks_that_already_followed_the_rule_are_unchanged` |
| A5 | no two workflows with a `pull_request` trigger render one group for a pull request, read without case | `test_workflow_concurrency.py` `no_two_workflows_share_a_pull_requests_group` |
| A6 | no workflow file cancels a push, tag, schedule or dispatch run, and no job sets a concurrency block | `test_workflow_concurrency.py` `no_workflow_cancels_a_run_that_is_not_a_pull_requests` |

```acceptance
A1: python3 -m unittest discover -s scripts/tests -p test_workflow_concurrency.py -k every_workflow_with_a_pull_request_trigger_follows_the_rule
A2: python3 -m unittest discover -s scripts/tests -p test_workflow_concurrency.py -k a_run_that_is_not_a_pull_requests_is_unique_and_never_cancelled
A3: python3 -m unittest discover -s scripts/tests -p test_workflow_concurrency.py -k the_release_workflow_never_cancels
A4: python3 -m unittest discover -s scripts/tests -p test_workflow_concurrency.py -k the_blocks_that_already_followed_the_rule_are_unchanged
A5: python3 -m unittest discover -s scripts/tests -p test_workflow_concurrency.py -k no_two_workflows_share_a_pull_requests_group
A6: python3 -m unittest discover -s scripts/tests -p test_workflow_concurrency.py -k no_workflow_cancels_a_run_that_is_not_a_pull_requests
```

The test reads each workflow with the reader `test_ci_workflows.py` uses and renders the group and
the condition in scenarios (a pull request, pushes to dev and main, a tag, a schedule, a dispatch)
with that file's evaluator. Each test prints how many workflows it examined and refuses zero. The
set of checks a pull request's newest run reports is the same before and after: `ci`, `changelog`'s
`fragment`, `engine-measure`'s job and `mutation-weekly`'s `rehearsal` keep their names, so only a
superseded run differs.

## 4. File manifest

| file | context | change |
|---|---|---|
| `.github/workflows/changelog.yml` | repo | changed: R2, the block |
| `.github/workflows/engine-measure.yml` | repo | changed: R3, the block |
| `scripts/tests/test_workflow_concurrency.py` | repo | added: A1 to A6 |
| `scripts/mutation-rows.d/S19000-S19099.json` | repo | added: four hand-proved rows, the fourth (S19004) turning `release.yml`'s `cancel-in-progress: false` to `true`, which the release test alone kills |
| `docs/specs/SPEC-190-every-workflow-cancels-a-superseded-pull-request-run.md` | repo | added, from `docs/specs/planned/` |
| `docs/decisions/ADR-190-one-concurrency-rule-for-every-workflow-with-a-pull-request-trigger.md` | repo | added |
| `docs/specs/SPEC-038-ci-runs-the-gate-in-parallel-jobs-and-only-dev-and-main-save-a-cache.md` | repo | changed: an insert-only amendment at its end |
| `docs/red-first/SPEC-190.md` | repo | added |
| `changelog.d/ci-concurrency-190.md` | repo | added |

No schematic: the change adds no component; the rule is `ci.yml`'s, applied to two more files.

## 5. What this does NOT do

- It does not change the strict up-to-date rule on `dev`, because that is a ruleset and not a
  workflow (#369).
- It does not change `mutation-weekly.yml`'s block, which already follows the rule (#369).
- It changes no Rust and no Python outside the new test, because the defect is in two workflows'
  concurrency blocks (#369).
- It does not cancel a push, tag, schedule or dispatch run, because a land's full run must finish
  (ADR-055, #369).
- It does not change `release.yml`'s group, its tag's ref, so a third run of one tag still replaces
  a waiting second under GitHub's one pending run per group (#377).
- It changes no check's name and no job, so the set of checks a pull request's newest run reports
  is the one it reported before (#369).

## 6. Risks

- **GitHub keeps one pending run per group.** A pull request's group is its ref, so a third push
  cancels a pending second one. That is the intent for a pull request; a push has a group of its
  own, so it is never affected (ADR-055).
- **A path-filtered workflow on a pull request.** `engine-measure` runs only when its paths change;
  after a newer push that leaves the pull request no longer changing them (GitHub compares a pull
  request's three-dot diff), no run starts, so the superseded run is not cancelled and finishes.
  That is today's behaviour for `mutation-weekly.yml` too (`ci.yml` has no path filter) and is left
  as it is.

## 7. References

Issue #369; SPEC-038 R8; ADR-055; ADR-016; ADR-190.

## 8. Amendment of 2026-09-30 (ADR-292, #377)

This amendment is insert-only. It closes the one case R5 and section 5 name: a third run of one tag
no longer replaces a waiting second. R5's other statements stand: `release.yml` has no pull-request
trigger, never cancels a run in progress and keeps its tag's ref as its group.

R10. **Every workflow whose concurrency group can hold two runs of one release queues them.** A
workflow that runs for a tag is one with a `release` or a `create` trigger, or a `push` trigger whose
filters admit a tag: a `tags` or `tags-ignore` filter, or neither a branch nor a tag filter, since
GitHub then runs it for tags too (`create` takes no filter and runs for every tag created). It carries
one workflow-level block and no job's own, under a key of any case. Its block holds only the keys
GitHub's workflow parser defines (`group`, `cancel-in-progress`, `queue`), spelt with their case, and
no mapping of the workflow holds a key twice read without case, since the parser refuses the whole
workflow for either. Its group is present, is the same for two runs of one tag under each event that
runs it for a tag, and is no other workflow's, read without case (R8): another workflow's group is
rendered with its own name as `github.workflow`, in a run for a tag of each event it declares whose
ref can be a tag (`push`, `create`, `release`, `workflow_dispatch`, `registry_package`, `deployment`,
`deployment_status`). A `workflow_dispatch`, `registry_package` or `deployment` run holds a tag's ref
only when a person or a package picks it, so such a workflow is not in the class, and its group is
still held to be no release's. Its `cancel-in-progress` is false for every such run, and it sets `queue: max`, so a
run waits behind the one running and none is replaced (ADR-292). GitHub keeps at most a hundred waiting
runs in one group and cancels any run beyond them, which lists as a cancelled run and is not lost
unseen (ADR-292). The test derives the
workflows from the directory and prints how many it examined (today one, `release.yml`); a workflow it
cannot read, or a group it cannot render, is refused.

## 9. Acceptance criteria of the 2026-09-30 amendment

| id | criterion | decided by |
|---|---|---|
| A7 | every workflow that runs for a tag (R10) has one workflow-level block and no job's own, a group that is present, one for two runs of one tag under each event that runs it for a tag and no other workflow's, cancels no run and sets `queue: max`; a tag group with no queue, `queue: single`, a group keyed by the run id, `cancel-in-progress` true, a job-level block (under a quoted key too), a missing block, a block with no group, a group another workflow renders in another case, a release event's run-id group or cancel, and an unreadable workflow are each refused; a push with a `tags-ignore` filter, a paths filter only or no filter, a push named in a list and a release are each in the class, and a push of branches only is not | `test_workflow_concurrency.py` `every_workflow_that_can_hold_two_runs_of_a_release_queues_them` |
| A8 | the class of R10 is read as GitHub reads it, by a population planted on a workflow of the test's own, never on the live file: a push, a create and a release, each as a name, a list and a mapping, are in the class and refused with the default queue, and a dispatch is not in it; another workflow whose group names the release's through `github.workflow` or through its event is refused under each event whose ref can be a tag, and one of another name is not; each block key in another case or misspelt is refused; each key held twice, without case, is unread; and a job's key of any case is a block of its own | `test_workflow_concurrency.py` `the_release_class_is_read_as_github_reads_it` |

```acceptance
A7: python3 -m unittest discover -s scripts/tests -p test_workflow_concurrency.py -k every_workflow_that_can_hold_two_runs_of_a_release_queues_them
A8: python3 -m unittest discover -s scripts/tests -p test_workflow_concurrency.py -k the_release_class_is_read_as_github_reads_it
```

### File manifest of the amendment

| file | context | change |
|---|---|---|
| `.github/workflows/release.yml` | repo | changed: R10, `queue: max` |
| `scripts/tests/test_workflow_concurrency.py` | repo | changed: A7, its planted shapes and its planted triggers; A8 |
| `scripts/tests/test_ci_workflows.py` | repo | changed: the reader refuses a key a mapping already holds, read without case (R10) |
| `scripts/mutation-rows.d/S19000-S19099.json` | repo | changed: S19005 to S19010 |
| `docs/decisions/ADR-292-a-release-tags-runs-never-replace-a-waiting-run.md` | repo | added |
| `docs/red-first/SPEC-190.md` | repo | changed: A7, A8 |
| `changelog.d/ci-release-queue-377.md` | repo | added |

## 10. Amendment of 2026-09-30 (round 3): the release class closed by construction (ADR-292, #377)

This amendment is insert-only and supersedes two sentences of section 8. R10's sentence that a
release workflow's group "is no other workflow's, read without case" and is checked by rendering
another workflow's group "with its own name as `github.workflow`", in a run of each event it
declares, is superseded by R11(4): sampling a rendering misses a workflow that a caller calls (a
called workflow reads its caller's context; the reusable-workflows page says
"the `github` context is always associated with the caller workflow."), a job's own block, a literal group for
another tag and a group with no text of its own. A7's clause "no other workflow's" is superseded by
A9 in the same way. Every other line of section 8 stands.

R11. **A release workflow is closed by construction.** For every workflow that runs for a tag (R10):
1. every key it holds at the root, under `on:`, in a `push` or a `release` filter and in a job is one
   that GitHub's workflow parser defines there, read with its case;
2. its group reads `github.ref` and nothing else, plus a leading `github.workflow` where no workflow
   can call it, so every run of one tag takes one group whatever its event or its tag value;
3. its `cancel-in-progress` is absent or the boolean `false` as GitHub's parser types it, never a
   quoted string and never an expression (R12);
4. no other concurrency block, a workflow's or a job's, under any key spelling and in any workflow
   file, starts with text that the release group's start can also be, read without case, and a block
   with no literal text of its own is refused, since it can render as any group.

Because the rule closes the class from the group's own text, the test never renders a sample of
another workflow's group, and a new workflow that could share a release's group is refused whatever
its name, its events or its caller, where every call is to a workflow file of this repository that
the rule reads; R12 refuses any other call. GitHub documents that group names are
"case insensitive", which is why part 4 reads without case. The test derives its population from
constants of the parser's keys, of the events that run for a tag and of the `github` context's
properties, and lists none of its members; it prints how many groups, cancellations, blocks, blocks
of text of their own and keys it examined.

Disclosed remainder. R12 (section 12) names what the rule does not read and the follow-up that
reads each part. A model of the tag queue, its interleavings of a running run, a waiting run and a
run beyond the queue, is owed once `covers` accepts a workflow file, which follow-up #467 tracks;
`covers` today takes only `.rs`, `.py` and `.sh`.

Eviction, as `release.yml`'s block configures it. That block sets `queue: max`, so the depth of its
group is a hundred and not the default's one: GitHub's workflow syntax page (read 2026-09-30) says
"Up to 100 jobs or workflow runs can be `pending` in the concurrency group." and that once the queue is
full, further runs are canceled. Without the key, the default keeps one pending run and a newer run
replaces it, and the replaced run lists as cancelled. Either way an evicted run is a visible cancelled
run, never a silent loss. `release.yml` runs on a push of a SemVer tag and nothing else, and
`.github/rulesets/release-tags.json` allows no deletion, no update and no non-fast-forward of a `v*` tag.

## 11. Acceptance criteria of the 2026-09-30 round 3 amendment

| id | criterion | decided by |
|---|---|---|
| A9 | the class is closed by construction (R11), by a population generated from constants and never listed: each key of the parser's schema misspelt, in another case or missing at the root, under `on:`, in a filter and in a job; a group reading any `github` property other than `github.ref`, in each place the group is read; a `cancel-in-progress` that is an expression; another workflow's block, a job's block, a block under a key of another case, a group with no literal text of its own and a group whose text starts as the release group's can, are each refused, and the live workflows are held to the same rule | `test_workflow_concurrency.py` `the_release_class_is_closed_by_construction` |

```acceptance
A9: python3 -m unittest discover -s scripts/tests -p test_workflow_concurrency.py -k the_release_class_is_closed_by_construction
```

### File manifest of the round 3 amendment

| file | context | change |
|---|---|---|
| `scripts/tests/test_workflow_concurrency.py` | repo | changed: R11, A9 and its generated population |
| `scripts/mutation-rows.d/S19000-S19099.json` | repo | changed: S19011 to S19014, one row per part of R11 |
| `docs/decisions/ADR-292-a-release-tags-runs-never-replace-a-waiting-run.md` | repo | changed: Decision Outcome and Consequences hold for R11 |
| `docs/red-first/SPEC-190.md` | repo | changed: A9 |
| `changelog.d/ci-release-queue-377.md` | repo | changed: the rule of R11 |

## 12. Amendment of 2026-09-30 (round 5): the release class read as GitHub parses it (ADR-292, #377)

This amendment adds R12, A10 and A11. It corrects three sentences of section 10 in place: R11(3),
the sentence on a new workflow's caller, and the disclosed remainder, which now points here. R11's
other parts stand; where R11 and R12 differ, R12 decides.

R12. **No run a tag can start is cancelled, or replaced while it waits below the queue's depth, by
any concurrency block of this repository.**

Membership. A release workflow is one a tag can start (a `push` whose filters admit a tag, a
`create` or a `release`, as R10 reads them), and every workflow such a workflow calls, at any depth.
Every run of a release workflow is a release run, whatever its event, a dispatch included. Every
other workflow is a reacher, and a person-dispatched run of a workflow no tag starts is not a release
run: a reacher's blocks are judged only by whether they can reach a release run's group. Two groups
can be equal when, read without case and with leading space dropped, the text every rendering of one
starts with is a start of the other's. A leading `github.workflow` renders the name of the workflow
whose run holds the block, which for a called workflow is its caller's (the reusable-workflows page:
"the `github` context is always associated with the caller workflow."). `github.ref`, `inputs`,
`vars` and every other expression can render as any text.

The rule, over every workflow file:
1. Reading. Each file is read as GitHub's parser reads it, with YAML 1.2's core schema: a quoted or
   block `false` is a string, `no` and `off` are strings, and `False` is the boolean false. The
   reader reads only these named forms, each as YAML 1.2.2 defines it:
   - a block mapping whose keys are plain names, bare or in matching quotes, that the core schema
     types as strings, and a block sequence of `- ` items;
   - a plain scalar on one line whose first character is not a YAML indicator, or is `-`, `?` or `:`
     before a character that is not white space (ns-plain-first);
   - a single- or double-quoted scalar on one line, a double-quoted one holding no escape;
   - a literal block scalar under the header `|` (its text keeps one final line feed) or `|-` (it
     keeps none), with nothing after the header;
   - a flow list on one line of such plain items, `[]` when it holds none, with at most one trailing
     comma;
   - blank lines, comment lines and a ` #` comment after a value; a line ends at a line feed, a
     carriage return just before one dropped with it, as GitHub's parser ends a line.
   Every other form is refused by its line, by a message that names it (SPEC-034 R7): a character
   other than printable ASCII and the tab, a byte-order mark and a carriage return that does not
   end a line included; a tab in any line's indentation, a comment or blank line outside a block
   scalar's text included; a tab after a sequence item's `-`; a key that is not such a name, and a plain value holding `: `, which YAML reads as a
   key; a key a mapping already holds, read without case; a value whose first character is any other
   YAML indicator (`,[]{}#&*!|>'"%@` or the backtick) or `-`, `?` or `:` before a space or the
   line's end, so a sequence item that is itself a sequence or an explicit key is refused; an empty
   sequence item; an anchor, alias or tag; a flow mapping; a flow list whose items are not plain, or
   with an empty entry other than one trailing comma; a quoted value that does not end at its
   closing quote, and a double-quoted value holding an escape; a block scalar header other than `|`
   or `|-`; a block scalar's line indented less than its first line of text, a blank line above that
   text indented more than it, and a blank line of it that holds a tab; and a line the reader cannot
   place, a `---` among them. GitHub's parser refuses a file YAML refuses ("The file is not valid
   YAML"), and a form YAML reads otherwise than as the text the reader would return is misread, so
   each is refused rather than read.
   The tests read a workflow file through one loader alone, `workflow_file_text` in
   `test_ci_workflows.py`: its bytes, decoded as strict UTF-8 and never as `utf-8-sig`, so a
   byte-order mark reaches the reader and is refused by its name. A census computed when the tests
   run proves it, default-deny: in every module that imports the loader's module at any depth, a
   re-export and a module the tests add included, and in `_support.py`, every call that can read a
   file, a stream or a process's output, every call whose callee cannot be named and every call of a
   function that reads what it is given is the loader's one definition, bound by its module and
   qualified name, or a read listed by its module, qualified name and text, with its count and its
   reason. Every site in the test directory that imports, runs code or reaches a namespace by a name
   held in data is listed the same way, and any other is refused (A12).
2. Schema. Every value a release workflow holds is of a type GitHub's workflow parser defines there,
   read with case: each key, each constant, and each mapping, sequence or scalar. This holds from its
   root through `on:` and every event's mapping, `permissions`, `defaults`, `env`, `concurrency` and
   each job, a call job included, down to the job's own keys. A job's `permissions` scopes are in the
   class; round 3 left them to #464. A `${{` with no closing `}}`, in any string of a release
   workflow, steps included, is refused.
3. Calls. Every call in every workflow that runs is `./.github/workflows/<file>`, with that file
   present and taking `workflow_call`, walked to its end with no cycle. Any other call, a remote one
   or this repository's own pinned by commit, is refused, since what it would run in the caller's
   context cannot be read.
4. Blocks. A release workflow holds no job-level block. Every workflow-level block a release run
   holds, its own or a callee's, is read in that run's context. Its group starts with text of its own
   and then reads `github.ref` (or `github['ref']`) and nothing else. Its `cancel-in-progress` is
   absent or the boolean false. It sets `queue: max`, a callee's block included, since the docs do not
   say how a called workflow's group queues within its caller's run.
5. Collisions. No other block, a reacher's, a job's, or a callee's in another caller's run, starts
   with text a release block's start can be, read without case.

Advisory over-refusals. The rule refuses some shapes GitHub runs with a tag's runs kept:
- a reacher's block that can equal a release group only under an event that never holds a tag's ref
  (a schedule, a pull request, a branch push, `workflow_run` or `repository_dispatch`), since the
  rule does not read which events reach a group;
- a reacher's group with no text of its own, such as `${{ github.ref }}` alone;
- a `cancel-in-progress` expression that evaluates to false, since the rule evaluates none;
- a form the reader does not read (part 1), and a tab between a key's colon and its value;
- a reacher's call GitHub would refuse, to a missing file or in a cycle;
- a callee that one release job calls once, with the default queue (part 4);
- two release workflows' blocks that can share a group, though both queue;
- a step's text holding a line that starts `concurrency:`, which section 8's count of blocks reads.

Disclosed remainder. The rule does not read:
- the contents of a job's `steps`, `strategy`, `container` and `services`, or the mappings of
  `runs-on`, `environment` and `snapshot`; expression grammar, function names, or the contexts a bare
  `if:` reads; or a workflow file the directory scan does not select, one with an upper-case
  extension or one named `.yml` or `.yaml` alone. Follow-up #464 reads them;
- a tag whose push starts no run, which follow-up #475 tracks;
- a model of the tag queue's interleavings, owed once `covers` accepts a workflow file (#467).

The docs are silent on a group read through `GITHUB.REF`, a group with a leading space and a group
that renders empty; the rule refuses all three.

## 13. Acceptance criteria of the 2026-09-30 round 5 amendment

| id | criterion | decided by |
|---|---|---|
| A10 | the class is read as GitHub parses it (R12), by a population generated from constants and never listed, each member a set of workflow files beside a release workflow: each `cancel-in-progress` and `queue` a YAML 1.2 reader types, quoted, in a block or as a YAML 1.1 word; a tab in each line's indentation, and a block scalar's line indented less than its text or a blank line above it indented more, against a tab inside a value; a tab in the indentation of a comment line and of a blank line after each line; each line-break character and a byte-order mark where each line ends and inside it, refused by its name, against a line ended by a line feed or a carriage return and a line feed; a workflow file read by the tests as its bytes, as GitHub reads it, so a carriage return or a byte-order mark in it reaches the reader; each character YAML reserves as a plain scalar's first (`@`, the backtick, `%`, `,`, `]` and `}`, and `-`, `?` or `:` before a space on a mapping's value) at every scalar line of a release workflow, a second tag workflow and a called workflow, and a tag filter written as a flow list with an empty entry; each unclosed expression; every event of the parser's schema with each wrong kind of value, and each root and job key with one; callees one to three calls deep, local, remote, missing and in a cycle, and a callee holding a key the parser does not define; a release group that splits one tag's runs, and a job's own block; and every block that can render as a release group, in each context and event. A member GitHub refuses, or whose tag run can be cancelled or replaced, is refused, and a member GitHub runs with its tag's runs kept is not. The membership census is derived from R12's rule, never pinned by name | `test_workflow_concurrency.py` `the_release_class_is_read_as_github_parses_it` |
| A11 | the reader reads only R12 part 1's named forms, by a population generated from YAML 1.2.2's constants: every printable ASCII character first in a plain value, a sequence item and a flow item, each against the same text quoted; a character outside printable ASCII first; every block scalar header YAML defines, with and without text; a literal block's trailing blank lines and a blank line of it holding a tab; a flow list with an empty entry at every place, alone and before one trailing comma; each key the core schema types, against it quoted; and a comment after each one-line form, after a space or a tab, a comment line, a `#` inside a value and a blank line; each line-break character and a byte-order mark at every place a line holds text, refused by its name and its line, against lines ended by a line feed or a carriage return and a line feed; and a tab in the indentation of a comment line and of a blank line at every place, against a tab in a block's text or a comment's text; and a tab after a sequence item's `-`, before a key or a colon, at the root, nested and under a mapping. A named form reads as YAML reads it, and every other form is refused by its line with the message that names it | `test_ci_workflows.py` `the_reader_reads_only_its_named_forms` |
| A12 | the tests read a workflow file only through the loader (R12 part 1), by a census computed when the tests run over the modules that import the loader's module at any depth and `_support.py`, never a list of modules: every read site, every call whose callee cannot be named, every call of a function that reads what it is given, and every dynamic import in the test directory, is the loader's one definition, bound by its module and qualified name, or listed by its module, qualified name and text with its count and its reason, so a read swapped with its count kept is refused; and by the census's killer, planted copies of the test directory, each red naming its module: a read by each spelling the census names, in a module that imports the reader, in a new module through a re-export and at each place a site can sit; each path into the population (an alias, a star, a relative, dotted, package or two-level re-export import, an import inside a function or under `try`, a module outside `test_*`, `_support.py`, a helper module the population imports, a reader it defines); each dynamic import; each binding of the loader's name; and each listed site moved, swapped or changed; beside three controls that stay green | `test_ci_workflows.py` `the_census_is_red_on_every_planted_site` |

```acceptance
A10: python3 -m unittest discover -s scripts/tests -p test_workflow_concurrency.py -k the_release_class_is_read_as_github_parses_it
A11: python3 -m unittest discover -s scripts/tests -p test_ci_workflows.py -k the_reader_reads_only_its_named_forms
A12: python3 -m unittest discover -s scripts/tests -p test_ci_workflows.py -k the_census_is_red_on_every_planted_site
```

### File manifest of the round 5 amendment

| file | context | change |
|---|---|---|
| `scripts/tests/test_workflow_concurrency.py` | repo | changed: R12 and A10; A7 applies R12 to every workflow file; the membership census is derived from R12 |
| `scripts/tests/test_ci_workflows.py` | repo | changed: the reader keeps a scalar's quoting and types it by YAML 1.2's core schema, reads only R12 part 1's named forms and refuses every other form by its line (A11); a workflow file is read only through the loader, proved by the census and its killer (A12) |
| `scripts/tests/test_mutation_workflows.py` | repo | changed: each workflow file is read through the loader (A10, A12) |
| `scripts/tests/test_release_workflow.py` | repo | changed: `release.yml` is read through the loader (A10, A12) |
| `scripts/tests/test_rust_cache_workflow.py` | repo | changed: each workflow file is read through the loader (A10, A12) |
| `scripts/tests/test_mutation_python_verdict.py` | repo | changed: `re` is imported by its name, never through `__import__` (A12) |
| `scripts/mutation-rows.d/S19000-S19099.json` | repo | changed: S19015 to S19038, one row per arm of R12; S19041 to S19058, the census's killer on the real test directory (A12) |
| `docs/decisions/ADR-292-a-release-tags-runs-never-replace-a-waiting-run.md` | repo | changed: Decision Outcome, Consequences and considered options hold for R12 |
| `docs/red-first/SPEC-190.md` | repo | changed: A10, A11 and A12 |
| `changelog.d/ci-release-queue-377.md` | repo | changed: the rule of R12 |

# SPEC-674: the repository's stale branches are counted and held to a recorded count

Path:
`docs/specs/SPEC-674-the-repositorys-stale-branches-are-counted-and-held-to-a-recorded-count.md`.

- **Issue:** #1394 (referenced: it stays open for the promotion to `fail`, a delivery of its own under ADR-450 D6).
- **Context(s):** one census module and its planted lists under `scripts/tests/`, one workflow
  that writes the lists it reads, the rows band and the documents. No product source, existing
  test, existing workflow, `scripts/check.sh` or test-runner configuration changes.
- **Decided by:** ADR-697 (a stale branch is counted from two read-only lists a workflow step
  writes, and the count starts at warn). It follows ADR-450's promotion rule and amends nothing.
- **Schematic:** none: the change adds no state and one data flow, the workflow's read step
  feeding the census module one text file, which R4 states whole.
- **Status:** delivered by the pull request that adds this file, with its tests and
  `docs/red-first/SPEC-674.md`.

## 1. The problem, measured

The repository's branches are not counted by any check, and no SPEC, ADR or contributor guide
states a rule for a branch no pull request names. At the design's reading the forge listed 64
branches, read by `gh api 'repos/RexRenatus/deck-streak/branches?per_page=100' --paginate`, one
`gh api graphql` query of each branch's last commit time, and `gh api
'repos/RexRenatus/deck-streak/pulls?state=open&per_page=100' --paginate`:

| kind | branches | an open pull request names it | stale at 30 days | stale at 90 days |
|---|---|---|---|---|
| the default branch | 1 | 0 | 0 | 0 |
| the release branch | 1 | 0 | 1, not counted (R3) | 0 |
| a dependency bot's branch | 1 | 0 | 0 | 0 |
| an open pull request's head | 43 | 43 | 0 | 0 |
| any other branch | 18 | 0 | 8 | 0 |

The default and the release branch are the two the list marks protected; no merge queue branch
was listed. Each count is a lower bound of the list at its reading: the census reads the whole
list in its own workflow, and its first run records the count (R9).

## 2. Requirements

- R1. `scripts/tests/test_stale_branches.py` counts the repository's stale branches over the
  lists of R4 and compares the count with `RECORD`, a count and its reason.
- R2. A branch is stale when no open pull request whose head repository is this repository names
  it as its head and its last commit is older than `WINDOW_DAYS`, 90 days, before the lists' read
  stamp; a commit exactly at that edge is inside the window.
- R3. A branch `KEPT` names (the default branch `dev` and the release branch `main`) and a branch
  the list marks protected are not counted, however old. Every other branch is counted when
  stale: a bot's branch, a merge queue branch and a branch a closed pull request left included.
- R4. The census reads one text, line by line, its fields split by tabs: one `read-at` stamp in
  seconds, one `repository`, a `protected` line and a `commit` line (its last commit time in
  seconds) for each branch, and a `pull` line (head branch, head repository) for each open pull
  request. It reads neither the forge nor the wall clock. A line of any other form, a second
  stamp or repository, and a branch one of the two branch reads names and the other does not are
  refused by count.
- R5. The census prints `examined N branch(es)` and refuses a list with no branch, which is VOID.
- R6. `LEVEL` starts at `warn`: a count past the record prints `WARN stale-branches: N stale
  branch(es), past the recorded R` and passes, and at `fail` it fails. A count equal to the
  record prints `stale branches N, recorded R` and passes.
- R7. At both levels a record above the count fails with `STALE-RECORD stale-branches: recorded
  R, a count no longer reached; the count is N, so the record moves down`.
- R8. The planted lists are the three texts `planted.lists.fixture`, `empty.lists.fixture` and
  `disagree.lists.fixture` under `scripts/tests/fixtures/stale-branches/`, read as text; no runner
  collects, compiles or imports one.
- R9. `.github/workflows/stale-branches.yml` writes the lists of R4 with a token that reads
  contents and pull requests only, on a daily schedule, on a dispatch and on a pull request that
  changes the census, its lists or the workflow, then runs the census module alone with
  `STALE_BRANCHES_LISTS` naming the file. Where that variable is unset the repository test is
  skipped by name, and the planted tests decide every other criterion. `RECORD` holds the count
  the census's first run over the whole list measured, with that job's id in its reason.
- R10. The census prints counts and never a branch's name.

## 3. Acceptance criteria of SPEC-674

| id | criterion | decided by |
|---|---|---|
| A1 | R2: a branch past the window that an open pull request of this repository names as its head is not stale | `test_stale_branches.PlantedListsAreJudged.test_a_branch_an_open_pull_request_names_is_not_stale` |
| A2 | R2: a branch whose last commit is inside the window, or exactly at its edge, is not stale | `test_stale_branches.PlantedListsAreJudged.test_a_branch_with_a_commit_inside_the_window_is_not_stale` |
| A3 | R2, R4: over the planted list the stale branches are exactly the two past the window that no open pull request of this repository names, one of them named only by a fork's pull request | `test_stale_branches.PlantedListsAreJudged.test_a_branch_past_the_window_with_no_open_pull_request_is_stale` |
| A4 | R3: a branch KEPT names and a branch the list marks protected are not counted, however old | `test_stale_branches.PlantedListsAreJudged.test_the_kept_and_protected_branches_are_not_counted` |
| A5 | R6: a count equal to the record passes at both levels with its exact line | `test_stale_branches.PlantedListsAreJudged.test_a_count_at_the_record_passes_with_its_line` |
| A6 | R6: a count past the record prints its exact WARN line and passes at warn, and fails at fail | `test_stale_branches.PlantedListsAreJudged.test_a_count_past_the_record_is_a_site_at_warn_and_fails_at_fail` |
| A7 | R7: a record above the count fails with its exact line at warn as at fail | `test_stale_branches.PlantedListsAreJudged.test_a_record_above_the_count_fails_at_warn_as_at_fail` |
| A8 | R5: a list with no branch is refused naming `examined 0 branch(es)` | `test_stale_branches.PlantedListsAreJudged.test_an_empty_list_is_void` |
| A9 | R4: lists whose two branch reads disagree are refused by count | `test_stale_branches.PlantedListsAreJudged.test_lists_whose_branch_reads_disagree_are_refused` |
| A10 | R6, R9: the level is warn, the window 90 days, KEPT the default and release branches, and the record carries its reason | `test_stale_branches.PlantedListsAreJudged.test_the_census_starts_at_warn_over_ninety_days` |
| A11 | R8: the planted lists are exactly the three `.fixture` texts | `test_stale_branches.PlantedListsAreJudged.test_every_planted_list_is_a_fixture_text` |
| A12 | R1, R9: over the lists the workflow writes, the census examines every branch and holds RECORD | `test_stale_branches.TheRepositoryHoldsItsRecord.test_the_repository_holds_its_recorded_stale_count` |

```acceptance
A1: python3 -m unittest discover -s scripts/tests -p test_stale_branches.py -k test_a_branch_an_open_pull_request_names_is_not_stale
A2: python3 -m unittest discover -s scripts/tests -p test_stale_branches.py -k test_a_branch_with_a_commit_inside_the_window_is_not_stale
A3: python3 -m unittest discover -s scripts/tests -p test_stale_branches.py -k test_a_branch_past_the_window_with_no_open_pull_request_is_stale
A4: python3 -m unittest discover -s scripts/tests -p test_stale_branches.py -k test_the_kept_and_protected_branches_are_not_counted
A5: python3 -m unittest discover -s scripts/tests -p test_stale_branches.py -k test_a_count_at_the_record_passes_with_its_line
A6: python3 -m unittest discover -s scripts/tests -p test_stale_branches.py -k test_a_count_past_the_record_is_a_site_at_warn_and_fails_at_fail
A7: python3 -m unittest discover -s scripts/tests -p test_stale_branches.py -k test_a_record_above_the_count_fails_at_warn_as_at_fail
A8: python3 -m unittest discover -s scripts/tests -p test_stale_branches.py -k test_an_empty_list_is_void
A9: python3 -m unittest discover -s scripts/tests -p test_stale_branches.py -k test_lists_whose_branch_reads_disagree_are_refused
A10: python3 -m unittest discover -s scripts/tests -p test_stale_branches.py -k test_the_census_starts_at_warn_over_ninety_days
A11: python3 -m unittest discover -s scripts/tests -p test_stale_branches.py -k test_every_planted_list_is_a_fixture_text
A12: python3 -m unittest discover -s scripts/tests -p test_stale_branches.py -k test_the_repository_holds_its_recorded_stale_count
```

Every planted list is text the fixture spells literally, and every expected name, line and count
is a literal the test writes, never a value the census computes. Each test asserts its positive
artifact before an absence.

## 4. File manifest

| path | part | change |
|---|---|---|
| `scripts/tests/test_stale_branches.py` | the census | added: R1 to R8, R10 |
| `scripts/tests/fixtures/stale-branches/planted.lists.fixture` | a planted list | added: R2 to R4 |
| `scripts/tests/fixtures/stale-branches/empty.lists.fixture` | a planted list | added: R5 |
| `scripts/tests/fixtures/stale-branches/disagree.lists.fixture` | a planted list | added: R4 |
| `.github/workflows/stale-branches.yml` | the workflow that writes the lists | added: R9 |
| `scripts/mutation-rows.d/S67400-S67499.json` | the rows | added (section 9) |
| `docs/specs/SPEC-674-the-repositorys-stale-branches-are-counted-and-held-to-a-recorded-count.md` | the spec | added |
| `docs/decisions/ADR-697-a-stale-branch-is-counted-from-two-read-only-lists-a-workflow-step-writes-and-the-count-starts-at-warn.md` | the decision | added |
| `docs/red-first/SPEC-674.md` | the record | added |
| `changelog.d/stale-branches.md` | the fragment | added |

## 5. What this does NOT cover

- It deletes, renames or protects no branch, and changes no forge setting; a cleanup of today's
  stale branches, or a deletion rule, is decided apart, #1394.
- It does not count local worktrees, which the repository cannot see, #913.
- It does not tell a branch a tag or a release still needs from any other branch: such a branch
  is counted when stale, #1394.
- It does not promote `LEVEL` to `fail`; that is a delivery of its own under ADR-450's promotion
  rule, #1394.

## 6. Risks

- **The run's own time moves the count.** The count grows with no change in any pull request as
  branches age; the census runs on its own schedule and on pull requests that change it, never in
  `ci`, so no other pull request reads it, and at warn a growth only prints its line.
- **A tree-scanning guard reads the new files.** `scripts/tests/test_ci_workflows.py` and
  `scripts/tests/test_workflow_concurrency.py` read every workflow file, and
  `scripts/tests/test_temp_hygiene.py` reads every test module; each is read in CI at push 1.
- **The forge's lists change shape.** A field the step reads that is missing fails the step or
  the reader (R4), never a silent count.
- **A schedule stops.** The forge disables a public repository's schedule after 60 days with no
  repository activity; a dispatch runs it again.

## 7. What only CI proves

- The repository test reads the whole list only in the stale-branches workflow's `census` job,
  at push 1, whose job id `RECORD` then names.
- `scripts/tests/test_ci_workflows.py` and `scripts/tests/test_workflow_concurrency.py` over the
  new workflow, and the workflow lint, are read in CI by name.

## 8. Formal model

No state, actor or write path is added; the census is a pure reader of one text, closed by the
planted lists of section 3 and by section 9's rows. No model checker runs.

## 9. Mutation rows (S674)

Rows in `scripts/mutation-rows.d/S67400-S67499.json`, each on `scripts/tests/test_stale_branches.py` and
killed by a test of that module by its full id. Each find occurs exactly once in the file.

| row | what the mutant removes or breaks | killer |
|---|---|---|
| S67400 | R2: a branch an open pull request of this repository names is not stale | `test_a_branch_an_open_pull_request_names_is_not_stale` |
| S67401 | R2: a commit exactly at the window's edge is inside it | `test_a_branch_with_a_commit_inside_the_window_is_not_stale` |
| S67402 | R2: the window ends at the read stamp | `test_a_branch_with_a_commit_inside_the_window_is_not_stale` |
| S67403 | R2: only a pull request whose head repository is this one keeps a branch | `test_a_branch_past_the_window_with_no_open_pull_request_is_stale` |
| S67404 | R3: a branch KEPT names is not counted | `test_the_kept_and_protected_branches_are_not_counted` |
| S67405 | R3: a protected branch is not counted | `test_the_kept_and_protected_branches_are_not_counted` |
| S67406 | R3: KEPT names the release branch | `test_the_kept_and_protected_branches_are_not_counted` |
| S67407 | R4: a protected line reads true as protected | `test_the_kept_and_protected_branches_are_not_counted` |
| S67408 | R4: the read stamp is the list's own | `test_a_branch_past_the_window_with_no_open_pull_request_is_stale` |
| S67409 | R4: two branch reads that disagree are refused | `test_lists_whose_branch_reads_disagree_are_refused` |
| S67410 | R5: a list with no branch is refused | `test_an_empty_list_is_void` |
| S67411 | R6: a count past the record passes at warn | `test_a_count_past_the_record_is_a_site_at_warn_and_fails_at_fail` |
| S67412 | R6: a count past the record fails at fail | `test_a_count_past_the_record_is_a_site_at_warn_and_fails_at_fail` |
| S67413 | R6: a count equal to the record is no site | `test_a_count_at_the_record_passes_with_its_line` |
| S67414 | R7: a record above the count fails at warn | `test_a_record_above_the_count_fails_at_warn_as_at_fail` |
| S67415 | R6: the level starts at warn | `test_the_census_starts_at_warn_over_ninety_days` |
| S67416 | R2: the window is 90 days | `test_the_census_starts_at_warn_over_ninety_days` |

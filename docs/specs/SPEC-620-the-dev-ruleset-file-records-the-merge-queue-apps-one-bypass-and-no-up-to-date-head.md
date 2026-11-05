# SPEC-620: the dev ruleset file records the merge queue app's one bypass and no up-to-date head

- **Issue:** none. **Context(s):** the repository's branch rulesets (`.github/rulesets/`) and the merge
  queue file (`.mergify.yml`).
- **Decided by:** ADR-642 (the dev ruleset exempts the merge queue app and no longer requires an
  up-to-date head).
- **Schematic:** none; the change edits data files and tests, and no shape changes.
- **Status:** accepted
- **Mutation band:** none; the change writes no mutation rows.

## 1. The problem, measured

The dev ruleset that GitHub enforces differs from the file that records it, and the setup script
puts the file, not the live ruleset, in force. Measured at the cut of this change:

- The file lists no bypass actor and requires an up-to-date head:
  `grep -n -E 'bypass_actors|strict_required_status_checks_policy' .github/rulesets/dev.json` prints
  `"bypass_actors": []` (line 13) and `"strict_required_status_checks_policy": true` (line 39).
- The live dev ruleset, read once with the forge's read-only API, lists one bypass actor, the merge
  queue app (actor 10562, type `Integration`, mode `exempt`), so the queue can merge a tested batch;
  it does not require an up-to-date head; and its pull request rule carries two fields the file
  lacks, `require_extra_approval_for_unattributed_changes: true` and `required_reviewers: []`.
  Its name, target, enforcement, conditions and the other rule values equal the file's.
- `scripts/github-setup.sh` (lines 85 to 99) sends each ruleset file whole, so running it would
  replace the live ruleset with the file and take the queue's bypass away.
- `scripts/tests/test_rulesets.py` pins the old values: one test asserts that every ruleset file has
  `bypass_actors == []`, and another asserts that dev's strict policy is `True`.
- `RELEASING.md` says `dev`'s ruleset does require an up-to-date head, which the live ruleset does
  not.
- The merge queue file does not say which branch Mergify reads it from.

## 2. Requirements

R1. `.github/rulesets/dev.json` holds exactly one bypass actor: the merge queue app, actor 10562, type
`Integration`, mode `exempt`.

R2. The files of `main` and of the release tags hold no bypass actor.

R3. `.github/rulesets/dev.json` does not require an up-to-date head: its
`strict_required_status_checks_policy` is `false`.

R4. `.github/rulesets/dev.json` equals the live dev ruleset in its name, target, enforcement,
conditions, bypass actors and rules, as read at the cut, so applying the rulesets keeps the live
ruleset as it is.

R5. `.mergify.yml` states in one comment line that Mergify reads it from `dev`, the default branch,
and its queue rules are unchanged: the added line is a comment and no other line differs.

## 3. Acceptance criteria of SPEC-620

Each criterion is one test of `scripts/tests/test_rulesets.py`, named in the block below.

| criterion | what it decides | test |
|---|---|---|
| A1 | dev's bypass actors are exactly the merge queue app, exempt (R1, R4) | `test_rulesets.py` |
| A2 | no other ruleset file has a bypass actor, and every ruleset file keeps to enforceable rule types (R2) | `test_rulesets.py` |
| A3 | dev's strict policy is `false` (R3, R4) | `test_rulesets.py` |

```acceptance
A1: python3 -m unittest discover -s scripts/tests -p test_rulesets.py -k test_the_dev_ruleset_exempts_only_the_merge_queue_app
A2: python3 -m unittest discover -s scripts/tests -p test_rulesets.py -k test_no_other_ruleset_has_a_bypass_actor_and_none_has_a_refused_rule
A3: python3 -m unittest discover -s scripts/tests -p test_rulesets.py -k test_dev_does_not_require_an_up_to_date_head
```

## 4. File manifest

| path | part | change |
|---|---|---|
| `docs/specs/SPEC-620-the-dev-ruleset-file-records-the-merge-queue-apps-one-bypass-and-no-up-to-date-head.md` | C1 | new |
| `docs/decisions/ADR-642-the-dev-ruleset-exempts-the-merge-queue-app-and-no-longer-requires-an-up-to-date-head.md` | C1 | new |
| `scripts/tests/test_rulesets.py` | C2 | replaces two tests with three and adds one constant |
| `docs/red-first/SPEC-620.md` | C2 | new, appended at C3 and C4 |
| `.github/rulesets/dev.json` | C3 | the bypass actor, the strict policy and two pull request fields |
| `.mergify.yml` | C3 | one comment line added as line 2 |
| `RELEASING.md` | C3 | one sentence in lines 33 and 34 |
| `changelog.d/ci-dev-ruleset-records-the-merge-queue.md` | C3 | new |

## 5. What this does NOT cover

- Applying a ruleset: this change edits the file and runs no setup script; applying a ruleset stays
  the owner's act.
- The ruleset of `main`, which keeps no bypass actor and no up-to-date head.
- The merge queue's rules: the queue file gains one comment line, and its conditions, batch size and
  merge method are as they were.

## 6. Risks

- **A later live edit drifts from the file.** A ruleset changed in the forge's settings is not seen
  by the tests; the file is read again against the live ruleset whenever the rulesets are applied.
- **A second bypass actor.** A1 compares the actor list whole, so an added actor turns it red.
- **Dev without an up-to-date rule.** A pull request can merge on checks that ran on an older tip;
  the queue tests each batch on the tip of dev before it merges, and A3 holds the rule off on
  purpose.
